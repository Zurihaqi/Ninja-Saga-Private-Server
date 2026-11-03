use socket_protocol::*;

#[test]
fn legacy_javascript_parser_fixture() {
    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/engine-v3.json")).unwrap();
    let encoded = fixture["encoded"].as_str().unwrap();
    let packets = decode_payload(encoded, Limits::default()).unwrap();
    assert_eq!(packets.len(), 4);
    assert_eq!(packets[0].kind, EngineKind::Ping);
    assert_eq!(packets[0].data, "probe");
    assert_eq!(
        packets[2].data,
        "2/compat,[\"echo\",\"忍者😀\",{\"value\":1}]"
    );
    assert_eq!(
        encode_payload(&packets, Limits::default()).unwrap(),
        encoded
    );
    assert!(
        matches!(decode_socket(&packets[1].data, Limits::default()).unwrap(), SocketPacket::Connect { namespace } if namespace == "/compat")
    );
    let event = decode_socket(&packets[2].data, Limits::default()).unwrap();
    assert_eq!(
        encode_socket(&event, Limits::default()).unwrap(),
        packets[2].data
    );
}

#[test]
fn malformed_lengths_and_unsupported_frames() {
    for value in [
        "",
        "x:4hi",
        "2:4",
        "999999999999999999999:4",
        "1:😀",
        "2:4😀",
        "1:7",
        "0:1:2",
    ] {
        assert!(decode_payload(value, Limits::default()).is_err(), "{value}");
    }
    // Four UTF-16 units: message marker + BMP character + surrogate pair.
    assert_eq!(
        decode_payload("4:4忍😀", Limits::default()).unwrap()[0].data,
        "忍😀"
    );
    for value in [
        "3/compat,1[]",
        "5/compat,[]",
        "2/compat,12[\"echo\"]",
        "2/compat,[]",
        "2/compat,{}",
        "2/compat,[1]",
        "0/compat,{}",
    ] {
        assert!(decode_socket(value, Limits::default()).is_err(), "{value}");
    }
}

#[test]
fn bounds_cover_packets_payloads_and_json_output() {
    let limits = Limits {
        max_payload_bytes: 8,
        max_packet_bytes: 4,
        max_packets: 1,
    };
    assert!(decode_payload("1:21:2", limits).is_err());
    assert!(decode_payload("5:4test", limits).is_err());
    assert!(decode_payload("1:21:21:2", limits).is_err());
    assert!(
        encode_payload(
            &[EnginePacket {
                kind: EngineKind::Message,
                data: "large".into()
            }],
            limits
        )
        .is_err()
    );
    assert!(
        encode_socket(
            &SocketPacket::Event {
                namespace: "/".into(),
                name: "echo".into(),
                args: vec![serde_json::json!("large")]
            },
            limits
        )
        .is_err()
    );
    assert_eq!(decode_payload("0:", limits).unwrap(), vec![]);
}

#[test]
fn malformed_mutations_never_panic() {
    for n in 0..128u8 {
        for value in [
            format!("1:{}", n as char),
            format!("2/compat,[{}]", n as char),
        ] {
            let _ = decode_payload(&value, Limits::default());
            let _ = decode_socket(&value, Limits::default());
        }
    }
}
