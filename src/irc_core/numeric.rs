// #[derive(Debug, Clone, Copy)]
// pub enum ErrorReply {
//     NoTextToSend = 412,
//     NoNicknameGiven = 431,
//     ErroneusNickname = 432,
//     NicknameInUse = 433,
//     NickCollicion = 436,
//     NeedMoreParams = 461,
//     AlreadyRegistred = 462,
//     PasswdMismatch = 463,
// }

#[derive(Debug, Clone)]
pub enum ErrorReplyId {
    IdNoTextToSend = 412,
    IdNoNicknameGiven = 431,
    IdErroneusNickname = 432,
    IdNicknameInUse = 433,
    IdNickCollicion = 436,
    IdNeedMoreParams = 461,
    IdAlreadyRegistred = 462,
    IdPasswdMismatch = 464,
}

#[derive(Debug, Clone)]
pub enum ReplyId {
    IdNameReply = 353,
    IdEndOfNames = 366,
    IdMotd = 372,
    IdMotdStart = 375,
    IdEndOfMotd = 376,
    IdWhoReply = 352,
    IdEndOfWho = 315,
}

#[derive(Debug, Clone)]
pub enum ErrorReply {
    NoTextToSend{client: String},
    NoNicknameGiven{client: String},
    ErroneusNickname{client: String, nick: String},
    NicknameInUse{client: String, nick: String},
    NickCollicion{client: String, nick: String, user: String, host: String},
    NeedMoreParams{client: String, command: String},
    AlreadyRegistred{client: String},
    PasswdMismatch{client: String},
}

#[derive(Debug, Clone)]
pub enum Reply {
    NameReply{client: String, symbol: char, channel: String, members: Vec<String>},
    EndOfNames{client: String, channel: String},
    Motd{client: String, line: String},
    MotdStart{client: String, line: String},
    EndOfMotd{client: String},
    WhoReply{client: String, channel: String, username: String, host: String, server: String, nick: String, flags: String, hopcount: String, realname: String},
    EndOfWho{client: String, mask: String},
}

impl ErrorReply {
    pub fn serialize(&self, ) -> Vec<String> {
        let mut command_parts: Vec<String> = Vec::new();
        match self {
            ErrorReply::NoTextToSend{client} => {
                command_parts.push((ErrorReplyId::IdNoTextToSend as i32).to_string());
                command_parts.push(client.to_string());
                command_parts.push(":No text to send".to_string());
            },

            ErrorReply::NoNicknameGiven{client} => {
                command_parts.push((ErrorReplyId::IdNoNicknameGiven as i32).to_string());
                command_parts.push(client.to_string());
                command_parts.push(":No nickname given".to_string());
            },

            ErrorReply::ErroneusNickname{client, nick} => {
                command_parts.push((ErrorReplyId::IdErroneusNickname as i32).to_string());
                command_parts.push(client.to_string());
                command_parts.push(nick.to_string());
                command_parts.push(":Erroneus nickname".to_string());
            },

            ErrorReply::NicknameInUse{client, nick} => {
                command_parts.push((ErrorReplyId::IdNicknameInUse as i32).to_string());
                command_parts.push(client.to_string());
                command_parts.push(nick.to_string());
                command_parts.push(":Nickname is already in use".to_string());
            },

            ErrorReply::NickCollicion{client, nick, user, host} => {
                command_parts.push((ErrorReplyId::IdNickCollicion as i32).to_string());
                command_parts.push(client.to_string());
                command_parts.push(nick.to_string());
                command_parts.push(":Nickname collision KILL from".to_string());
                command_parts.push(user.to_string()+"@"+host);
            },

            ErrorReply::NeedMoreParams{client, command} => {
                command_parts.push((ErrorReplyId::IdNeedMoreParams as i32).to_string());
                command_parts.push(client.to_string());
                command_parts.push(command.to_string());
                command_parts.push(":Not enough parameters".to_string());
            },

            ErrorReply::AlreadyRegistred{client} => {
                command_parts.push((ErrorReplyId::IdAlreadyRegistred as i32).to_string());
                command_parts.push(client.to_string());
                command_parts.push(":You may not reregister".to_string());
            },

            ErrorReply::PasswdMismatch{client} => {
                command_parts.push((ErrorReplyId::IdPasswdMismatch as i32).to_string());
                command_parts.push(client.to_string());
                command_parts.push(":Password incorrect".to_string());
            },
        };
        return command_parts;
    }
}


impl Reply {
    pub fn serialize(&self) -> Vec<String> {
        let mut command_parts: Vec<String> = Vec::new();
        match self {
            Reply::NameReply{client, symbol, channel, members} => {
                command_parts.push((ReplyId::IdNameReply as i32).to_string());
                command_parts.push(client.to_string());
                command_parts.push(symbol.to_string());
                command_parts.push(channel.to_string());
                command_parts.append(&mut members.clone());
            },

            Reply::EndOfNames{client, channel} => {
                command_parts.push((ReplyId::IdEndOfNames as i32).to_string());
                command_parts.push(client.to_string());
                command_parts.push(channel.to_string());
                command_parts.push(":End of /NAMES list".to_string());
            },

            Reply::Motd{client, line} => {
                command_parts.push((ReplyId::IdMotd as i32).to_string());
                command_parts.push(client.to_string());
                command_parts.push(":".to_owned()+&line);
            },

            Reply::MotdStart{client, line} => {
                command_parts.push((ReplyId::IdMotdStart as i32).to_string());
                command_parts.push(client.to_string());
                command_parts.push(":- ".to_owned()+&line+" - ");
            },

            Reply::EndOfMotd{client} => {
                command_parts.push((ReplyId::IdEndOfMotd as i32).to_string());
                command_parts.push(client.to_string());
                command_parts.push(":End of /MOTD command.".to_string());
            },

            Reply::WhoReply{client, channel, username, host, server, nick, flags, hopcount, realname} => {
                command_parts.push((ReplyId::IdWhoReply as i32).to_string());
                command_parts.push(client.to_string());
                command_parts.push(channel.to_string());
                command_parts.push(username.to_string());
                command_parts.push(host.to_string());
                command_parts.push(server.to_string());
                command_parts.push(nick.to_string());
                command_parts.push(flags.to_string());
                command_parts.push(hopcount.to_string());
                command_parts.push(realname.to_string());
            },

            Reply::EndOfWho{client, mask} => {
                command_parts.push((ReplyId::IdEndOfWho as i32).to_string());
                command_parts.push(client.to_string());
                command_parts.push(mask.to_string());
                command_parts.push(":End of WHO list".to_string());
            },
        };
        return command_parts;
    }
}
