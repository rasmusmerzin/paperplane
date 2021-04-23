use async_std::sync::Arc;
use async_std::task;
use async_tungstenite::async_std::{connect_async, ConnectStream};
use async_tungstenite::tungstenite::protocol::frame::coding::CloseCode;
use async_tungstenite::tungstenite::protocol::CloseFrame;
use async_tungstenite::WebSocketStream;
use futures::sink::SinkExt;
use futures::stream::StreamExt;
use paperplane::tungstenite::{self, Message};
use paperplane::{Event, Server};

const LOCALHOST: &str = "127.0.0.1";

async fn base(
    port: usize,
    count: usize,
) -> tungstenite::Result<(Arc<Server>, Vec<WebSocketStream<ConnectStream>>)> {
    let addr = format!("{}:{}", LOCALHOST, port);
    let server = Server::new(10);
    server.listen(&addr).await?;

    let mut clients = vec![];
    for i in 0..count {
        clients.push(connect_async(format!("ws://{}", addr)).await?.0);
        assert_eq!(
            server.next::<Message>().await,
            Some(Event::Connected(i as u128))
        );
    }

    Ok((server, clients))
}

#[test]
fn receive() -> tungstenite::Result<()> {
    task::block_on(async {
        let (server, mut clients) = base(8001, 2).await?;

        let msg = Message::Text("first".into());
        clients[0].send(msg.clone()).await?;
        assert_eq!(server.next::<Message>().await, Some(Event::Message(0, msg)));

        let msg = Message::Text("second".into());
        clients[1].send(msg.clone()).await?;
        assert_eq!(server.next::<Message>().await, Some(Event::Message(1, msg)));

        server.close().await
    })
}

#[test]
fn send() -> tungstenite::Result<()> {
    task::block_on(async {
        let (server, mut clients) = base(8002, 3).await?;

        let msg0 = Message::Text("first back".into());
        server.send(Some(0), msg0.clone()).await?;
        assert_eq!(clients[0].next().await.unwrap()?, msg0);

        let msg1 = Message::Text("second back".into());
        server.send(Some(1), msg1.clone()).await?;
        assert_eq!(clients[1].next().await.unwrap()?, msg1);

        let msg = Message::Text("both back".into());
        server.send(None, msg.clone()).await?;
        assert_eq!(clients[0].next().await.unwrap()?, msg);
        assert_eq!(clients[1].next().await.unwrap()?, msg);
        assert_eq!(clients[2].next().await.unwrap()?, msg);

        server.close().await
    })
}

#[test]
fn disconnect() -> tungstenite::Result<()> {
    task::block_on(async {
        let (server, mut clients) = base(8003, 4).await?;

        server.kick(Some(0), "kick").await?;
        assert_eq!(
            clients[0].next().await.unwrap()?,
            Message::Close(Some(CloseFrame {
                code: CloseCode::Normal,
                reason: "kick".into(),
            }))
        );
        assert_eq!(
            server.next::<Message>().await,
            Some(Event::Kicked(0, "kick".into()))
        );

        clients[1].close(None).await?;
        assert_eq!(server.next::<Message>().await, Some(Event::Disconnected(1)));

        server.kick(None, "all".into()).await?;
        let close_frame = Message::Close(Some(CloseFrame {
            code: CloseCode::Normal,
            reason: "all".into(),
        }));
        assert_eq!(clients[2].next().await.unwrap()?, close_frame);
        assert_eq!(clients[3].next().await.unwrap()?, close_frame);

        server.close().await
    })
}

#[test]
fn close() -> tungstenite::Result<()> {
    task::block_on(async {
        let (server, clients) = base(8010, 4).await?;

        server.close().await?;

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
