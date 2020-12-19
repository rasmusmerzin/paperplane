mod connection;
mod event;
mod server;

/// Re-export of `tungstenite::Message`.
pub type Message = async_tungstenite::tungstenite::Message;
/// Re-export of `tungstenite::Error`.
pub type WsError = async_tungstenite::tungstenite::Error;
/// Re-export of `tungstenite::Result`.
pub type WsResult<T> = async_tungstenite::tungstenite::Result<T>;
/// Connection id.
pub type Id = usize;

pub use event::Event;
pub use server::Server;
