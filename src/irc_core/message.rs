use std::fmt;
use core::str;
use std::io::Cursor;
use bytes::Buf;
use log::debug;

use crate::irc_core::command::Command;
use crate::irc_core::message_errors::IRCError;

#[derive(Debug)]
pub struct Message {
    pub prefix: Option<String>,
    pub command: Command,
}

impl<'a> Message {
    pub fn parse(src: &'a [u8]) -> Result<Option<Message>, IRCError> {
        let has_prefix = src.starts_with(b":");
        let mut msg_parts: Vec<String> = match str::from_utf8(src) {
            Ok(m) => m.trim().split(" ").map(|s| s.to_string()).collect::<Vec<String>>(),
            // Err(_) => return Err(IRCError::SilentDiscard),
            Err(_) => return Ok(None),
        };
        debug!("received: {:?}", msg_parts);
        msg_parts.reverse();

        let prefix: Option<String>;
        if has_prefix {
            match msg_parts.pop() {
                // None => return Err(IRCError::SilentDiscard),
                None => return Ok(None),
                Some(p) => prefix = Some(p),
            }
        } else {
            prefix = None;
        }

        let command: Command = match msg_parts.pop() {
            Some(c) => {
                match Command::parse(&prefix, c, &mut msg_parts) {
                    Ok(command) => command,
                    Err(e) => {
                        match e {
                            IRCError::SilentDiscard => return Ok(None),
                            _ => return Err(e),
                        }
                    },
                }
            },
            // None => return Err(IRCError::SilentDiscard),
            None => return Ok(None),
        };
        Ok(Some(Message { prefix, command}))
    }

    pub fn check(src: &mut Cursor<&'a [u8]>) -> Result<&'a [u8], IRCError> {
        if !src.has_remaining() {
            return Err(IRCError::NoMessageLeftInBuffer)
        }

        return get_message(src);
    }

    pub fn get_parts(&self) -> Vec<String> {
        let mut msg_parts: Vec<String> = Vec::new();
        if let Some(prefix) = &self.prefix {
            msg_parts.push(prefix.to_string());
        }
        msg_parts.append(&mut self.command.get_parts());

        debug!("sent: {:?}", msg_parts);
        return msg_parts;
    }
}

pub fn get_message<'a>(src: &mut Cursor<&'a [u8]>) -> Result<&'a [u8], IRCError> {
    let start = src.position() as usize;
    let end = src.get_ref().len();

    for i in start..end {
        if src.get_ref()[i] == b'\r' && src.get_ref()[i+1] == b'\n' {
            // if (i+1) - start > 512 {
            //     return Err(IRCError::SilentDiscard);
            // }
            src.set_position((i+2)as u64);
            return Ok(&src.get_ref()[start..i]);
        }

        // if src.get_ref()[i] == b'|' {
        //     if i - start > 512 {
        //         return Err(IRCError::SilentDiscard);
        //     }
        //     src.set_position((i+1)as u64);
        //     return Ok(&src.get_ref()[start..i]);
        // }
    }
    // return Err(IRCError::SilentDiscard);
    src.set_position((end+1)as u64);
    return Err(IRCError::NoMessageLeftInBuffer);
}

impl fmt::Display for Message {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
