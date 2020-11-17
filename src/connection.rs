use async_std::net::TcpStream;
use async_std::sync::Mutex;
use async_tungstenite::tungstenite::protocol::CloseFrame;
use async_tungstenite::tungstenite::protocol::frame::coding::CloseCode;
use async_tungstenite::{accept_async, WebSocketStream};
use futures::sink::SinkExt;
use futures::stream::{SplitSink, SplitStream, StreamExt};
use std::io;

pub use async_tungstenite::tungstenite::{Message, Error as WsError, Result as WsResult};

pub struct Connection {
    sender: Mutex<SplitSink<WebSocketStream<TcpStream>, Message>>,
    receiver: Mutex<SplitStream<WebSocketStream<TcpStream>>>,
}

impl Connection {
    pub async fn accept(stream: TcpStream) -> WsResult<Self> {
        let (sender, receiver) = accept_async(stream).await?.split();
        Ok(Self {
            sender: Mutex::new(sender),
            receiver: Mutex::new(receiver)
        })
    }

    pub async fn next(&self) -> Option<WsResult<Message>> {
        self.receiver.lock().await.next().await
    }

    pub async fn send(&self, msg: Message) -> WsResult<()> {
        self.sender.lock().await.send(msg).await
    }

    pub async fn close_undefined(&self) -> WsResult<()> {
        self.sender.lock().await.close().await
    }

    pub async fn close(self, reason: &str) -> WsResult<()> {
        match self.sender.into_inner().reunite(self.receiver.into_inner()) {
            Ok(mut ws) => {
                ws.close(Some(CloseFrame {
                    code: CloseCode::Normal,
                    reason: reason.into(),
                })).await
            }
            Err(_) => Err(WsError::Io(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "could not join websocket sender & receiver"
            ))),
        }
    }
}
