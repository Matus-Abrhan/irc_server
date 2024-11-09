#[derive(Debug, PartialEq)]
pub struct Channel {
    pub name: String,
    pub members: Vec<String>,
    pub flags: Vec<u8>

}

impl Channel {
    pub fn new(name: String, member: String) -> Channel {
        Channel{
            name,
            members: Vec::from([member]),
            flags: Vec::new(),
        }
    }
}
