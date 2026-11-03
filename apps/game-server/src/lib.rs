//! Connectivity spike: only the upstream diagnostic Game.getVersion method is implemented.
use amf_codec::{Body, Envelope, Limits, Value};
use axum::{
    Router,
    body::Bytes,
    extract::DefaultBodyLimit,
    http::{StatusCode, header},
    response::{IntoResponse, Response},
    routing::any,
};

pub fn router() -> Router {
    Router::new()
        .route("/amf", any(gateway))
        .route("/gateway.php", any(gateway))
        .layer(DefaultBodyLimit::max(Limits::default().max_bytes))
}

async fn gateway(bytes: Bytes) -> Response {
    if bytes.is_empty() {
        return "AMF Gateway Ready".into_response();
    }
    let request = match amf_codec::decode(&bytes, Limits::default()) {
        Ok(request) => request,
        Err(error) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Invalid AMF Data: {error}"),
            )
                .into_response();
        }
    };
    let wrap = request.version == 3;
    let bodies = request
        .bodies
        .into_iter()
        .map(|body| {
            let data = dispatch(&body.target, wrap);
            Body {
                target: format!("{}/onResult", body.response),
                response: String::new(),
                data: if wrap {
                    Value::Amf3(Box::new(data))
                } else {
                    data
                },
            }
        })
        .collect();
    match amf_codec::encode(
        &Envelope {
            version: 0,
            headers: vec![],
            bodies,
        },
        Limits::default(),
    ) {
        Ok(output) => ([(header::CONTENT_TYPE, "application/x-amf")], output).into_response(),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "AMF response encoding failed",
        )
            .into_response(),
    }
}

fn dispatch(target: &str, amf3: bool) -> Value {
    // Source: app/Services/Amf/GameService/VersionService.php, pinned reference commit.
    let pairs = if matches!(
        target,
        "Game.getVersion" | "GameService.getVersion" | "game.getVersion" | "gameService.getVersion"
    ) {
        vec![
            ("version".into(), Value::String("1.0.0".into())),
            (
                "server".into(),
                Value::String("Laravel Ninja Saga Revived".into()),
            ),
            ("status".into(), Value::String("active".into())),
        ]
    } else {
        // Explicit spike limitation; not a fabricated gameplay implementation.
        vec![
            (
                "status".into(),
                if amf3 {
                    Value::Integer(500)
                } else {
                    Value::Number(500.0)
                },
            ),
            (
                "error".into(),
                Value::String(format!(
                    "{target} failed: method not implemented in Rust compatibility spike"
                )),
            ),
        ]
    };
    if amf3 {
        Value::Object(pairs)
    } else {
        Value::MixedArray(pairs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::{Body as HttpBody, to_bytes},
        http::Request,
    };
    use tower::ServiceExt;

    #[tokio::test]
    async fn gateway_matches_reference_php_bytes() {
        let cases: &[(&[u8], &[u8])] = &[
            (
                include_bytes!(
                    "../../../tests/compatibility/fixtures/game-version-amf0-request.amf"
                ),
                include_bytes!(
                    "../../../tests/compatibility/fixtures/game-version-amf0-response.amf"
                ),
            ),
            (
                include_bytes!(
                    "../../../tests/compatibility/fixtures/game-version-amf3-request.amf"
                ),
                include_bytes!(
                    "../../../tests/compatibility/fixtures/game-version-amf3-response.amf"
                ),
            ),
        ];
        for path in ["/amf", "/gateway.php"] {
            for (request, expected) in cases {
                let response = router()
                    .oneshot(
                        Request::post(path)
                            .body(HttpBody::from(request.to_vec()))
                            .unwrap(),
                    )
                    .await
                    .unwrap();
                assert_eq!(response.status(), StatusCode::OK);
                assert_eq!(
                    response.headers()[header::CONTENT_TYPE],
                    "application/x-amf"
                );
                assert_eq!(
                    to_bytes(response.into_body(), 4096).await.unwrap().as_ref(),
                    *expected
                );
            }
        }
    }

    #[tokio::test]
    async fn both_routes_preserve_amf3_wrapper_and_multiple_response_targets() {
        for path in ["/amf", "/gateway.php"] {
            for version in [0, 3] {
                let input = Envelope {
                    version,
                    headers: vec![],
                    bodies: vec![
                        Body {
                            target: "Game.getVersion".into(),
                            response: "/1".into(),
                            data: Value::Array(vec![]),
                        },
                        Body {
                            target: "Unknown.method".into(),
                            response: "/2".into(),
                            data: Value::Array(vec![]),
                        },
                    ],
                };
                let bytes = amf_codec::encode(&input, Limits::default()).unwrap();
                let response = router()
                    .oneshot(Request::post(path).body(HttpBody::from(bytes)).unwrap())
                    .await
                    .unwrap();
                assert_eq!(response.status(), StatusCode::OK);
                assert_eq!(
                    response.headers()[header::CONTENT_TYPE],
                    "application/x-amf"
                );
                let bytes = to_bytes(response.into_body(), 4096).await.unwrap();
                let output = amf_codec::decode(&bytes, Limits::default()).unwrap();
                assert_eq!(output.version, 0);
                assert_eq!(output.bodies.len(), 2);
                assert_eq!(output.bodies[0].target, "/1/onResult");
                assert_eq!(output.bodies[1].target, "/2/onResult");
                assert_eq!(
                    output.bodies[0].data,
                    if version == 3 {
                        Value::Amf3(Box::new(dispatch("Game.getVersion", true)))
                    } else {
                        dispatch("Game.getVersion", false)
                    }
                );
            }
        }
    }

    #[tokio::test]
    async fn malformed_and_oversized_requests_are_rejected() {
        for (data, status) in [
            (vec![0, 3, 0], StatusCode::INTERNAL_SERVER_ERROR),
            (vec![0; 1024 * 1024 + 1], StatusCode::PAYLOAD_TOO_LARGE),
        ] {
            let response = router()
                .oneshot(Request::post("/amf").body(HttpBody::from(data)).unwrap())
                .await
                .unwrap();
            assert_eq!(response.status(), status);
        }
    }
}
