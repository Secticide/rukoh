use std::fmt;

#[derive(Debug)]
pub enum Error {
    Windows(windows::core::Error),
    Image(String),
    Font(String),
    Audio(String),
    InvalidState(&'static str),
}

impl From<windows::core::Error> for Error {
    fn from(e: windows::core::Error) -> Self {
        Self::Windows(e)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Windows(e) => write!(f, "{e}"),
            Self::Image(e) => write!(f, "image error: {e}"),
            Self::Font(e) => write!(f, "font error: {e}"),
            Self::Audio(e) => write!(f, "audio error: {e}"),
            Self::InvalidState(e) => write!(f, "invalid state: {e}"),
        }
    }
}

impl std::error::Error for Error {}
