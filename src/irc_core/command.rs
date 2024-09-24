use std::fmt;

use crate::irc_core::message_errors::ErrReply;

#[derive(Debug)]
pub enum Command {
    Pass{password: String, version: String, flags: String, options: Vec<String>},
    Server{servername: String, hopcount: String, token: String, info: String},
    Nick{nickname: String, hopcount: String, username: String, host: String, servertoken: String, umode: String, realname: String},
    Service{servicename: String, servertoken: String, distribution: String, r#type: String, hopcount: String, info: String},
    Quit{quit_message: String},
    Squit,
    Join,
    Njoin,
    Mode,
}

impl Command {
    pub fn parse(_prefix: &Option<String>, command: String, options: &mut Vec<String>) -> Result<Command, ErrReply> {
        match &command[..] {
            "PASS" => {
                let password = match options.pop() {
                    Some(res) => res,
                    None => return Err(ErrReply::SilentDiscard),
                };
                // TODO: if from user or service only use password
                let version = match options.pop() {
                    Some(res) => res,
                    None => return Err(ErrReply::SilentDiscard),
                };
                let flags = match options.pop() {
                    Some(res) => res,
                    None => return Err(ErrReply::SilentDiscard),
                };
                options.reverse();
                Ok(Command::Pass{
                    password, version, flags, options: options.to_vec()
                })
            },

            "SERVER" => {
                let servername = match options.pop() {
                    Some(res) => res,
                    None => return Err(ErrReply::SilentDiscard)
                };
                let hopcount = match options.pop() {
                    Some(res) => res,
                    None => return Err(ErrReply::SilentDiscard)
                };
                let token = match options.pop() {
                    Some(res) => res,
                    None => return Err(ErrReply::SilentDiscard)
                };
                options.reverse();
                let info = options.join(" ");
                Ok(Command::Server {
                    servername, hopcount, token, info
                })
            },

            "NICK" => {
                let nickname = match options.pop() {
                    Some(res) => res,
                    None => return Err(ErrReply::SilentDiscard),
                };
                let hopcount = match options.pop() {
                    Some(res) => res,
                    None => return Err(ErrReply::SilentDiscard),
                };
                let username = match options.pop() {
                    Some(res) => res,
                    None => return Err(ErrReply::SilentDiscard),
                };
                let host = match options.pop() {
                    Some(res) => res,
                    None => return Err(ErrReply::SilentDiscard),
                };
                let servertoken = match options.pop() {
                    Some(res) => res,
                    None => return Err(ErrReply::SilentDiscard),
                };
                let umode = match options.pop() {
                    Some(res) => res,
                    None => return Err(ErrReply::SilentDiscard),
                };
                let realname = match options.pop() {
                    Some(res) => res,
                    None => return Err(ErrReply::SilentDiscard),
                };
                Ok(Command::Nick {
                    nickname, hopcount, username, host, servertoken, umode, realname
                })
            },

            "SERVICE" => {
                let servicename = match options.pop() {
                    Some(res) => res,
                    None => return Err(ErrReply::SilentDiscard),
                };
                let servertoken = match options.pop() {
                    Some(res) => res,
                    None => return Err(ErrReply::SilentDiscard),
                };
                let distribution = match options.pop() {
                    Some(res) => res,
                    None => return Err(ErrReply::SilentDiscard),
                };
                let r#type = match options.pop() {
                    Some(res) => res,
                    None => return Err(ErrReply::SilentDiscard),
                };
                let hopcount = match options.pop() {
                    Some(res) => res,
                    None => return Err(ErrReply::SilentDiscard),
                };
                options.reverse();
                let info = options.join(" ");
                Ok(Command::Service {
                    servicename, servertoken, distribution, r#type, hopcount, info
                })
            },
            "QUIT" => {
                options.reverse();
                let quit_message = options.join(" ");
                Ok(Command::Quit {quit_message})
            },
            _ => Err(ErrReply::SilentDiscard),
        }
    }
}

impl fmt::Display for Command {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
        // or, alternatively:
        // fmt::Debug::fmt(self, f)
    }
}

