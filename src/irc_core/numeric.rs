#[derive(Debug, Clone, Copy)]
pub enum ErrorReply {
    NoTextToSend = 412,
    NoNicknameGiven = 431,
    ErroneusNickname = 432,
    NicknameInUse = 433,
    NickCollicion = 436,
    NeedMoreParams = 461,
    AlreadyRegistred = 462,
    PasswdMismatch = 463,
}

#[derive(Debug, Clone, Copy)]
pub enum Reply {

}
