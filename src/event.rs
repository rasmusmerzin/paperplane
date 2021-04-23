use crate::tungstenite::Message;
use std::convert::TryInto;

/// An enum representing server events.
#[derive(Debug, PartialEq, Clone)]
pub enum Event<Msg = Message> {
    /// New connection created with the id.
    Connected(u128),
    /// Connection with the id closed by client.
    Disconnected(u128),
    /// Connection with the id closed by the server with the reason.
    Kicked(u128, String),
    /// Message sent by a client with the connection id.
    Message(u128, Msg),
}

impl<M> Event<M> {
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

    pub fn into<T>(self) -> Event<T>
    where
        M: Into<T>,
    {
        match self {
            Self::Connected(id) => Event::Connected(id),
            Self::Disconnected(id) => Event::Disconnected(id),
            Self::Kicked(id, reason) => Event::Kicked(id, reason),
            Self::Message(id, msg) => Event::Message(id, msg.into()),
        }
    }

    pub fn try_into<T>(self) -> Result<Event<T>, <M as TryInto<T>>::Error>
    where
        M: TryInto<T>,
    {
        Ok(match self {
            Self::Connected(id) => Event::Connected(id),
            Self::Disconnected(id) => Event::Disconnected(id),
            Self::Kicked(id, reason) => Event::Kicked(id, reason),
            Self::Message(id, msg) => Event::Message(id, msg.try_into()?),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tungstenite::Message;
    use std::convert::TryFrom;
    use std::string::FromUtf8Error;

    #[derive(Debug, PartialEq)]
    struct Msg(String);

    impl From<Message> for Msg {
        fn from(msg: Message) -> Self {
            Self(String::from_utf8_lossy(&msg.into_data()).into())
        }
    }

    #[derive(Debug, PartialEq)]
    struct TrueMsg(String);

    impl TryFrom<Message> for TrueMsg {
        type Error = FromUtf8Error;
        fn try_from(msg: Message) -> Result<Self, Self::Error> {
            Ok(Self(String::from_utf8(msg.into_data())?))
        }
    }

    #[test]
    fn variant_connected() {
        let event = Event::Connected::<()>(0);
        assert!(event.is_connected());
        assert!(!event.is_disconnected());
        assert!(!event.is_kicked());
        assert!(!event.is_message());
    }

    #[test]
    fn variant_disconnected() {
        let event = Event::Disconnected::<()>(0);
        assert!(!event.is_connected());
        assert!(event.is_disconnected());
        assert!(!event.is_kicked());
        assert!(!event.is_message());
    }

    #[test]
    fn variant_kicked() {
        let event = Event::Kicked::<()>(0, "".into());
        assert!(!event.is_connected());
        assert!(!event.is_disconnected());
        assert!(event.is_kicked());
        assert!(!event.is_message());
    }

    #[test]
    fn variant_message() {
        let event = Event::Message(0, Message::Text("some".into()));
        assert!(!event.is_connected());
        assert!(!event.is_disconnected());
        assert!(!event.is_kicked());
        assert!(event.is_message());
    }

    #[test]
    fn into() {
        let event: Event<Msg> = Event::Message(0, Message::Text("some".into())).into();
        assert_eq!(event, Event::Message(0, Msg("some".into())));
    }

    #[test]
    fn try_into() {
        let res = Event::Message(0, Message::Text("some".into())).try_into::<TrueMsg>();
        assert_eq!(res, Ok(Event::Message(0, TrueMsg("some".into()))));

        let res = Event::Message(0, Message::Binary(vec![0, 159])).try_into::<TrueMsg>();
        assert!(res.is_err());
    }
}
