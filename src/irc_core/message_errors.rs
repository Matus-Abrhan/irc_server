#[derive(Debug)]
pub enum ErrReply{
    SilentDiscard = 0,
    Needmoreparams = 461,
    Alreadyregistred = 462,
    Passwdmismatch = 463,
}
