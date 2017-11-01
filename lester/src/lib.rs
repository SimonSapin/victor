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
    Cairo(cairo::CairoError),
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Error::Io(e)
    }
}

impl From<cairo::CairoError> for Error {
    fn from(e: cairo::CairoError) -> Self {
        Error::Cairo(e)
    }
}
