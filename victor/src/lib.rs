pub extern crate euclid;
#[macro_use] extern crate lopdf;
extern crate parking_lot_core;
#[macro_use] extern crate victor_internal_derive;

pub mod display_lists;
pub mod fonts;

mod pdf;
mod raw_mutex;
mod write;

#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    Victor(fonts::FontError),
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Io(e)
    }
}

impl From<fonts::FontError> for Error {
    fn from(e: fonts::FontError) -> Self {
        Error::Victor(e)
    }
}
