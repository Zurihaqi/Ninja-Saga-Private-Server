use super::*;
use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::{connect_async, tungstenite::Message};
type Client =
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;
async fn receive(client: &mut Client, needle: &str) -> String {
    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            let text = client
                .next()
                .await
                .unwrap()
                .unwrap()
                .into_text()
                .unwrap()
                .to_string();
            if text.contains(needle) {
                return text;
            }
        }
    })
    .await
    .expect("WebSocket event deadline")
}
async fn open(base: &str, id: u64, key: &str) -> Client {
    let (mut client, _) = connect_async(format!("{base}/socket.io/?EIO=3&transport=websocket"))
        .await
        .unwrap();
    let handshake = receive(&mut client, "sid").await;
    assert!(handshake.starts_with('0'));
    assert_eq!(receive(&mut client, "40").await, "40");
    client
        .send(Message::Text("40/global-chat,".into()))
        .await
        .unwrap();
    receive(&mut client, "40/global-chat").await;
    client
        .send(Message::Text(
            format!(
                "42/global-chat,[\"auth\",{{\"character_id\":{id},\"session_key\":\"{key}\"}}]"
            )
            .into(),
        ))
        .await
        .unwrap();
    client
}
#[tokio::test]
async fn websocket_chat_auth_unicode_ping_and_disconnect() {
    let server = tests::test_server();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("ws://{}", listener.local_addr().unwrap());
    let app = router(server.clone());
    let task = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    let mut first = open(&base, 101, "valid_session_1").await;
    let mut second = open(&base, 102, "valid_session_2").await;
    receive(&mut first, "history").await;
    receive(&mut second, "history").await;
    first
        .send(Message::Text(
            "42/global-chat,[\"sendMessage\",\"忍者😀\"]".into(),
        ))
        .await
        .unwrap();
    assert!(receive(&mut second, "忍者😀").await.contains("message"));
    first.send(Message::Text("2probe".into())).await.unwrap();
    assert_eq!(receive(&mut first, "3probe").await, "3probe");
    let mut rejected = open(&base, 101, "wrong_session").await;
    receive(&mut rejected, "error").await;
    first.close(None).await.unwrap();
    second.close(None).await.unwrap();
    rejected.close(None).await.unwrap();
    tokio::time::sleep(Duration::from_millis(40)).await;
    assert!(server.inner.state.lock().unwrap().sessions.is_empty());
    task.abort();
}
