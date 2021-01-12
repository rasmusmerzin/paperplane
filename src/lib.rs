mod connection;
mod event;
mod server;

pub use event::Event;
pub use server::Server;

pub use async_tungstenite::tungstenite;

/// Connection id type.
pub type Id = u64;
