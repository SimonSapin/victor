#![feature(dropck_parametricity)]

#[macro_use] extern crate matches;
extern crate selectors;
#[macro_use] extern crate string_cache;
extern crate xml as xml_rs;

mod arena;
mod select;

pub use select::SelectorList;

pub mod xml;

pub mod pdf {
    pub mod document_structure;
    mod file_structure;
}

pub mod svg {
    pub mod path;
}


