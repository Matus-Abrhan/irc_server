#[derive(Debug, Clone, Copy)]
pub enum IRCError{
    ClientExited = -1,
    SilentDiscard = -2,
    NoMessageLeftInBuffer = -3,

    NoTextToSend = 412,
    NoNicknameGiven = 431,
    ErroneusNickname = 432,
    NicknameInUse = 433,
    NickCollicion = 436,
    NeedMoreParams = 461,
    AlreadyRegistred = 462,
    PasswdMismatch = 463,
}
