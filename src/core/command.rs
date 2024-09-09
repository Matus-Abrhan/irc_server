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

impl TryFrom<&str> for Command{
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
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

