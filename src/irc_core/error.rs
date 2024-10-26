#[derive(Debug, Clone, Copy)]
pub enum IRCError {
    ClientExited = -1,
    // SilentDiscard = -2,
    NoMessageLeftInBuffer = -3,
}
