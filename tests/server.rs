use async_std::sync::Arc;
use async_std::task;
use async_tungstenite::async_std::{connect_async, ConnectStream};
use async_tungstenite::tungstenite::protocol::frame::coding::CloseCode;
use async_tungstenite::tungstenite::protocol::CloseFrame;
use async_tungstenite::WebSocketStream;
use futures::sink::SinkExt;
use futures::stream::StreamExt;
use paperplane::{Event, Message, Server, WsResult};

const LOCALHOST: &str = "127.0.0.1";

async fn base(
    port: usize,
    count: usize,
) -> WsResult<(Arc<Server>, Vec<WebSocketStream<ConnectStream>>)> {
    let addr = format!("{}:{}", LOCALHOST, port);
    let server = Server::new();
    server.listen(&addr).await?;

    let mut clients = vec![];
    for i in 0..count {
        clients.push(connect_async(format!("ws://{}", addr)).await?.0);
        assert_eq!(server.next().await, Some(Event::Connected(i)));
    }

    Ok((server, clients))
}

#[test]
fn receive() -> WsResult<()> {
    task::block_on(async {
        let (server, mut clients) = base(8001, 2).await?;

        let msg = Message::Text("first".into());
        clients[0].send(msg.clone()).await?;
        assert_eq!(server.next().await, Some(Event::Message(0, msg)));

        let msg = Message::Text("second".into());
        clients[1].send(msg.clone()).await?;
        assert_eq!(server.next().await, Some(Event::Message(1, msg)));

        server.close().await
    })
}

#[test]
fn send() -> WsResult<()> {
    task::block_on(async {
        let (server, mut clients) = base(8002, 3).await?;

        let msg0 = Message::Text("first back".into());
        server.send(0, msg0.clone()).await?;
        assert_eq!(clients[0].next().await.unwrap()?, msg0);

        let msg1 = Message::Text("second back".into());
        server.send(1, msg1.clone()).await?;
        assert_eq!(clients[1].next().await.unwrap()?, msg1);

        server
            .send_map(|id| match id {
                0 => Some(msg0.clone()),
                1 => Some(msg1.clone()),
                _ => None,
            })
            .await?;
        assert_eq!(clients[0].next().await.unwrap()?, msg0);
        assert_eq!(clients[1].next().await.unwrap()?, msg1);

        let msg = Message::Text("both back".into());
        server.send_all(msg.clone()).await?;
        assert_eq!(clients[0].next().await.unwrap()?, msg);
        assert_eq!(clients[1].next().await.unwrap()?, msg);
        assert_eq!(clients[2].next().await.unwrap()?, msg);

        server.close().await
    })
}

#[test]
fn disconnect() -> WsResult<()> {
    task::block_on(async {
        let (server, mut clients) = base(8003, 4).await?;

        server
            .kick_map(|id| match id {
                1 => Some("half".into()),
                _ => None,
            })
            .await?;
        assert_eq!(
            clients[1].next().await.unwrap()?,
            Message::Close(Some(CloseFrame {
                code: CloseCode::Normal,
                reason: "half".into(),
            }))
        );
        assert_eq!(
            server.next().await.unwrap(),
            Event::Kicked(1, "half".into())
        );

        server.kick(2, "kick").await?;
        assert_eq!(
            clients[2].next().await.unwrap()?,
            Message::Close(Some(CloseFrame {
                code: CloseCode::Normal,
                reason: "kick".into(),
            }))
        );
        assert_eq!(server.next().await, Some(Event::Kicked(2, "kick".into())));

        clients[0].close(None).await?;
        assert_eq!(server.next().await, Some(Event::Disconnected(0)));

        server.kick_all("all".into()).await?;
        assert_eq!(
            clients[3].next().await.unwrap()?,
            Message::Close(Some(CloseFrame {
                code: CloseCode::Normal,
                reason: "all".into(),
            }))
        );
        assert_eq!(server.next().await.unwrap(), Event::Kicked(3, "all".into()));

        server.close().await
    })
}

#[test]
fn close() -> WsResult<()> {
    task::block_on(async {
        let (server, clients) = base(8010, 4).await?;

        assert_eq!(server.listener_count().await, 1);
        server.listen(format!("{}:{}", LOCALHOST, 8011)).await?;
        assert_eq!(server.listener_count().await, 2);
        server.listen(format!("{}:{}", LOCALHOST, 8012)).await?;
        assert_eq!(server.listener_count().await, 3);
        server.listen(format!("{}:{}", LOCALHOST, 8013)).await?;
        assert_eq!(server.listener_count().await, 4);

        server.close().await?;
        assert_eq!(server.listener_count().await, 0);

        for mut client in clients {
            assert_eq!(
                client.next().await.unwrap()?,
                Message::Close(Some(CloseFrame {
                    code: CloseCode::Normal,
                    reason: "server closed".into(),
                }))
            );
        }

        Ok(())
    })
}
