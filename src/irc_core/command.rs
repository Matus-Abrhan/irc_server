use std::fmt;

use crate::irc_core::numeric::ErrorReply;

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
    pub fn parse(_prefix: &Option<String>, command: String, options: &mut Vec<String>) -> Result<Option<Command>, ErrorReply> {
        match &command[..] {
            "CAP" => {
                Ok(Some(Command::Cap))
            },

            "PASS" => {
                let password = match options.pop() {
                    Some(res) => res,
                    None => return Ok(None),
                };
                Ok(Some(Command::Pass{password}))
            },

            "NICK" => {
                let nickname = match options.pop() {
                    Some(res) => res,
                    None => return Ok(None),
                };
                Ok(Some(Command::Nick{nickname}))
            },

            "USER" => {
                let user = match options.pop() {
                    Some(res) => res,
                    None => return Ok(None),
                };
                let mode = match options.pop() {
                    Some(res) => res,
                    None => return Ok(None),
                };
                let unused = match options.pop() {
                    Some(res) => res,
                    None => return Ok(None),
                };
                options.reverse();
                Ok(Some(Command::User{user, mode, unused, realname: options.join(" ")}))
            },

            "PING" => {
                let token = options.pop().ok_or(ErrorReply::NeedMoreParams)?;
                Ok(Some(Command::Ping{token}))
            },

            "PONG" => {
                options.reverse();
                let token = match options.pop() {
                    Some(res) => res,
                    None => return Ok(None),
                };
                let server = options.pop();
                Ok(Some(Command::Pong{server, token}))
            },

            "OPER" => {
                let name = options.pop().ok_or(ErrorReply::NeedMoreParams)?;
                let password = options.pop().ok_or(ErrorReply::NeedMoreParams)?;
                Ok(Some(Command::Oper{name, password}))
            },

            "QUIT" => {
                if options.is_empty() {
                    return Ok(Some(Command::Quit {reason: None}));
                }
                options.reverse();
                let reason = options.join(" ");
                Ok(Some(Command::Quit{reason: Some(reason)}))
            },

            "ERROR" => {
                let reason = match options.pop() {
                    Some(res) => res,
                    None => return Ok(None),
                };
                Ok(Some(Command::Error{reason}))
            },

            "JOIN" => {
                let channels = match options.pop() {
                    Some(res) => res,
                    None => return Ok(None),
                };
                let keys = options.pop();

                Ok(Some(Command::Join{channels, keys}))
            },

            "PRIVMSG" => {
                let targets = match options.pop() {
                    Some(res) => res,
                    None => return Ok(None),
                };
                if options.is_empty() {
                    return Err(ErrorReply::NoTextToSend);
                }
                options.reverse();
                let text = options.join(" ");

                Ok(Some(Command::PrivMsg{targets, text}))
            },

            _ => Ok(None),
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
            Command::Join{channels, keys} => {
                command_parts.push("JOIN".to_string());
                command_parts.push(channels.to_string());
                if let Some(keys) = keys {
                    command_parts.push(keys.to_string())
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

