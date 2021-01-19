use async_std::sync::Arc;
use async_std::task;
use async_tungstenite::async_std::{connect_async, ConnectStream};
use async_tungstenite::WebSocketStream;
use futures::sink::SinkExt;
use futures::stream::StreamExt;
use paperplane::tungstenite::protocol::frame::coding::CloseCode;
use paperplane::tungstenite::protocol::CloseFrame;
use paperplane::tungstenite::{self, Message};
use paperplane::{Event, Server};

const LOCALHOST: &str = "127.0.0.1";

#[derive(Default, Clone)]
struct Session {
    txt: String,
}

async fn base(
    port: usize,
    count: usize,
) -> tungstenite::Result<(Arc<Server<Session>>, Vec<WebSocketStream<ConnectStream>>)> {
    let addr = format!("{}:{}", LOCALHOST, port);
    let server = Server::new(10);
    server.listen(&addr).await?;

    let mut clients = vec![];
    for i in 0..count {
        clients.push(connect_async(format!("ws://{}", addr)).await?.0);
        assert_eq!(server.next().await, Some(Event::Connected(i as u128)));
    }

    Ok((server, clients))
}

#[test]
fn receive() -> tungstenite::Result<()> {
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
fn send() -> tungstenite::Result<()> {
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
fn disconnect() -> tungstenite::Result<()> {
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
fn close() -> tungstenite::Result<()> {
    task::block_on(async {
        let (server, clients) = base(8004, 4).await?;

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

#[test]
fn state() -> tungstenite::Result<()> {
    task::block_on(async {
        let (server, _clients) = base(8005, 4).await?;
        for i in 0..4 {
            assert_eq!(server.get_session(i).await.unwrap().txt, "");
        }
        for i in 0..4 {
            server
                .set_session(i, Session { txt: i.to_string() })
                .await
                .unwrap();
        }
        for i in 0..4 {
            assert_eq!(server.get_session(i).await.unwrap().txt, i.to_string());
        }
        for i in 0..4 {
            assert_eq!(
                server
                    .find_session(|sess| sess.txt == i.to_string())
                    .await
                    .unwrap()
                    .0,
                i
            );
        }
        for i in 0..4 {
            server
                .replace_session(i, |sess| Session {
                    txt: format!("{}{0}", sess.txt),
                })
                .await
                .unwrap();
        }
        for i in 0..4 {
            assert_eq!(
                server.get_session(i).await.unwrap().txt,
                format!("{}{0}", i)
            );
        }
        Ok(())
    })
}
