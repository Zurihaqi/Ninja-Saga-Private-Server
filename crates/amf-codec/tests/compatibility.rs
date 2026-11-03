use amf_codec::{Body, Envelope, Limits, Value, decode, encode};

fn envelope(value: Value) -> Envelope {
    Envelope {
        version: 0,
        headers: vec![],
        bodies: vec![Body {
            target: "test".into(),
            response: "/1".into(),
            data: value,
        }],
    }
}

#[test]
fn sabre_amf0_multiple_bodies() {
    let bytes = include_bytes!("fixtures/amf0-multiple.amf");
    let message = decode(bytes, Limits::default()).unwrap();
    assert_eq!(message.bodies.len(), 2);
    assert_eq!(message.bodies[0].target, "SystemLogin.checkVersion");
    assert_eq!(
        message.bodies[0].data,
        Value::Array(vec![
            Value::String("1.0.0".into()),
            Value::Bool(true),
            // SabreAMF writeArray uses isset(), so null array slots become undefined.
            Value::Undefined,
            Value::Number(42.5)
        ])
    );
    assert_eq!(
        message.bodies[1].data,
        Value::MixedArray(vec![("key".into(), Value::String("value".into()))])
    );
    assert_eq!(encode(&message, Limits::default()).unwrap(), bytes);
}

#[test]
fn sabre_amf3_request_integer_boundaries() {
    let bytes = include_bytes!("fixtures/amf3-request.amf");
    let message = decode(bytes, Limits::default()).unwrap();
    assert_eq!(message.version, 3);
    assert_eq!(
        message.bodies[0].data,
        Value::Amf3(Box::new(Value::Array(vec![
            Value::String("1.0.0".into()),
            Value::Integer(-268435456),
            Value::Integer(268435455),
            Value::Bool(true),
            Value::Null
        ])))
    );
    assert_eq!(encode(&message, Limits::default()).unwrap(), bytes);
}

#[test]
fn sabre_amf3_response_in_amf0_envelope() {
    let bytes = include_bytes!("fixtures/amf3-response.amf");
    let message = decode(bytes, Limits::default()).unwrap();
    assert_eq!(message.version, 0);
    assert_eq!(message.bodies[0].target, "/1/onResult");
    assert_eq!(
        message.bodies[0].data,
        Value::Amf3(Box::new(Value::Object(vec![
            ("status".into(), Value::Integer(500)),
            (
                "error".into(),
                Value::String("Unknown.method failed".into())
            )
        ])))
    );
    assert_eq!(encode(&message, Limits::default()).unwrap(), bytes);
}

#[test]
fn all_truncated_prefixes_fail() {
    let bytes = include_bytes!("fixtures/amf0-multiple.amf");
    for length in 0..bytes.len() {
        assert!(
            decode(&bytes[..length], Limits::default()).is_err(),
            "prefix {length}"
        );
    }
}

#[test]
fn php_byte_array_fixture_is_bounded_and_round_trips() {
    let bytes = include_bytes!("fixtures/amf3-bytes.amf");
    let message = decode(bytes, Limits::default()).unwrap();
    assert_eq!(
        message.bodies[0].data,
        Value::Amf3(Box::new(Value::Array(vec![Value::ByteArray(
            b"\x00\x01\xfftest".to_vec()
        )])))
    );
    assert_eq!(encode(&message, Limits::default()).unwrap(), bytes);
    for length in 0..bytes.len() {
        assert!(decode(&bytes[..length], Limits::default()).is_err());
    }
    let huge = Envelope {
        version: 0,
        headers: vec![],
        bodies: vec![Body {
            target: "test".into(),
            response: "/1".into(),
            data: Value::ByteArray(vec![0; 1024]),
        }],
    };
    assert!(
        encode(
            &huge,
            Limits {
                max_bytes: 100,
                ..Limits::default()
            }
        )
        .is_err()
    );
}

#[test]
fn bounded_input_and_output() {
    let message = envelope(Value::Array(vec![Value::Array(vec![Value::Null])]));
    let bytes = encode(&message, Limits::default()).unwrap();
    for limits in [
        Limits {
            max_bytes: 4,
            ..Limits::default()
        },
        Limits {
            max_depth: 1,
            ..Limits::default()
        },
        Limits {
            max_values: 2,
            ..Limits::default()
        },
        Limits {
            max_string_bytes: 1,
            ..Limits::default()
        },
    ] {
        assert!(decode(&bytes, limits).is_err());
        assert!(encode(&message, limits).is_err());
    }
}

#[test]
fn rejects_declared_length_mismatch_trailing_and_huge_array() {
    let mut bytes = encode(&envelope(Value::Null), Limits::default()).unwrap();
    let size_offset = bytes.len() - 5;
    bytes[size_offset..size_offset + 4].copy_from_slice(&2u32.to_be_bytes());
    assert!(decode(&bytes, Limits::default()).is_err());
    bytes[size_offset..size_offset + 4].copy_from_slice(&1u32.to_be_bytes());
    assert!(decode(&bytes, Limits::default()).is_ok());
    bytes.push(0);
    assert!(decode(&bytes, Limits::default()).is_err());
    let mut bytes = encode(&envelope(Value::Array(vec![])), Limits::default()).unwrap();
    let len = bytes.len();
    bytes[len - 4..].copy_from_slice(&u32::MAX.to_be_bytes());
    assert!(decode(&bytes, Limits::default()).is_err());
}

#[test]
fn string_references_are_bounded_and_checked() {
    let mut bytes = encode(
        &envelope(Value::Amf3(Box::new(Value::Array(vec![
            Value::String("repeat".into()),
            Value::String("repeat".into()),
        ])))),
        Limits::default(),
    )
    .unwrap();
    bytes.truncate(bytes.len() - 8);
    bytes.extend_from_slice(&[6, 0]); // second string is reference zero
    let message = decode(&bytes, Limits::default()).unwrap();
    assert_eq!(
        message.bodies[0].data,
        Value::Amf3(Box::new(Value::Array(vec![
            Value::String("repeat".into()),
            Value::String("repeat".into())
        ])))
    );
    *bytes.last_mut().unwrap() = 2;
    assert!(decode(&bytes, Limits::default()).is_err());
}

#[test]
fn invalid_reference_fails_explicitly() {
    let mut bytes = encode(&envelope(Value::Null), Limits::default()).unwrap();
    bytes.pop();
    bytes.extend_from_slice(&[17, 10, 0]);
    assert_eq!(
        decode(&bytes, Limits::default()).unwrap_err().0,
        "invalid object reference"
    );
}

#[test]
fn pinned_php_verified_object_array_and_trait_references() {
    let bytes = include_bytes!("fixtures/amf3-references.amf");
    let decoded = decode(bytes, Limits::default()).unwrap();
    let object = |n| Value::Object(vec![("x".into(), Value::Integer(n))]);
    assert_eq!(
        decoded.bodies[0].data,
        Value::Amf3(Box::new(Value::Array(vec![
            object(7),
            object(9),
            object(7),
            Value::Array(vec![Value::Integer(11)]),
            Value::Array(vec![Value::Integer(11)])
        ])))
    );
    // Encoding expands references; semantics, not byte identity, are the contract here.
    assert_eq!(
        decode(
            &encode(&decoded, Limits::default()).unwrap(),
            Limits::default()
        )
        .unwrap(),
        decoded
    );
    let decoded = decode(
        include_bytes!("fixtures/amf0-references.amf"),
        Limits::default(),
    )
    .unwrap();
    let object = Value::Object(vec![("x".into(), Value::Number(7.0))]);
    assert_eq!(
        decoded.bodies[0].data,
        Value::Array(vec![
            object.clone(),
            object,
            Value::Array(vec![Value::Bool(true)]),
            Value::Array(vec![Value::Bool(true)])
        ])
    );
}

fn raw_body(payload: &[u8]) -> Vec<u8> {
    let mut bytes = encode(&envelope(Value::Null), Limits::default()).unwrap();
    bytes.pop();
    bytes.extend_from_slice(payload);
    bytes
}

#[test]
fn cycles_and_bad_trait_indices_are_rejected() {
    for payload in [
        vec![10, 0, 0, 0, 1, 7, 0, 0],
        vec![17, 9, 3, 1, 9, 0],
        vec![17, 10, 19, 1, 3, b'x', 10, 0],
    ] {
        assert_eq!(
            decode(&raw_body(&payload), Limits::default())
                .unwrap_err()
                .0,
            "cyclic object reference not supported"
        );
    }
    assert_eq!(
        decode(&raw_body(&[17, 10, 1]), Limits::default())
            .unwrap_err()
            .0,
        "invalid trait reference"
    );
    assert_eq!(
        decode(&raw_body(&[7, 0, 0]), Limits::default())
            .unwrap_err()
            .0,
        "invalid object reference"
    );
}

#[test]
fn object_reference_expansion_respects_depth_value_and_string_limits() {
    // An array containing a completed one-element array, then a reference to it.
    let bytes = raw_body(&[17, 9, 5, 1, 9, 3, 1, 1, 9, 2]);
    assert!(decode(&bytes, Limits::default()).is_ok());
    assert!(
        decode(
            &bytes,
            Limits {
                max_values: 9,
                ..Limits::default()
            }
        )
        .is_err()
    );
    // Reinsert a cached array one level deeper than its original occurrence.
    let bytes = raw_body(&[17, 9, 5, 1, 9, 3, 1, 1, 9, 3, 1, 9, 2]);
    assert!(
        decode(
            &bytes,
            Limits {
                max_depth: 3,
                ..Limits::default()
            }
        )
        .is_err()
    );
    let mut payload = vec![17, 9, 0x81, 0x4b, 1, 10, 19, 1, 3, b'x', 6, 0x81, 1];
    payload.extend_from_slice(&[b'z'; 64]);
    for _ in 0..100 {
        payload.extend_from_slice(&[10, 2]);
    }
    let bytes = raw_body(&payload);
    assert_eq!(
        decode(
            &bytes,
            Limits {
                max_bytes: 1024,
                ..Limits::default()
            }
        )
        .unwrap_err()
        .0,
        "decoded string budget exceeded"
    );
}

#[test]
fn repeated_string_references_cannot_amplify_without_limit() {
    let mut bytes = encode(&envelope(Value::Null), Limits::default()).unwrap();
    bytes.pop();
    // Wrapped AMF3 array of 101 strings. One inline 64-byte string, then 100 references.
    bytes.extend_from_slice(&[17, 9, 0x81, 0x4b, 1, 6, 0x81, 1]);
    bytes.extend_from_slice(&[b'x'; 64]);
    for _ in 0..100 {
        bytes.extend_from_slice(&[6, 0]);
    }
    assert!(bytes.len() < 1024);
    let limits = Limits {
        max_bytes: 1024,
        ..Limits::default()
    };
    assert_eq!(
        decode(&bytes, limits).unwrap_err().0,
        "decoded string budget exceeded"
    );
}

#[test]
fn malformed_byte_sequences_do_not_panic() {
    // Deterministic mutation corpus, not a replacement for a coverage-guided fuzzer.
    let original = include_bytes!("fixtures/amf3-response.amf");
    for offset in 0..original.len() {
        for byte in [0, 1, 3, 7, 10, 17, 127, 128, 255] {
            let mut input = original.to_vec();
            input[offset] = byte;
            let _ = decode(&input, Limits::default());
        }
    }
}
