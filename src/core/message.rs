use super::command::Command;


#[derive(Debug)]
pub struct Message {
    pub prefix: Option<String>,
    pub command: Command,
    pub params: Vec<String>,
}

impl TryFrom<&str> for Message {
    type Error = ();

    fn try_from(msg: &str) -> Result<Self, Self::Error> {
        let has_prefix: bool = msg.starts_with(":");
        let mut msg_parts: Vec<String> = msg.split(" ").map(|s| s.to_string()).collect();
        msg_parts.reverse();

        let prefix: Option<String>;
        if has_prefix {
            prefix = match msg_parts.pop() {
                None => return Result::Err(()),
                Some(p) => Some(p),
            }
        } else {
            prefix = None
        }

        let command: Command = match msg_parts.pop() {
            None => return Result::Err(()),
            Some(c) => {
                match Command::try_from(&c[..]) {
                    Ok(command) => command,
                    Err(_) => return Result::Err(()),
                }
            },
        };

        if msg_parts.len() > 15 {
            return Err(())
        }
        let mut params: Vec<String> = Vec::new();
        for part in msg_parts {
            params.insert(0, part);
        }
        return Ok(Message { prefix, command, params });
    }
}
