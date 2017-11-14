pub extern crate euclid;
pub mod fonts;
pub mod pdf;
pub mod primitives;

extern crate dtoa;
extern crate itoa;
extern crate parking_lot_core;
#[macro_use] extern crate victor_internal_derive;

mod raw_mutex;

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
