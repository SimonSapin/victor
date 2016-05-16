#![feature(dropck_parametricity)]

#[macro_use] extern crate matches;
extern crate selectors;
#[macro_use] extern crate string_cache;
extern crate xml as xml_rs;

mod select;

pub use select::SelectorList;

pub mod xml;

pub mod pdf {
    pub mod document_structure;
    mod file_structure;
}

pub mod svg {
    pub mod elliptical_arc;
    pub mod geometry;
    pub mod path;
    // Despite simple_path.rs being next to path.rs, it is its sub-module: svg::path::simple_path.
}
