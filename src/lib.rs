#![feature(dropck_parametricity)]

#[macro_use] extern crate matches;
extern crate selectors;
#[macro_use] extern crate string_cache;
extern crate xml as xml_rs;

mod arena;
mod select;
pub mod svg {
    pub mod path;
}
pub mod xml;

pub use select::SelectorList;
