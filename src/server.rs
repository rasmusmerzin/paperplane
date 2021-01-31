use crate::connection::Connection;
use crate::tungstenite::{self, Message};
use crate::Event;
use async_std::channel::{bounded, Receiver, Sender};
use async_std::net::{TcpListener, TcpStream, ToSocketAddrs};
use async_std::sync::{Arc, Mutex, RwLock};
use async_std::task;
use futures::stream::StreamExt;
use std::collections::HashMap;
use std::io;

/// A TCP WebSocket Server.
pub struct Server<Session> {
    sender: Mutex<Sender<Event>>,
    receiver: Mutex<Receiver<Event>>,
    connection_seq: Mutex<u128>,
    connections: RwLock<HashMap<u128, (Arc<Connection<TcpStream>>, task::JoinHandle<()>)>>,
    sessions: RwLock<HashMap<u128, Session>>,
    listeners: Mutex<Vec<task::JoinHandle<()>>>,
}

impl<Session> Server<Session>
where
    Session: Default + Clone + Send + Sync + 'static,
{
    /// Create a new `Server` instance.
    /// Takes a capacity for [channel](https://docs.rs/async-std/1.9.0/async_std/channel/fn.bounded.html)
    /// and returns `Arc<Server>` for convenience.
    pub fn new(cap: usize) -> Arc<Self> {
        let (sender, receiver) = bounded(cap);
        Arc::new(Self {
            sender: Mutex::new(sender),
            receiver: Mutex::new(receiver),
            connection_seq: Mutex::new(0),
            connections: RwLock::new(HashMap::new()),
            sessions: RwLock::new(HashMap::new()),
            listeners: Mutex::new(vec![]),
        })
    }

    /// Accept given TcpStream as a WebSocket connection.
    pub async fn accept(self: &Arc<Self>, stream: TcpStream) -> tungstenite::Result<u128> {
        let conn = Arc::new(Connection::accept(stream).await?);

        let conn_id;
        {
            let mut conn_seq = self.connection_seq.lock().await;
            conn_id = *conn_seq;
            *conn_seq += 1;
        }

        let server = self.clone();

        self.connections.write().await.insert(
            conn_id,
            (
                conn.clone(),
                task::spawn(async move {
                    while let Some(msg) = conn.next().await {
                        if let Ok(msg) = msg {
                            if msg.is_binary() || msg.is_text() {
                                server
                                    .sender
                                    .lock()
                                    .await
                                    .send(Event::Message(conn_id, msg))
                                    .await
                                    .ok();
                            }
                        }
                    }
                    server.connections.write().await.remove(&conn_id);
                    server.sessions.write().await.remove(&conn_id);
                    server
                        .sender
                        .lock()
                        .await
                        .send(Event::Disconnected(conn_id))
                        .await
                        .ok();
                }),
            ),
        );

        self.sessions
            .write()
            .await
            .insert(conn_id, Session::default());

        self.sender
            .lock()
            .await
            .send(Event::Connected(conn_id))
            .await
            .ok();

        Ok(conn_id)
    }

    /// Start listening on given socket address.
    pub async fn listen<A>(self: &Arc<Self>, addr: A) -> io::Result<()>
    where
        A: ToSocketAddrs,
    {
        let listener = TcpListener::bind(addr).await?;
        let server = self.clone();
        self.listeners.lock().await.push(task::spawn(async move {
            let mut incoming = listener.incoming();
            while let Some(stream) = incoming.next().await {
                if let Ok(stream) = stream {
                    let server = server.clone();
                    task::spawn(async move {
                        server.accept(stream).await.ok();
                    });
                }
            }
        }));
        Ok(())
    }

    /// Close all connections and stop all listeners.
    pub async fn close(self: &Arc<Self>) -> tungstenite::Result<()> {
        let mut listeners = self.listeners.lock().await;
        while let Some(task) = listeners.pop() {
            task.cancel().await;
        }
        self.kick_all("server closed".into()).await
    }

    /// Get next server event.
    pub async fn next(&self) -> Option<Event> {
        self.receiver.lock().await.next().await
    }

    /// Send a message to a connection with the given id.
    pub async fn send(&self, id: u128, msg: Message) -> tungstenite::Result<()> {
        match self.connections.read().await.get(&id) {
            Some((conn, _)) => conn.send(msg).await,
            None => Err(tungstenite::Error::Io(io::ErrorKind::NotFound.into())),
        }
    }

    /// Send a message to all current connections.
    pub async fn send_all(&self, msg: Message) -> tungstenite::Result<()> {
        let mut tasks = vec![];
        for (conn, _) in self.connections.read().await.values() {
            let msg = msg.clone();
            let conn = conn.clone();
            tasks.push(task::spawn(async move { conn.send(msg).await }));
        }
        let mut result = Ok(());
        for task in tasks {
            result = result.and(task.await);
        }
        result
    }

    /// Loop through connections and send messages determined by the given closure.
    pub async fn send_map<F>(&self, map: F) -> tungstenite::Result<()>
    where
        F: Fn(u128) -> Option<Message>,
    {
        let mut tasks = vec![];
        for (id, (conn, _)) in self.connections.read().await.iter() {
            if let Some(msg) = map(*id) {
                let conn = conn.clone();
                tasks.push(task::spawn(async move { conn.send(msg).await }));
            }
        }
        let mut result = Ok(());
        for task in tasks {
            result = result.and(task.await);
        }
        result
    }

    /// Close a connection with the given id and reason.
    pub async fn kick(&self, id: u128, reason: &str) -> tungstenite::Result<()> {
        match self.connections.write().await.remove(&id) {
            Some((conn, task)) => {
                task.cancel().await;
                self.sender
                    .lock()
                    .await
                    .send(Event::Kicked(id, reason.into()))
                    .await
                    .ok();
                match Arc::try_unwrap(conn) {
                    Ok(conn) => conn.close(reason).await,
                    Err(conn) => conn.close_undefined().await,
                }
            }
            None => Err(tungstenite::Error::Io(io::ErrorKind::NotFound.into())),
        }
    }

    /// Close all current connections with the given reason.
    pub async fn kick_all(self: &Arc<Self>, reason: &str) -> tungstenite::Result<()> {
        let mut tasks = vec![];
        for (id, (conn, task)) in self.connections.write().await.drain() {
            let server = self.clone();
            let reason = reason.to_string();
            tasks.push(task::spawn(async move {
                task.cancel().await;
                server
                    .sender
                    .lock()
                    .await
                    .send(Event::Kicked(id, reason.clone()))
                    .await
                    .ok();
                match Arc::try_unwrap(conn) {
                    Ok(conn) => conn.close(&reason).await,
                    Err(conn) => conn.close_undefined().await,
                }
            }));
        }
        let mut result = Ok(());
        for task in tasks {
            result = result.and(task.await);
        }
        result
    }

    /// Loop through connections and kick the ones for which's id the given closure returns a reason.
    pub async fn kick_map<F>(self: &Arc<Self>, map: F) -> tungstenite::Result<()>
    where
        F: Fn(u128) -> Option<String>,
    {
        let mut tasks = vec![];
        for id in self.connections.read().await.keys() {
            let id = *id;
            if let Some(reason) = map(id) {
                let server = self.clone();
                tasks.push(task::spawn(async move {
                    match server.connections.write().await.remove(&id) {
                        Some((conn, task)) => {
                            task.cancel().await;
                            server
                                .sender
                                .lock()
                                .await
                                .send(Event::Kicked(id, reason.clone()))
                                .await
                                .ok();
                            match Arc::try_unwrap(conn) {
                                Ok(conn) => conn.close(&reason).await,
                                Err(conn) => conn.close_undefined().await,
                            }
                        }
                        None => Err(tungstenite::Error::Io(io::ErrorKind::NotFound.into())),
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

    /// Get session by connection id.
    pub async fn get_session(&self, id: u128) -> Option<Session> {
        self.sessions.read().await.get(&id).map(|sess| sess.clone())
    }

    /// Set session by connection id and return the previous session.
    pub async fn set_session(&self, id: u128, state: Session) -> io::Result<Session> {
        let mut sess_write = self.sessions.write().await;
        match sess_write.contains_key(&id) {
            true => Ok(sess_write.insert(id, state.into()).unwrap()),
            false => Err(io::ErrorKind::NotFound.into()),
        }
    }

    /// Replace session for connection id using the given predicate.
    pub async fn update_session<F>(&self, id: u128, mut predicate: F) -> io::Result<()>
    where
        F: FnMut(&Session) -> Session,
    {
        self.sessions
            .write()
            .await
            .get_mut(&id)
            .map(|sess| *sess = predicate(sess))
            .ok_or(io::ErrorKind::NotFound.into())
    }

    /// Find the first connection id and session pair that matches the given predicate.
    pub async fn find_session<F>(&self, predicate: F) -> Option<(u128, Session)>
    where
        F: Fn(&Session) -> bool,
    {
        self.sessions
            .read()
            .await
            .iter()
            .find_map(|(id, sess)| match predicate(sess) {
                true => Some((*id, sess.clone())),
                false => None,
            })
    }

    /// Update the first pair that the given predicate returns some for.
    pub async fn find_and_update_session<F>(&self, mut predicate: F) -> io::Result<()>
    where
        F: FnMut(u128, &Session) -> Option<Session>,
    {
        let mut sess_w = self.sessions.write().await;
        sess_w
            .iter()
            .find_map(|(id, sess)| predicate(*id, sess).map(|sess| (*id, sess)))
            .map(|(id, new_sess)| {
                sess_w.insert(id, new_sess);
            })
            .ok_or(io::ErrorKind::NotFound.into())
    }

    /// Get a list of connection id and session pairs that match the given predicate.
    pub async fn filter_sessions<F>(&self, predicate: F) -> Vec<(u128, Session)>
    where
        F: Fn(&Session) -> bool,
    {
        self.sessions
            .read()
            .await
            .iter()
            .filter_map(|(id, sess)| match predicate(sess) {
                true => Some((*id, sess.clone())),
                false => None,
            })
            .collect()
    }

    /// Filter and update pairs that the given predicate returns some for.
    pub async fn filter_and_update_sessions<F>(&self, mut predicate: F) -> usize
    where
        F: FnMut(u128, &Session) -> Option<Session>,
    {
        let mut sess_w = self.sessions.write().await;
        let updates: Vec<(u128, Session)> = sess_w
            .iter()
            .filter_map(|(id, sess)| predicate(*id, sess).map(|sess| (*id, sess)))
            .collect();
        let count = updates.len();
        for (id, new_sess) in updates {
            sess_w.insert(id, new_sess);
        }
        count
    }
}
