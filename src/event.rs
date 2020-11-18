use super::{Id, Message};

#[derive(Debug, PartialEq, Clone)]
pub enum Event {
    Connected(Id),
    Disconnected(Id),
    Kicked(Id, String),
    Message(Id, Message),
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
