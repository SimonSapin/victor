pub use euclid;

pub mod dom;
pub mod fonts;
pub mod pdf;
pub mod primitives;
pub mod text;
pub mod text_plain;

#[macro_use]
mod tagged_union_with_jump_tables;
mod geom;
mod layout;
mod paint;
mod style;

#[macro_use]
extern crate cssparser;

#[macro_use]
extern crate html5ever;

#[macro_use]
extern crate matches;

#[macro_use]
extern crate victor_internal_proc_macros;

/*

## Specifications

PDF:
    https://www.adobe.com/devnet/pdf/pdf_reference.html
    https://www.adobe.com/content/dam/acom/en/devnet/pdf/PDF32000_2008.pdf

TrueType:
    https://developer.apple.com/fonts/TrueType-Reference-Manual/RM06/Chap6.html

OpenType (including TrueType):
    https://www.microsoft.com/typography/otspec/

PNG:
    https://www.w3.org/TR/2003/REC-PNG-20031110/

JPEG:
    https://www.w3.org/Graphics/JPEG/


## Font libraries

https://github.com/devongovett/fontkit
https://github.com/fonttools/fonttools
https://github.com/bodoni/opentype + https://github.com/bodoni/truetype

*/
