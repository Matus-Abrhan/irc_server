use std::fmt;

use crate::irc_core::message_errors::IRCError;

#[derive(Debug)]
pub enum Command {
    Pass{password: String}, //, version: Option<String>, flags: Option<String>, options: Option<Vec<String>>
    Nick{nickname: String}, //, hopcount: String, username: String, host: String, servertoken: String, umode: String, realname: String
    User{user: String, mode: String, unused: String, realname: String},
    Ping{token: String},
    Pong{server: Option<String>, token: String},
    Oper{name: String, password: String},
    Quit{reason: Option<String>},
    Error{reason: String},
}

impl Command {
    pub fn parse(_prefix: &Option<String>, command: String, options: &mut Vec<String>) -> Result<Command, IRCError> {
        match &command[..] {
            "PASS" => {
                let password = match options.pop() {
                    Some(res) => res,
                    None => return Err(IRCError::NeedMoreParams),
                };
                Ok(Command::Pass{password})
            },

            "NICK" => {
                let nickname = match options.pop() {
                    Some(res) => res,
                    None => return Err(IRCError::NoNicknameGiven),
                };
                Ok(Command::Nick{nickname})
            },

            "USER" => {
                let user = match options.pop() {
                    Some(res) => res,
                    None => return Err(IRCError::SilentDiscard),
                };
                let mode= match options.pop() {
                    Some(res) => res,
                    None => return Err(IRCError::SilentDiscard),
                };
                let unused = match options.pop() {
                    Some(res) => res,
                    None => return Err(IRCError::SilentDiscard),
                };
                options.reverse();
                Ok(Command::User{user, mode, unused, realname: options.join(" ")})
            },

            "PING" => {
                let token = options.pop().ok_or(IRCError::NeedMoreParams)?;
                Ok(Command::Ping{token})
            },

            "PONG" => {
                let mut server: Option<String> = None;
                if options.len() > 1 {
                    server = Some(options.pop().ok_or(IRCError::SilentDiscard)?);
                }
                let token = options.pop().ok_or(IRCError::SilentDiscard)?;
                Ok(Command::Pong{server, token})
            },

            "OPER" => {
                // let name = options.pop().ok_or(IRCError::NeedMoreParams)?;
                let name = match options.pop() {
                    Some(res) => res,
                    None => return Err(IRCError::NeedMoreParams),
                };
                // let password = options.pop().ok_or(IRCError::NeedMoreParams)?;
                let password = match options.pop() {
                    Some(res) => res,
                    None => return Err(IRCError::NeedMoreParams),
                };
                Ok(Command::Oper{name, password})
            },

            "QUIT" => {
                if options.is_empty() {
                    return Ok(Command::Quit {reason: None});
                }
                options.reverse();
                let reason = options.join(" ");
                Ok(Command::Quit{reason: Some(reason)})
            },
            "ERROR" => {
                let reason = match options.pop() {
                    Some(res) => res,
                    None => return Err(IRCError::SilentDiscard),
                };
                Ok(Command::Error{reason})
            }

            _ => Err(IRCError::SilentDiscard),
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

