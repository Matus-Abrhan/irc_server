use std::fmt;
use core::str;
use std::io::Cursor;
use bytes::Buf;
use log::debug;

use crate::irc_core::command::Command;
use crate::irc_core::numeric::{Reply, ErrorReply};
use crate::irc_core::error::IRCError;

#[derive(Debug)]
pub struct Message {
    pub prefix: Option<String>,
    // pub content: Command,
    pub content: Content,
}

#[derive(Debug)]
pub enum Content {
    Command(Command),
    Reply(Reply),
    ErrorReply(ErrorReply),
}

impl<'a> Message {
    pub fn serialize(src: &'a [u8]) -> Option<Message> {
        let has_prefix = src.starts_with(b":");
        let mut msg_parts: Vec<String> = match str::from_utf8(src) {
            Ok(m) => m.trim().split(" ").map(|s| s.to_string()).collect::<Vec<String>>(),
            Err(_) => return None,
        };
        debug!("received: {:?}", msg_parts);
        msg_parts.reverse();

        let prefix: Option<String>;
        if has_prefix {
            match msg_parts.pop() {
                None => return None,
                Some(p) => prefix = Some(p),
            }
        } else {
            prefix = None;
        }

        let content: Content = match msg_parts.pop() {
            Some(c) => {
                match Command::deserialize(&prefix, c, &mut msg_parts) {
                    Ok(Some(command)) => Content::Command(command),
                    Ok(None) => return None,
                    Err(error) => Content::ErrorReply(error),
                }
            },
            None => return None,
        };
        Some(Message{prefix, content})
    }

    pub fn check(src: &mut Cursor<&'a [u8]>) -> Result<&'a [u8], IRCError> {
        if !src.has_remaining() {
            return Err(IRCError::NoMessageLeftInBuffer)
        }

        return get_message(src);
    }

    pub fn deserialize(&self) -> Vec<String> {
        let mut msg_parts: Vec<String> = Vec::new();
        if let Some(prefix) = &self.prefix {
            msg_parts.push(":".to_owned()+&prefix.to_string());
        }
        msg_parts.append(&mut self.content.get_parts());

        debug!("sent: {:?}", msg_parts);
        return msg_parts;
    }
}

impl Content {
    fn get_parts(&self) -> Vec<String> {
        return match self {
            Content::Command(content) => content.serialize(),
            Content::Reply(content) => content.serialize(),
            Content::ErrorReply(content) => content.serialize(),
        };
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
    }
    src.set_position((end+1)as u64);
    return Err(IRCError::NoMessageLeftInBuffer);
}

impl fmt::Display for Message {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
