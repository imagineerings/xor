use rpc::proto;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MessagePriority {
    #[default]
    Normal,
    Important,
    Urgent,
}

impl MessagePriority {
    pub fn from_proto_value(priority: i32) -> Self {
        match priority {
            1 => Self::Important,
            2 => Self::Urgent,
            _ => Self::Normal,
        }
    }

    pub fn label(self) -> Option<&'static str> {
        match self {
            Self::Normal => None,
            Self::Important => Some("Important"),
            Self::Urgent => Some("Urgent"),
        }
    }

    pub fn color_token(self) -> Option<&'static str> {
        match self {
            Self::Normal => None,
            Self::Important => Some("warning"),
            Self::Urgent => Some("error"),
        }
    }

    pub fn icon_token(self) -> Option<&'static str> {
        match self {
            Self::Normal => None,
            Self::Important => Some("alert-triangle"),
            Self::Urgent => Some("alert-octagon"),
        }
    }

    pub fn to_proto(self) -> i32 {
        match self {
            Self::Normal => 0,
            Self::Important => 1,
            Self::Urgent => 2,
        }
    }
}

impl From<proto::ChannelMessagePriority> for MessagePriority {
    fn from(priority: proto::ChannelMessagePriority) -> Self {
        match priority {
            proto::ChannelMessagePriority::Important => Self::Important,
            proto::ChannelMessagePriority::Urgent => Self::Urgent,
            proto::ChannelMessagePriority::Normal => Self::Normal,
        }
    }
}
