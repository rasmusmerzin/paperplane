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

#[derive(Debug, Default, Clone, PartialEq)]
struct Session {
    txt: String,
}

async fn base(
    port: usize,
    count: u128,
) -> tungstenite::Result<(Arc<Server<Session>>, Vec<WebSocketStream<ConnectStream>>)> {
    let addr = format!("{}:{}", LOCALHOST, port);
    let server = Server::new(10);
    server.listen(&addr).await?;

    let mut clients = vec![];
    for i in 0..count {
        clients.push(connect_async(format!("ws://{}", addr)).await?.0);
        assert_eq!(server.next().await, Some(Event::Connected(i)));
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
        let count = 4u128;
        let (server, _clients) = base(8005, count).await?;

        for i in 0..count {
            assert_eq!(server.get_session(i).await, Some(Session::default()));
        }
        for i in 0..count {
            let mut new_sess = None;
            server
                .update_session(i, |_| {
                    let sess = Session { txt: i.to_string() };
                    new_sess = Some(sess.clone());
                    sess
                })
                .await
                .unwrap();
            assert_eq!(new_sess, Some(Session { txt: i.to_string() }));
        }
        for i in 0..count {
            assert_eq!(
                server.get_session(i).await,
                Some(Session { txt: i.to_string() })
            );
        }
        for i in 0..count {
            assert_eq!(
                server.find_session(|sess| sess.txt == i.to_string()).await,
                Some((i, Session { txt: i.to_string() }))
            );
        }
        for i in 0..count {
            let mut old_sess = None;
            server
                .find_and_update_session(|_, sess| match sess.txt == i.to_string() {
                    true => {
                        old_sess = Some(sess.clone());
                        Some(Session {
                            txt: format!("{}{0}", sess.txt),
                        })
                    }
                    false => None,
                })
                .await
                .unwrap();
            assert_eq!(old_sess, Some(Session { txt: i.to_string() }));
        }
        for i in 0..count {
            assert_eq!(
                server.get_session(i).await,
                Some(Session {
                    txt: format!("{}{0}", i)
                })
            );
        }
        {
            let mut sessions = server
                .filter_sessions(|sess| sess.txt == "11" || sess.txt == "22")
                .await;
            sessions.sort_unstable_by_key(|(id, _)| *id);
            assert_eq!(
                sessions,
                &[
                    (1, Session { txt: "11".into() }),
                    (2, Session { txt: "22".into() })
                ]
            );
        }
        assert_eq!(
            server
                .filter_and_update_sessions(|id, _| match id {
                    2 | 3 => Some(Session {
                        txt: id.to_string()
                    }),
                    _ => None,
                })
                .await,
            2
        );
        for i in 0..count {
            assert_eq!(
                server.get_session(i).await,
                Some(Session {
                    txt: match i {
                        2 | 3 => i.to_string(),
                        _ => format!("{}{0}", i),
                    }
                })
            );
        }

        Ok(())
    })
}

#[test]
fn state_cleanup() -> tungstenite::Result<()> {
    task::block_on(async {
        let (server, mut clients) = base(8006, 8).await?;

        for i in 0..2 {
            clients[i].close(None).await?;
            assert_eq!(server.next().await, Some(Event::Disconnected(i as u128)));
        }

        {
            let mut sessions = server.filter_sessions(|_| true).await;
            sessions.sort_unstable_by_key(|(id, _)| *id);
            let (ids, _): (Vec<_>, Vec<_>) = sessions.iter().cloned().unzip();
            assert_eq!(ids, &[2, 3, 4, 5, 6, 7]);
        }

        server.kick(2, "").await?;
        server.kick(3, "").await?;

        {
            let mut sessions = server.filter_sessions(|_| true).await;
            sessions.sort_unstable_by_key(|(id, _)| *id);
            let (ids, _): (Vec<_>, Vec<_>) = sessions.iter().cloned().unzip();
            assert_eq!(ids, &[4, 5, 6, 7]);
        }

        server
            .kick_map(|id| match id {
                6 | 7 => Some("".into()),
                _ => None,
            })
            .await?;

        {
            let mut sessions = server.filter_sessions(|_| true).await;
            sessions.sort_unstable_by_key(|(id, _)| *id);
            let (ids, _): (Vec<_>, Vec<_>) = sessions.iter().cloned().unzip();
            assert_eq!(ids, &[4, 5]);
        }

        server.kick_all("").await?;

        {
            let mut sessions = server.filter_sessions(|_| true).await;
            sessions.sort_unstable_by_key(|(id, _)| *id);
            let (ids, _): (Vec<_>, Vec<_>) = sessions.iter().cloned().unzip();
            assert_eq!(ids, &[]);
        }

        Ok(())
    })
}
