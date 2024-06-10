use lazy_static::lazy_static;
use regex::Regex;

use crate::irc::command::{MessageFrom, MessageTo};
use crate::irc::command::ctcp::dcc::{Dcc, DccType};

pub(crate) mod dcc;


lazy_static! {
    static ref CTCP_VERSION_REGEX: Regex = Regex::new(r#"\x01VERSION\x01"#).unwrap();
    static ref CTCP_PING_REGEX: Regex = Regex::new(r#"\x01PING (.*)\x01"#).unwrap();
    static ref CTCP_TIME_REGEX: Regex = Regex::new(r#"\x01TIME\x01"#).unwrap();
    static ref CTCP_CLIENTINFO_REGEX: Regex = Regex::new(r#"\x01CLIENTINFO\x01"#).unwrap();
}
pub(crate) enum CtcpRequestType {
    ClientInfo,
    Dcc(DccType),
    Ping(String),
    Time,
    Version,
}

pub(crate) struct CtcpRequest {
    pub(crate) sender: String,
    pub(crate) request_type: CtcpRequestType,
}

pub(crate) enum CtcpReply {
    Message(MessageTo),
    Dcc(Dcc),
}

impl CtcpRequest {
    pub(crate) fn try_from_request(value: MessageFrom) -> Option<CtcpRequest> {
        value.message.as_str().try_into().ok()
            .map(|request_type| Self {
                sender: value.sender,
                request_type,
            })
    }

    pub(crate) fn generate_reply(self) -> CtcpReply {
        let message = match self.request_type {
            // ACTION is accepted, but we do not handle it in any special way
            CtcpRequestType::ClientInfo => "\x01CLIENTINFO ACTION CLIENTINFO DCC PING TIME VERSION\x01".to_owned(),
            CtcpRequestType::Ping(content) => format!("\x01PING {}\x01", content),
            CtcpRequestType::Time => format!("\x01TIME {}\x01", chrono::Utc::now().to_rfc2822()),
            CtcpRequestType::Version => "\x01VERSION RustIrcClient 0.1-dev\x01".to_owned(),
            // Dcc commands have custom implementation, cannot be handled by a message response
            CtcpRequestType::Dcc(dcc_type) => return CtcpReply::Dcc(Dcc {
                sender: self.sender,
                dcc_type,
            }),
        };
        CtcpReply::Message(MessageTo {
            recipient: self.sender,
            message,
        })
    }
}


impl<'a> TryFrom<&'a str> for CtcpRequestType {
    type Error = &'a str;

    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        if CTCP_VERSION_REGEX.is_match(&value) {
            Ok(Self::Version)
        } else if let Some(captures) = CTCP_PING_REGEX.captures(&value) {
            let content = captures.get(1).unwrap().as_str();
            Ok(Self::Ping(content.to_owned()))
        } else if CTCP_TIME_REGEX.is_match(&value) {
            Ok(Self::Time)
        } else if CTCP_CLIENTINFO_REGEX.is_match(&value) {
            Ok(Self::ClientInfo)
        } else if let Ok(dcc_type) = value.try_into() {
            Ok(Self::Dcc(dcc_type))
        } else {
            Err(value)
        }
    }
}
