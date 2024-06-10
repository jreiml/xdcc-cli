use lazy_static::lazy_static;
use regex::Regex;

use crate::irc::command::MessageTo;

lazy_static! {
    static ref XDCC_REMOVE_REGEX: Regex = Regex::new(r#"/msg (.*) xdcc remove #?(\d+)"#).unwrap();
    static ref XDCC_REMOVE_ALL_REGEX: Regex = Regex::new(r#"/msg (.*) xdcc remove( all)?"#).unwrap();
    static ref XDCC_CANCEL_REGEX: Regex = Regex::new(r#"/msg (.*) xdcc cancel"#).unwrap();
    static ref XDCC_INFO_REGEX: Regex = Regex::new(r#"/msg (.*) xdcc info #?(\d+)"#).unwrap();
    static ref XDCC_SEND_REGEX: Regex = Regex::new(r#"/msg (.*) xdcc (send|get) #?(-1|\d+|list)"#).unwrap();
    static ref XDCC_BATCH_REGEX: Regex = Regex::new(r#"/msg (.*) xdcc batch ((#?\d+,)*#?\d+)"#).unwrap();
}


#[derive(Debug, Clone)]
pub enum Xdcc {
    Cancel(String),
    Send(String, Option<u32>),
    Info(String, u32),
    Remove(String, Option<u32>),
    Batch(String, Vec<u32>),
}

impl Xdcc {
    pub fn recipient(&self) -> &str {
        match self {
            Xdcc::Cancel(recipient) => recipient,
            Xdcc::Send(recipient, _) => recipient,
            Xdcc::Info(recipient, _) => recipient,
            Xdcc::Batch(recipient, _) => recipient,
            Xdcc::Remove(recipient, _) => recipient
        }.as_str()
    }
}

impl From<Xdcc> for MessageTo {
    fn from(value: Xdcc) -> Self {
        match value {
            Xdcc::Cancel(recipient) => {
                MessageTo {
                    recipient,
                    message: "xdcc cancel".to_owned(),
                }
            }
            Xdcc::Send(recipient, package) => {
                let message = if let Some(package) = package {
                    format!("xdcc send #{}", package)
                } else {
                    "xdcc send -1".into()
                };
                MessageTo {
                    recipient,
                    message,
                }
            }
            Xdcc::Info(recipient, package) => {
                MessageTo {
                    recipient,
                    message: format!("xdcc info #{}", package),
                }
            }
            Xdcc::Batch(recipient, packages) => {
                let packages = packages.iter().map(|p| format!("{}", p)).collect::<Vec<String>>().join(",");
                MessageTo {
                    recipient,
                    message: format!("xdcc batch {}", packages),
                }
            }
            Xdcc::Remove(recipient, package) => {
                let message = if let Some(package) = package {
                    format!("xdcc remove #{}", package)
                } else {
                    "xdcc remove all".into()
                };
                MessageTo {
                    recipient,
                    message
                }
            }
        }
    }
}

impl<'a> TryFrom<&'a str> for Xdcc {
    type Error = &'a str;

    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        let lower_value = value.to_lowercase();
        if let Some(captures) = XDCC_CANCEL_REGEX.captures(&lower_value) {
            Ok(Self::Cancel(captures.get(1).unwrap().as_str().to_string()))
        } else if let Some(captures) = XDCC_SEND_REGEX.captures(&lower_value) {
            let package = captures.get(3).unwrap().as_str();
            let package = if package != "list" {
                let package = package.parse::<i32>().unwrap();
                if package != -1 {
                    Some(package as u32)
                } else {
                    None
                }
            } else {
                None
            };
            Ok(Self::Send(
                captures.get(1).unwrap().as_str().to_string(),
                package,
            ))
        }  else if let Some(captures) = XDCC_INFO_REGEX.captures(&lower_value) {
            let package = captures.get(2).unwrap().as_str().parse::<u32>().unwrap();
            Ok(Self::Info(
                captures.get(1).unwrap().as_str().to_string(),
                package,
            ))
        } else if let Some(captures) = XDCC_BATCH_REGEX.captures(&lower_value) {
            let packages = captures.get(2).unwrap().as_str()
                .split(',')
                .map(|s| s.trim_start_matches('#').parse::<u32>().unwrap())
                .collect::<Vec<_>>();
            Ok(Self::Batch(
                captures.get(1).unwrap().as_str().to_string(),
                packages,
            ))
        } else if let Some(captures) = XDCC_REMOVE_REGEX.captures(&lower_value) {
            let package = captures.get(2).unwrap().as_str().parse::<u32>().unwrap();
            Ok(Self::Remove(
                captures.get(1).unwrap().as_str().to_string(),
                Some(package),
            ))
        } else if let Some(captures) = XDCC_REMOVE_ALL_REGEX.captures(&lower_value) {
            Ok(Self::Remove(
                captures.get(1).unwrap().as_str().to_string(),
                None
            ))
        } else {
            Err(value)
        }
    }
}
