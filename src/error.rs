#[derive(Debug, Clone, Copy)]
pub enum IRCError {
    ClientExited = -1,
    NoMessageLeftInBuffer = -2,
    LengthExceeded = -3,
}
