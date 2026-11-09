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
    .expect("PvP WebSocket event deadline")
}
#[tokio::test]
async fn websocket_pvp_auth_and_disconnect() {
    let server = tests::test_server();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let app = router(server.clone());
    let task = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    let (mut client, _) = connect_async(format!(
        "ws://{address}/socket.io/?EIO=3&transport=websocket"
    ))
    .await
    .unwrap();
    assert!(receive(&mut client, "sid").await.starts_with('0'));
    receive(&mut client, "40").await;
    client.send(Message::Text("40/pvp,".into())).await.unwrap();
    receive(&mut client, "40/pvp").await;
    client
        .send(Message::Text(
            "42/pvp,[\"System.auth\",{\"id\":1,\"session_key\":\"sess_1\"}]".into(),
        ))
        .await
        .unwrap();
    assert!(
        receive(&mut client, "Client.characterInfo")
            .await
            .contains("Naruto")
    );
    client.send(Message::Text("2probe".into())).await.unwrap();
    assert_eq!(receive(&mut client, "3probe").await, "3probe");
    client.close(None).await.unwrap();
    tokio::time::sleep(Duration::from_millis(40)).await;
    assert!(server.inner.state.lock().unwrap().sessions.is_empty());
    task.abort();
}
