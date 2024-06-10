use lazy_static::lazy_static;
use regex::Regex;

pub(crate) mod ctcp;
pub mod xdcc;


lazy_static! {
    static ref PRIVMSG_REGEX: Regex = Regex::new(r#":([^!]*)!.* PRIVMSG .* :(.*)"#).unwrap();
    static ref PING_REGEX: Regex = Regex::new(r#"PING (.*)"#).unwrap();
}

#[derive(Debug)]
pub(crate) struct MessageFrom {
    pub(crate) sender: String,
    pub(crate) message: String,
}

#[derive(Debug)]
pub(crate) struct MessageTo {
    pub(crate) recipient: String,
    pub(crate) message: String,
}

#[derive(Debug)]
pub(crate) enum ServerCommand {
    PrivMsg(MessageFrom),
    Ping(String),
    #[allow(dead_code)]
    Unknown(String),
}

#[derive(Debug)]
pub(crate) enum ClientCommand {
    Pong(String),
    Join(String),
    PrivMsg(MessageTo),
    Quit(String),
    Nick(String),
    User(String),
    Notice(MessageTo),
}


impl From<&str> for ServerCommand {
    fn from(value: &str) -> Self {
        if let Some(captures) = PING_REGEX.captures(value) {
            Self::Ping(captures.get(1).unwrap().as_str().to_string())
        } else if let Some(captures) = PRIVMSG_REGEX.captures(value) {
            Self::PrivMsg(MessageFrom {
                sender: captures.get(1).unwrap().as_str().to_string(),
                message: captures.get(2).unwrap().as_str().to_string(),
            })
        } else {
            Self::Unknown(value.to_string())
        }
    }
}

impl From<&ClientCommand> for String {
    fn from(value: &ClientCommand) -> Self {
        match value {
            ClientCommand::Pong(content) => format!("PONG {}\r\n", content),
            ClientCommand::Join(channel) => format!("JOIN :{}\r\n", channel),
            ClientCommand::PrivMsg(message) => format!("PRIVMSG {} :{}\r\n", message.recipient, message.message),
            ClientCommand::Quit(message) => format!("QUIT :{}\r\n", message),
            ClientCommand::User(nickname) => format!("USER {} 0 * {}\r\n", nickname, nickname),
            ClientCommand::Nick(nickname) => format!("NICK {}\r\n", nickname),
            ClientCommand::Notice(message) => format!("NOTICE {} :{}\r\n", message.recipient, message.message),
        }
    }
}
