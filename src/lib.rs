mod connection;
mod event;
mod server;

pub type Id = usize;
pub use connection::{Message, WsError, WsResult};
pub use event::Event;
pub use server::Server;
