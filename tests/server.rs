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
use std::convert::{TryFrom, TryInto};
use std::string::FromUtf8Error;

const LOCALHOST: &str = "127.0.0.1";

#[derive(Debug, PartialEq)]
struct Msg(String);

impl From<Message> for Msg {
    fn from(msg: Message) -> Self {
        Self(String::from_utf8_lossy(&msg.into_data()).into())
    }
}

#[derive(Debug, PartialEq)]
struct TrueMsg(String);

impl TryFrom<Message> for TrueMsg {
    type Error = FromUtf8Error;
    fn try_from(msg: Message) -> Result<Self, Self::Error> {
        Ok(Self(String::from_utf8(msg.into_data())?))
    }
}

#[derive(Debug, PartialEq)]
struct ErrMsg;

impl TryInto<Message> for ErrMsg {
    type Error = ();
    fn try_into(self) -> Result<Message, Self::Error> {
        Err(())
    }
}

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
fn receive_convert() -> tungstenite::Result<()> {
    task::block_on(async {
        let (server, mut clients) = base(8011, 2).await?;

        let msg = Message::Text("first".into());
        clients[0].send(msg.clone()).await?;
        assert_eq!(
            server.next().await,
            Some(Event::Message(0, Msg("first".into())))
        );

        let msg = Message::Text("second".into());
        clients[1].send(msg.clone()).await?;
        assert_eq!(
            server.next().await,
            Some(Event::Message(1, Msg("second".into())))
        );

        let msg = Message::Text("third".into());
        clients[0].send(msg.clone()).await?;
        assert_eq!(
            server.next_transform().await,
            Some(Ok(Event::Message(0, TrueMsg("third".into()))))
        );

        let msg = Message::Binary(vec![0, 159]);
        clients[1].send(msg.clone()).await?;
        assert!(server
            .next_transform::<TrueMsg>()
            .await
            .map(|res| res.is_err())
            .unwrap_or(false));

        server.close().await
    })
}

#[test]
fn send() -> tungstenite::Result<()> {
    task::block_on(async {
        let (server, mut clients) = base(8002, 3).await?;

        let msg = Message::Text("first back".into());
        server.send(Some(0), msg.clone()).await?;
        assert_eq!(clients[0].next().await.unwrap()?, msg);

        let msg = Message::Text("second back".into());
        server.send(Some(1), msg.clone()).await?;
        assert_eq!(clients[1].next().await.unwrap()?, msg);

        let msg = Message::Text("both back".into());
        server.send(None, msg.clone()).await?;
        assert_eq!(clients[0].next().await.unwrap()?, msg);
        assert_eq!(clients[1].next().await.unwrap()?, msg);
        assert_eq!(clients[2].next().await.unwrap()?, msg);

        server.close().await
    })
}

#[test]
fn send_convert() -> tungstenite::Result<()> {
    task::block_on(async {
        let (server, mut clients) = base(8012, 2).await?;

        let msg = "plain";
        server.send(Some(0), msg).await?;
        assert_eq!(clients[0].next().await.unwrap()?, Message::Text(msg.into()));

        assert!(server.send_transform(Some(1), ErrMsg).await.is_err());

        server.send(Some(1), vec![0, 65]).await?;
        assert_eq!(
            clients[1].next().await.unwrap()?,
            Message::Binary(vec![0, 65])
        );

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
