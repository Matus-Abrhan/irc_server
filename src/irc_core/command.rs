use std::fmt;

use crate::irc_core::message_errors::IRCError;

#[derive(Debug)]
pub enum Command {
    Cap,  //{subcommand: String, capabilities: String},
    Pass{password: String}, //, version: Option<String>, flags: Option<String>, options: Option<Vec<String>>
    Nick{nickname: String}, //, hopcount: String, username: String, host: String, servertoken: String, umode: String, realname: String
    User{user: String, mode: String, unused: String, realname: String},
    Ping{token: String},
    Pong{server: Option<String>, token: String},
    Oper{name: String, password: String},
    Quit{reason: Option<String>},
    Error{reason: String},

    Join{channels: String, keys: Option<String>},

    PrivMsg{targets: String, text: String},
}

impl Command {
    pub fn parse(_prefix: &Option<String>, command: String, options: &mut Vec<String>) -> Result<Command, IRCError> {
        match &command[..] {
            "CAP" => {
                Ok(Command::Cap)
            },

            "PASS" => {
                let password = options.pop().ok_or(IRCError::SilentDiscard)?;
                Ok(Command::Pass{password})
            },

            "NICK" => {
                let nickname = options.pop().ok_or(IRCError::SilentDiscard)?;
                Ok(Command::Nick{nickname})
            },

            "USER" => {
                let user = options.pop().ok_or(IRCError::SilentDiscard)?;
                let mode = options.pop().ok_or(IRCError::SilentDiscard)?;
                let unused = options.pop().ok_or(IRCError::SilentDiscard)?;
                options.reverse();
                Ok(Command::User{user, mode, unused, realname: options.join(" ")})
            },

            "PING" => {
                let token = options.pop().ok_or(IRCError::NeedMoreParams)?;
                Ok(Command::Ping{token})
            },

            "PONG" => {
                // let mut server: Option<String> = None;
                // if options.len() > 1 {
                //     server = Some(options.pop().ok_or(IRCError::SilentDiscard)?);
                // }
                // let token = options.pop().ok_or(IRCError::SilentDiscard)?;
                options.reverse();
                let token = options.pop().ok_or(IRCError::SilentDiscard)?;
                let server = options.pop();
                Ok(Command::Pong{server, token})
            },

            "OPER" => {
                let name = options.pop().ok_or(IRCError::NeedMoreParams)?;
                let password = options.pop().ok_or(IRCError::NeedMoreParams)?;
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
                let reason = options.pop().ok_or(IRCError::SilentDiscard)?;
                Ok(Command::Error{reason})
            }

            "JOIN" => {
                let channels = options.pop().ok_or(IRCError::SilentDiscard)?;
                let keys = options.pop();

                Ok(Command::Join{channels, keys})
            }

            _ => Err(IRCError::SilentDiscard),
        }
    }

    pub fn get_parts(&self) -> Vec<String> {
        let mut command_parts: Vec<String> = Vec::new();
        match self {
            Command::Pong{server, token} => {
                command_parts.push("PONG".to_string());
                command_parts.push(token.to_string());
                if let Some(server) = server {
                    command_parts.push(server.to_string());
                }
            },
            _ => {}
        }
        return command_parts;
    }
}

impl fmt::Display for Command {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

