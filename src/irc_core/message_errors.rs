#[derive(Debug)]
pub enum IRCError{
    SilentDiscard = -1,
    Incomplete = -2,
    ClientExited = -3,

    NoNicknameGiven = 431,
    ErroneusNickname = 432,
    NicknameInUse = 433,
    NickCollicion = 436,
    NeedMoreParams = 461,
    AlreadyRegistred = 462,
    PasswdMismatch = 463,
}
