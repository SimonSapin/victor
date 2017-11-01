use std::io;

pub mod cairo;

pub struct Argb32Image<'a> {
    pub width: usize,
    pub height: usize,
    pub pixels: &'a mut [u32],
}

#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    Cairo(cairo::Error),
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Error::Io(e)
    }
}

impl From<cairo::Error> for Error {
    fn from(e: cairo::Error) -> Self {
        Error::Cairo(e)
    }
}
