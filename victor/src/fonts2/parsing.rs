use fonts2::tables::OffsetSubtable;
use std::marker::PhantomData;
use std::mem;

/// The position of some piece of data within a font file,
/// in bytes from the start of the file.
///
/// The type parameter indicates what data is expected to be found there.
pub(in fonts2) struct Position<T> {
    byte_position: u32,
    ty: PhantomData<T>
}

/// The position and length of a consecutive sequence of homogeneous data in a font file
/// This is similar to `&[T]` in the same way that `Position<T>` is similar to `&T`.
pub(in fonts2) struct Slice<T> {
    pub(in fonts2) start: Position<T>,
    pub(in fonts2) count: u32,
}

/// An iterator for `Slice<T>`.
pub(in fonts2) struct SliceIter<T> {
    start: Position<T>,
    end: Position<T>,
}

impl Position<OffsetSubtable> {
    pub(in fonts2) fn initial() -> Self {
        Position { byte_position: 0, ty: PhantomData }
    }
}

impl<T> Position<T> {
    pub(in fonts2) fn offset<U>(self, by: u32) -> Position<U> {
        Position { byte_position: self.byte_position + by, ty: PhantomData }
    }

    pub(in fonts2) fn followed_by<U>(self) -> Position<U> {
        self.offset(mem::size_of::<T>() as u32)
    }

    pub(in fonts2) fn read_from(self, bytes: &[u8]) -> T where T: ReadFromBytes {
        T::read_from(&bytes[self.byte_position as usize..])
    }
}

pub(in fonts2) trait ReadFromBytes {
    fn read_from(bytes: &[u8]) -> Self;
}

macro_rules! byte_arrays {
    ( $( $size: expr ),+ ) => {
        $(
            impl ReadFromBytes for [u8; $size] {
                fn read_from(bytes: &[u8]) -> Self {
                    // `bytes[..$size].as_ptr()` returns the same pointer as `bytes.as_ptr()`,
                    // but also asserts that bytes.len() is large enough.
                    let ptr = bytes[..$size].as_ptr() as *const [u8; $size];

                    unsafe {
                        *ptr
                    }
                }
            }
        )+
    }
}

byte_arrays!(2, 4);

fn u16_from_bytes(bytes: [u8; 2]) -> u16 {
    unsafe { mem::transmute(bytes) }
}

fn u32_from_bytes(bytes: [u8; 4]) -> u32 {
    unsafe { mem::transmute(bytes) }
}

impl ReadFromBytes for u16 {
    fn read_from(bytes: &[u8]) -> Self {
        u16::from_be(u16_from_bytes(ReadFromBytes::read_from(bytes)))
    }
}

impl ReadFromBytes for u32 {
    fn read_from(bytes: &[u8]) -> Self {
        u32::from_be(u32_from_bytes(ReadFromBytes::read_from(bytes)))
    }
}

// ~~~~ Boring trait impls ~~~~

impl<T> Copy for Position<T> {}

impl<T> Clone for Position<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> PartialEq for Position<T> {
    fn eq(&self, other: &Self) -> bool {
        self.byte_position == other.byte_position
    }
}

impl<T> Copy for Slice<T> {}

impl<T> Clone for Slice<T> {
    fn clone(&self) -> Self { *self }
}

impl<T> IntoIterator for Slice<T> {
    type Item = Position<T>;
    type IntoIter = SliceIter<T>;

    fn into_iter(self) -> SliceIter<T> {
        SliceIter {
            start: self.start,
            end: self.start.offset(mem::size_of::<T>() as u32 * self.count),
        }
    }
}

impl<T> Iterator for SliceIter<T> {
    type Item = Position<T>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.start != self.end {
            let next = self.start;
            self.start = next.followed_by();
            Some(next)
        } else {
            None
        }
    }
}
