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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn priority_display_metadata_matches_each_level() {
        assert_eq!(MessagePriority::Normal.label(), None);
        assert_eq!(MessagePriority::Normal.color_token(), None);
        assert_eq!(MessagePriority::Normal.icon_token(), None);

        assert_eq!(MessagePriority::Important.label(), Some("Important"));
        assert_eq!(MessagePriority::Important.color_token(), Some("warning"));
        assert_eq!(
            MessagePriority::Important.icon_token(),
            Some("alert-triangle")
        );

        assert_eq!(MessagePriority::Urgent.label(), Some("Urgent"));
        assert_eq!(MessagePriority::Urgent.color_token(), Some("error"));
        assert_eq!(MessagePriority::Urgent.icon_token(), Some("alert-octagon"));
    }

    #[test]
    fn priority_protocol_conversion_preserves_known_values() {
        for (priority, value, proto_priority) in [
            (
                MessagePriority::Normal,
                0,
                proto::ChannelMessagePriority::Normal,
            ),
            (
                MessagePriority::Important,
                1,
                proto::ChannelMessagePriority::Important,
            ),
            (
                MessagePriority::Urgent,
                2,
                proto::ChannelMessagePriority::Urgent,
            ),
        ] {
            assert_eq!(priority.to_proto(), value);
            assert_eq!(MessagePriority::from_proto_value(value), priority);
            assert_eq!(MessagePriority::from(proto_priority), priority);
        }
    }

    #[test]
    fn unrecognized_protocol_values_default_to_normal() {
        assert_eq!(
            MessagePriority::from_proto_value(-1),
            MessagePriority::Normal
        );
        assert_eq!(
            MessagePriority::from_proto_value(3),
            MessagePriority::Normal
        );
    }
}
