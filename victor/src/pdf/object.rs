use std::io::{self, Write};
use super::syntax::IndirectObjectId;

#[derive(Debug)]
pub(crate) enum Object<'a> {
    Usize(usize),
    I32(i32),
    Float(f32),
    Name(&'a [u8]),
    LiteralString(&'a [u8]),
    HexString(&'a [u8]),
    Array(&'a [Object<'a>]),
    Dictionary(Dictionary<'a>),
    Reference(IndirectObjectId),

    GraphicsStateDictionaryAlpha(f32),
    DictionaryWithOwnedKeys(&'a [(Vec<u8>, Object<'a>)])
}

fn _static_assert_size() {
    let _ = ::std::mem::transmute::<Object<'static>, [u8; 32]>;
}

pub(crate) type KeyValuePairs<'a> = &'a [(&'a [u8], Object<'a>)];

#[derive(Debug)]
pub(crate) struct Dictionary<'a> {
    pub prev: Option<&'a Dictionary<'a>>,
    pub pairs: KeyValuePairs<'a>,
}

macro_rules! array {
    ($( $value: expr ),* ,) => {
        array![ $( $value ),* ]
    };
    ($( $value: expr ),*) => {
        &[ $( ::pdf::object::Object::from($value) ),* ][..]
    }
}

macro_rules! key_value_pairs {
    ($( $key: expr => $value: expr ),+ ,) => {
        key_value_pairs!( $($key => $value),+ )
    };
    ($( $key: expr => $value: expr ),*) => {
        &[
            $(
                (AsRef::<[u8]>::as_ref($key), ::pdf::object::Object::from($value)),
            )*
        ]
    };
}

macro_rules! dictionary {
    ($($pairs: tt)*) => {
        Dictionary {
            prev: None,
            pairs: key_value_pairs!($($pairs)*),
        }
    }
}

macro_rules! linked_dictionary {
    ($prev: expr, $($pairs: tt)*) => {
        Dictionary {
            prev: Some(::std::borrow::Borrow::borrow($prev)),
            pairs: key_value_pairs!($($pairs)*),
        }
    }
}

impl<'a, T: Copy> From<&'a T> for Object<'a> where Object<'a>: From<T> {
    fn from(value: &'a T) -> Self {
        Object::from(*value)
    }
}

impl<'a> From<i32> for Object<'a> {
    fn from(value: i32) -> Self {
        Object::I32(value)
    }
}

impl<'a> From<usize> for Object<'a> {
    fn from(value: usize) -> Self {
        Object::Usize(value)
    }
}

impl<'a> From<f32> for Object<'a> {
    fn from(value: f32) -> Self {
        Object::Float(value)
    }
}

impl<'a> From<&'a str> for Object<'a> {
    fn from(name: &'a str) -> Self {
        Object::Name(name.as_bytes())
    }
}

impl<'a> From<&'a String> for Object<'a> {
    fn from(value: &'a String) -> Self {
        Object::Name(value.as_bytes())
    }
}

impl<'a> From<&'a [Object<'a>]> for Object<'a> {
    fn from(value: &'a [Object]) -> Self {
        Object::Array(value)
    }
}

impl<'a> From<KeyValuePairs<'a>> for Object<'a> {
    fn from(value: KeyValuePairs<'a>) -> Self {
        Object::Dictionary(Dictionary {
            prev: None,
            pairs: value,
        })
    }
}

impl<'a> From<Dictionary<'a>> for Object<'a> {
    fn from(value: Dictionary<'a>) -> Self {
        Object::Dictionary(value)
    }
}

impl<'a> From<IndirectObjectId> for Object<'a> {
    fn from(value: IndirectObjectId) -> Self {
        Object::Reference(value)
    }
}

impl<'a> Object<'a> {
    pub fn write<W: Write>(&self, w: &mut W) -> io::Result<()> {
        match *self {
            // https://www.adobe.com/content/dam/acom/en/devnet/pdf/PDF32000_2008.pdf#G6.1965566
            Object::I32(value) => ::itoa::write(w, value).map(|_| ()),
            Object::Usize(value) => ::itoa::write(w, value).map(|_| ()),
            Object::Float(value) => ::dtoa::write(w, value).map(|_| ()),
            Object::Name(value) => write_name(value, w),
            Object::Dictionary(ref value) => value.write(w),
            Object::LiteralString(value) => {
                w.write_all(b"(")?;
                for &byte in value {
                    match byte {
                        b'\\' | b'(' | b')' => w.write_all(&[b'\\', byte])?,
                        _ => w.write_all(&[byte])?,
                    }
                }
                w.write_all(b")")
            }
            Object::HexString(value) => {
                w.write_all(b"<")?;
                for &byte in value {
                    write_hex(byte, w)?
                }
                w.write_all(b">")
            }
            Object::Array(value) => {
                w.write_all(b"[")?;
                let mut iter = value.iter();
                if let Some(item) = iter.next() {
                    item.write(w)?;
                    for item in iter {
                        w.write_all(b" ")?;
                        item.write(w)?
                    }
                }
                w.write_all(b"]")
            }
            Object::Reference(IndirectObjectId(id)) => {
                ::itoa::write(&mut *w, id)?;
                w.write_all(b" 0 R")
            }
            Object::GraphicsStateDictionaryAlpha(value) => {
                let dict = dictionary! {
                    "CA" => value,
                    "ca" => value,
                };
                dict.write(w)
            }
            Object::DictionaryWithOwnedKeys(value) => {
                w.write_all(b"<<")?;
                for &(ref key, ref value) in value {
                    w.write_all(b" ")?;
                    write_name(key, w)?;
                    w.write_all(b" ")?;
                    value.write(w)?
                }
                w.write_all(b" >>")
            }
        }
    }
}

impl<'a> Dictionary<'a> {
    pub fn write<W: Write>(&self, w: &mut W) -> io::Result<()> {
        w.write_all(b"<<")?;
        self.write_pairs(w)?;
        w.write_all(b" >>")
    }

    pub fn write_pairs<W: Write>(&self, w: &mut W) -> io::Result<()> {
        if let Some(prev) = self.prev {
            prev.write_pairs(w)?
        }
        for &(key, ref value) in self.pairs {
            w.write_all(b" ")?;
            write_name(key, w)?;
            w.write_all(b" ")?;
            value.write(w)?
        }
        Ok(())
    }
}

fn write_hex<W: Write>(byte: u8, w: &mut W) -> io::Result<()> {
    const HEX_DIGITS: [u8; 16] = *b"0123456789ABCDEF";
    w.write_all(&[
        HEX_DIGITS[(byte >> 4) as usize],
        HEX_DIGITS[(byte & 0x0F) as usize],
    ])
}

fn write_name<W: Write>(name: &[u8], w: &mut W) -> io::Result<()> {
    w.write_all(b"/")?;
    for &byte in name {
        match KIND[byte as usize] {
            CharKind::Regular => w.write_all(&[byte])?,
            CharKind::Whitespace |
            CharKind::Delimiter => {
                w.write_all(b"#")?;
                write_hex(byte, w)?
            }
        }
    }
    Ok(())
}

// https://www.adobe.com/content/dam/acom/en/devnet/pdf/PDF32000_2008.pdf#G6.1839343
#[repr(u8)]
enum CharKind {
    Whitespace,
    Delimiter,
    Regular,
}

/*
kind = ['r'] * 256
for byte in [0, 0x09, 0x0A, 0x0C, 0x0D, 0x20]:
    kind[byte] = 'W'
for char in "()<>[]{}/%":
    kind[ord(char)] = 'D'

for line in range(32):
    print '       ',
    for column in range(8):
        byte = column + 8 * line
        print kind[byte] + ',',
    print '//',
    for column in range(8):
        byte = column + 8 * line
        print repr(chr(byte))[1:-1].replace("\\\\", "\\"),
    print
*/
static KIND: [CharKind; 256] = {
    use self::CharKind::{Whitespace as W, Delimiter as D, Regular as r};
    [
        W, r, r, r, r, r, r, r, // \x00 …
        r, W, W, r, W, W, r, r, // \x08 \t \n \x0b \x0c \r \x0e \x0f
        r, r, r, r, r, r, r, r, // \x10 …
        r, r, r, r, r, r, r, r, // \x18 …
        W, r, r, r, r, D, r, r, //   ! " # $ % & '
        D, D, r, r, r, r, r, D, // ( ) * + , - . /
        r, r, r, r, r, r, r, r, // 0 1 2 3 4 5 6 7
        r, r, r, r, D, r, D, r, // 8 9 : ; < = > ?
        r, r, r, r, r, r, r, r, // @ A B C D E F G
        r, r, r, r, r, r, r, r, // H I J K L M N O
        r, r, r, r, r, r, r, r, // P Q R S T U V W
        r, r, r, D, r, D, r, r, // X Y Z [ \ ] ^ _
        r, r, r, r, r, r, r, r, // ` a b c d e f g
        r, r, r, r, r, r, r, r, // h i j k l m n o
        r, r, r, r, r, r, r, r, // p q r s t u v w
        r, r, r, D, r, D, r, r, // x y z { | } ~ \x7f
        r, r, r, r, r, r, r, r, // \x80 …
        r, r, r, r, r, r, r, r, // \x88 …
        r, r, r, r, r, r, r, r, // \x90 …
        r, r, r, r, r, r, r, r, // \x98 …
        r, r, r, r, r, r, r, r, // \xa0 …
        r, r, r, r, r, r, r, r, // \xa8 …
        r, r, r, r, r, r, r, r, // \xb0 …
        r, r, r, r, r, r, r, r, // \xb8 …
        r, r, r, r, r, r, r, r, // \xc0 …
        r, r, r, r, r, r, r, r, // \xc8 …
        r, r, r, r, r, r, r, r, // \xd0 …
        r, r, r, r, r, r, r, r, // \xd8 …
        r, r, r, r, r, r, r, r, // \xe0 …
        r, r, r, r, r, r, r, r, // \xe8 …
        r, r, r, r, r, r, r, r, // \xf0 …
        r, r, r, r, r, r, r, r, // \xf8 … \xff
    ]
};
