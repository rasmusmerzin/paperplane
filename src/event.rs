use crate::tungstenite::Message;

/// An enum representing server events.
#[derive(Debug, PartialEq, Clone)]
pub enum Event {
    /// New connection created with the id.
    Connected(u128),
    /// Connection with the id closed by client.
    Disconnected(u128),
    /// Connection with the id closed by the server with the reason.
    Kicked(u128, String),
    /// Message sent by a client with the connection id.
    Message(u128, Message),
}

impl Event {
    pub fn is_connected(&self) -> bool {
        match self {
            Event::Connected(_) => true,
            _ => false,
        }
    }

    pub fn is_disconnected(&self) -> bool {
        match self {
            Event::Disconnected(_) => true,
            _ => false,
        }
    }

    pub fn is_kicked(&self) -> bool {
        match self {
            Event::Kicked(_, _) => true,
            _ => false,
        }
    }

    pub fn is_message(&self) -> bool {
        match self {
            Event::Message(_, _) => true,
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn variant_checks() {
        let event = Event::Connected(0);
        assert!(event.is_connected());
        assert!(!event.is_disconnected());
        assert!(!event.is_kicked());
        assert!(!event.is_message());

        let event = Event::Disconnected(0);
        assert!(!event.is_connected());
        assert!(event.is_disconnected());
        assert!(!event.is_kicked());
        assert!(!event.is_message());

        let event = Event::Kicked(0, "".into());
        assert!(!event.is_connected());
        assert!(!event.is_disconnected());
        assert!(event.is_kicked());
        assert!(!event.is_message());

        let event = Event::Message(0, Message::Text("".into()));
        assert!(!event.is_connected());
        assert!(!event.is_disconnected());
        assert!(!event.is_kicked());
        assert!(event.is_message());
    }
}
