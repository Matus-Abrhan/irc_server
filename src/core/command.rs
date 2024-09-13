#[derive(Debug)]
pub enum Command {
    PASS,
    SERVER,
    NICK,
    // SERVICE,
    // QUIT,
    // SQUIT,
    // JOIN,
    // NJOIN,
    // MODE,
}

impl Command {
    pub fn parse(c: &str) -> Result<Command, ()> {
        match c {
            "PASS" => Ok(Command::PASS),
            "SERVER" => Ok(Command::SERVER),
            "NICK" => Ok(Command::NICK),
            _ => Err(()),
        }
    }
}

#[derive(Debug)]
pub enum ErrReply{
    ErrNeedmoreparams = 461,
    ErrAlreadyregistred = 462,
    ErrPasswdmismatch = 463,
}

