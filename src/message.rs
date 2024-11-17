use core::str;
use std::io::Cursor;
use bytes::Buf;

use irc_proto::message::{Message, Content};

use crate::error::IRCError;

pub fn create_message<'a>(src: &'a [u8]) -> Option<Message> {
    let mut msg_iter = match str::from_utf8(src) {
        Ok(m) => m.trim().split(" ").map(|s| s.to_string()).collect::<Vec<String>>().into_iter(),
        Err(_) => return None,
    };

    let mut prefix: Option<String> = None;
    if src.starts_with(b":") {
        match msg_iter.next() {
            None => return None,
            p => prefix = p,
        }
    }

    let command_numeric = match msg_iter.next() {
        None => return None,
        Some(cn) => cn,
    };

    let content = match Content::new(command_numeric.as_str(), msg_iter.collect()) {
        Content::Unknown() => return None,
        content => content,
    };

    Some(Message{prefix, content})
}

fn get_message<'a>(src: &mut Cursor<&'a [u8]>) -> Result<&'a [u8], IRCError> {
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

pub fn check_message<'a>(src: &mut Cursor<&'a [u8]>) -> Result<&'a [u8], IRCError> {
    if !src.has_remaining() {
        return Err(IRCError::NoMessageLeftInBuffer)
    }

    return get_message(src);
}

