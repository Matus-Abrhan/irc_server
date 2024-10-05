use std::fmt;
use core::str;
use std::io::Cursor;
use bytes::Buf;

use crate::irc_core::command::Command;
use crate::irc_core::message_errors::IRCError;

#[derive(Debug)]
pub struct Message {
    pub prefix: Option<String>,
    pub command: Command,
}

impl Message {
    pub fn parse(src: &mut Cursor<&[u8]>) -> Result<Message, IRCError> {
        let msg = get_message(src)?;
        let has_prefix = msg.starts_with(b":");
        let mut msg_parts: Vec<String> = match str::from_utf8(msg) {
            Ok(m) => m.trim().split(" ").map(|s| s.to_string()).collect::<Vec<String>>(),
            Err(_) => return Err(IRCError::SilentDiscard),
        };
        msg_parts.reverse();

        let prefix: Option<String>;
        if has_prefix {
            match msg_parts.pop() {
                None => return Err(IRCError::SilentDiscard),
                Some(p) => prefix = Some(p),
            }
        } else {
            prefix = None;
        }

        let command: Command = match msg_parts.pop() {
            Some(c) => {
                match Command::parse(&prefix, c, &mut msg_parts) {
                    Ok(command) => command,
                    Err(e) => return Err(e),
                }
            },
            None => return Err(IRCError::SilentDiscard),
        };

        Ok(Message { prefix, command})
    }

    pub fn check(src: &mut Cursor<&[u8]>) -> Result<(), IRCError> {
        if !src.has_remaining() {
            return Err(IRCError::SilentDiscard)
        }
        Ok(())
    }
}

pub fn get_message<'a>(src: &mut Cursor<&'a [u8]>) -> Result<&'a [u8], IRCError> {
    let start = src.position() as usize;
    let end = src.get_ref().len();

    for i in start..end {
        // if src.get_ref()[i] == b'\r' && src.get_ref()[i+i] == b'\n' {
        //     if (i+1) - start > 512 {
        //         return Err(Error::LengthExceeded);
        //     }
        //     src.set_position((i+2)as u64);
        //     return Ok(&src.get_ref()[start..i+1]);
        // }

        if src.get_ref()[i] == b'\n' {
            if i - start > 512 {
                return Err(IRCError::SilentDiscard);
            }
            src.set_position((i+1)as u64);
            return Ok(&src.get_ref()[start..i]);
        }
    }

    Err(IRCError::Incomplete)
}

impl fmt::Display for Message {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
