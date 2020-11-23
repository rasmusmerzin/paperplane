use async_std::net::{TcpListener, TcpStream, ToSocketAddrs};
use async_std::sync::{Arc, Mutex, RwLock};
use async_std::task;
use futures::channel::mpsc::{unbounded, UnboundedSender, UnboundedReceiver};
use futures::sink::SinkExt;
use futures::stream::StreamExt;
use std::collections::HashMap;
use std::io;
use std::num::Wrapping;

mod connection;
mod event;

pub use connection::{Connection, Message, WsError, WsResult};
pub use event::Event;

pub type Id = usize;

pub struct Server {
    sender: Mutex<UnboundedSender<Event>>,
    receiver: Mutex<UnboundedReceiver<Event>>,
    connection_seq: Mutex<Wrapping<Id>>,
    connections: RwLock<HashMap<Id, (Arc<Connection>, task::JoinHandle<()>)>>,
    listeners: RwLock<Vec<task::JoinHandle<()>>>,
}

impl Server {
    pub fn new() -> Arc<Self> {
        let (sender, receiver) = unbounded();
        Arc::new(Self {
            sender: Mutex::new(sender),
            receiver: Mutex::new(receiver),
            connection_seq: Mutex::new(Wrapping(Id::MIN)),
            connections: RwLock::new(HashMap::new()),
            listeners: RwLock::new(vec![])
        })
    }

    pub async fn add(self: &Arc<Self>, stream: TcpStream) -> WsResult<Id> {
        let conn = Arc::new(Connection::accept(stream).await?);

        let conn_id;
        {
            let mut conn_seq = self.connection_seq.lock().await;
            conn_id = *conn_seq;
            *conn_seq += Wrapping(1);
        }

        let server = self.clone();

        self.connections.write().await.insert(conn_id.0, (
            conn.clone(),
            task::spawn(async move {
                while let Some(msg) = conn.next().await {
                    if let Ok(msg) = msg {
                        if msg.is_binary() || msg.is_text() {
                            server.sender.lock().await.send(Event::Message(conn_id.0, msg)).await.ok();
                        }
                    }
                }

                server.connections.write().await.remove(&conn_id.0);

                server.sender.lock().await.send(Event::Disconnected(conn_id.0)).await.ok();
            }),
        ));

        self.sender.lock().await.send(Event::Connected(conn_id.0)).await.ok();

        Ok(conn_id.0)
    }

    pub async fn listen<A: ToSocketAddrs>(self: &Arc<Self>, addr: A) -> io::Result<()> {
        let listener = TcpListener::bind(addr).await?;
        let server = self.clone();
        self.listeners.write().await.push(
            task::spawn(async move {
                while let Ok((stream, _)) = listener.accept().await {
                    let server = server.clone();
                    task::spawn(async move {
                        server.add(stream).await.ok();
                    });
                }
            })
        );
        Ok(())
    }

    pub async fn listener_count(self: &Arc<Self>) -> usize {
        self.listeners.read().await.len()
    }

    pub async fn close(self: &Arc<Self>) -> WsResult<()> {
        let mut listeners = self.listeners.write().await;
        while let Some(task) = listeners.pop() {
            task.cancel().await;
        }
        self.kick_all("server closed".into()).await
    }

    pub async fn next(&self) -> Option<Event> {
        self.receiver.lock().await.next().await
    }

    pub async fn send(&self, id: Id, msg: Message) -> WsResult<()> {
        if let Some((conn, _)) = self.connections.read().await.get(&id) {
            conn.send(msg).await
        } else {
            Err(WsError::Io(io::Error::new(io::ErrorKind::NotFound, "connection not found")))
        }
    }

    pub async fn send_all(&self, msg: Message) -> WsResult<()> {
        let mut tasks = vec![];
        for (conn, _) in self.connections.read().await.values() {
            let msg = msg.clone();
            let conn = conn.clone();
            tasks.push(task::spawn(async move {
                conn.send(msg).await
            }));
        }
        let mut result = Ok(());
        for task in tasks {
            result = result.and(task.await);
        }
        result
    }

    pub async fn send_map<F: Fn(Id) -> Option<Message>>(&self, map: F) -> WsResult<()> {
        let mut tasks = vec![];
        for (id, (conn, _)) in self.connections.read().await.iter() {
            if let Some(msg) = map(*id) {
                let conn = conn.clone();
                tasks.push(task::spawn(async move {
                    conn.send(msg).await
                }));
            }
        }
        let mut result = Ok(());
        for task in tasks {
            result = result.and(task.await);
        }
        result
    }

    pub async fn kick(&self, id: Id, reason: &str) -> WsResult<()> {
        if let Some((conn, task)) = self.connections.write().await.remove(&id) {
            task.cancel().await;
            self.sender.lock().await.send(Event::Kicked(id, reason.into())).await.ok();
            match Arc::try_unwrap(conn) {
                Ok(conn) => conn.close(reason).await,
                Err(conn) =>  conn.close_undefined().await,
            }
        } else {
            Err(WsError::Io(io::Error::new(io::ErrorKind::NotFound, "connection not found")))
        }
    }

    pub async fn kick_all(self: &Arc<Self>, reason: String) -> WsResult<()> {
        let mut tasks = vec![];
        for (id, (conn, task)) in self.connections.write().await.drain() {
            let server = self.clone();
            let reason = reason.clone();
            tasks.push(task::spawn(async move {
                task.cancel().await;
                server.sender.lock().await.send(Event::Kicked(id, reason.clone())).await.ok();
                match Arc::try_unwrap(conn) {
                    Ok(conn) => conn.close(&reason).await,
                    Err(conn) =>  conn.close_undefined().await,
                }
            }));
        }
        let mut result = Ok(());
        for task in tasks {
            result = result.and(task.await);
        }
        result
    }

    pub async fn kick_map<F: Fn(Id) -> Option<String>>(self: &Arc<Self>, map: F) -> WsResult<()> {
        let mut tasks = vec![];
        for id in self.connections.read().await.keys() {
            let id = *id;
            if let Some(reason) = map(id) {
                let server = self.clone();
                tasks.push(task::spawn(async move {
                    if let Some((conn, task)) = server.connections.write().await.remove(&id) {
                        task.cancel().await;
                        server.sender.lock().await.send(Event::Kicked(id, reason.clone())).await.ok();
                        match Arc::try_unwrap(conn) {
                            Ok(conn) => conn.close(&reason).await,
                            Err(conn) =>  conn.close_undefined().await,
                        }
                    } else {
                        Err(WsError::Io(io::Error::new(io::ErrorKind::NotFound, "connection not found")))
                    }
                }));
            }
        }
        let mut result = Ok(());
        for task in tasks {
            result = result.and(task.await);
        }
        result
    }
}
