mod connection;
mod event;
mod server;

pub type Message = async_tungstenite::tungstenite::Message;
pub type WsError = async_tungstenite::tungstenite::Error;
pub type WsResult<T> = async_tungstenite::tungstenite::Result<T>;
pub type Id = usize;

pub use event::Event;
pub use server::Server;
