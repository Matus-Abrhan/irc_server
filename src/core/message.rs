use core::str;
use std::io::Cursor;

use bytes::Buf;

use super::command::Command;

#[derive(Debug)]
pub struct Message {
    pub prefix: Option<String>,
    pub command: Command,
    pub params: Vec<String>,
}

#[derive(Debug)]
pub enum Error {
    Incomplete,
    LengthExceeded,
    Generic,
    Invalid,
}

impl Message {
    pub fn parse(src: &mut Cursor<&[u8]>) -> Result<Message, Error> {
        let msg = get_message(src)?;
        let has_prefix = msg.starts_with(b":");
        let mut msg_parts: Vec<String> = match str::from_utf8(msg) {
            Ok(m) => m.trim().split(" ").map(|s| s.to_string()).collect::<Vec<String>>(),
            Err(_) => return Err(Error::Generic),
        };
        msg_parts.reverse();

        let prefix: Option<String>;
        if has_prefix {
            match msg_parts.pop() {
                None => return Err(Error::Incomplete),
                Some(p) => prefix = Some(p),
            }
        } else {
            prefix = None;
        }

        let command: Command = match msg_parts.pop() {
            Some(c) => {
                match Command::parse(&c[..]) {
                    Ok(command) => command,
                    Err(_) => return Err(Error::Invalid),
                }
            },
            None => return Err(Error::Invalid),
        };

        if msg_parts.len() > 15 {
            return Err(Error::Invalid);
        }
        msg_parts.reverse();

        Ok(Message { prefix, command, params: msg_parts })
    }

    pub fn check(src: &mut Cursor<&[u8]>) -> Result<(), Error> {
        if !src.has_remaining() {
            return Err(Error::Incomplete)
        }
        Ok(())
    }
}

pub fn get_message<'a>(src: &mut Cursor<&'a [u8]>) -> Result<&'a [u8], Error> {
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
                return Err(Error::LengthExceeded);
            }
            src.set_position((i+1)as u64);
            return Ok(&src.get_ref()[start..i]);
        }
    }

    Err(Error::Incomplete)
}
