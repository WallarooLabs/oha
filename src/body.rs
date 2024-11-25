use std::fs;
use std::io;
use std::path::Path;

use http_body_util::Full;

#[derive(Debug)]
pub(crate) enum Body {
    Empty,
    Raw(&'static [u8]),
}

impl Body {
    pub(crate) fn empty() -> Self {
        Self::Empty
    }

    pub(crate) fn from_string(body: String) -> Self {
        let body = Box::leak(body.into_boxed_str().into_boxed_bytes());
        Self::Raw(body)
    }

    pub(crate) fn from_file(path: impl AsRef<Path>) -> io::Result<Self> {
        let buf = fs::read(path)?;
        let body = Box::leak(buf.into_boxed_slice());
        Ok(Self::Raw(body))
    }

    pub(crate) fn next_body(&self) -> Full<&'static [u8]> {
        match self {
            Self::Empty => Full::default(),
            Self::Raw(body) => Full::new(body),
        }
    }
}
