use bitflags::bitflags;

bitflags! {
    pub struct RegistrationFlags: u8 {
        const NONE = 0b00000000;
        const PASS = 0b00000001;
        const NICK = 0b00000010;
        const USER = 0b00000100;
    }
}

pub struct User {
    pub username: String,
    pub nickname: String,
    pub realname: String,
    pub hostname: String,
    pub register_state: RegistrationFlags,
}


impl User {
    pub fn new() -> Self {
        return User{
            username: String::new(),
            nickname: String::new(),
            realname: String::new(),
            hostname: String::new(),
            register_state: RegistrationFlags::NONE,
        }
    }
}
