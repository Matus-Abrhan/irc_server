#[derive(Debug)]
pub enum Command {
    Pass{password: String, version: String, flags: String, options: Vec<String>},
    Server{servername: String, hopcount: String, token: String, info: String},
    Nick{nickname: String, hopcount: String, username: String, host: String, servertoken: String, umode: String, realname: String},
    Service{servicename: String, servertoken: String, distribution: String, r#type: String, hopcount: String, info: String},
    // QUIT,
    // SQUIT,
    // JOIN,
    // NJOIN,
    // MODE,
}

impl Command {
    pub fn parse(_prefix: &Option<String>, command: String, options: &mut Vec<String>) -> Result<Command, ()> {
        match &command[..] {
            "PASS" => {
                let password = match options.pop() {
                    Some(res) => res,
                    None => return Err(()),
                };
                // TODO: if from user or service only use password
                let version = match options.pop() {
                    Some(res) => res,
                    None => return Err(()),
                };
                let flags = match options.pop() {
                    Some(res) => res,
                    None => return Err(()),
                };
                options.reverse();
                Ok(Command::Pass{
                    password, version, flags, options: options.to_vec()
                })
            },

            "SERVER" => {
                let servername = match options.pop() {
                    Some(res) => res,
                    None => return Err(())
                };
                let hopcount = match options.pop() {
                    Some(res) => res,
                    None => return Err(())
                };
                let token = match options.pop() {
                    Some(res) => res,
                    None => return Err(())
                };
                options.reverse();
                let info = options.join(" ");
                Ok(Command::Server {
                    servername, hopcount, token, info
                })
            },

            "NICK" => {
                let nickname = match options.pop() {
                    Some(res) => res,
                    None => return Err(()),
                };
                let hopcount = match options.pop() {
                    Some(res) => res,
                    None => return Err(()),
                };
                let username = match options.pop() {
                    Some(res) => res,
                    None => return Err(()),
                };
                let host = match options.pop() {
                    Some(res) => res,
                    None => return Err(()),
                };
                let servertoken = match options.pop() {
                    Some(res) => res,
                    None => return Err(()),
                };
                let umode = match options.pop() {
                    Some(res) => res,
                    None => return Err(()),
                };
                let realname = match options.pop() {
                    Some(res) => res,
                    None => return Err(()),
                };
                Ok(Command::Nick {
                    nickname, hopcount, username, host, servertoken, umode, realname
                })
            },

            "SERVICE" => {
                let servicename = match options.pop() {
                    Some(res) => res,
                    None => return Err(()),
                };
                let servertoken = match options.pop() {
                    Some(res) => res,
                    None => return Err(()),
                };
                let distribution = match options.pop() {
                    Some(res) => res,
                    None => return Err(()),
                };
                let r#type = match options.pop() {
                    Some(res) => res,
                    None => return Err(()),
                };
                let hopcount = match options.pop() {
                    Some(res) => res,
                    None => return Err(()),
                };
                options.reverse();
                let info = options.join(" ");
                Ok(Command::Service {
                    servicename, servertoken, distribution, r#type, hopcount, info
                })
            },
            _ => Err(()),
        }
    }
}

#[derive(Debug)]
pub enum ErrReply{
    Needmoreparams = 461,
    Alreadyregistred = 462,
    Passwdmismatch = 463,
}
