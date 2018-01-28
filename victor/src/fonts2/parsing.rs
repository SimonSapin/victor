use euclid;
use fonts::FontError;
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
    start: Position<T>,
    count: u32,
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
    pub(in fonts2) fn cast<U>(self) -> Position<U> {
        Position { byte_position: self.byte_position, ty: PhantomData }
    }

    pub(in fonts2) fn offset<U, O: Into<u32>>(self, by: O) -> Position<U> {
        Position { byte_position: self.byte_position + by.into(), ty: PhantomData }
    }

    pub(in fonts2) fn followed_by<U>(self) -> Position<U> {
        self.offset(mem::size_of::<T>() as u32)
    }

    pub(in fonts2) fn read_from(self, bytes: &[u8]) -> Result<T, FontError> where T: ReadFromBytes {
        T::read_from(bytes.get(self.byte_position as usize..).ok_or(FontError::OffsetBeyondEof)?)
    }
}

pub(in fonts2) trait ReadFromBytes: Sized {
    fn read_from(bytes: &[u8]) -> Result<Self, FontError>;
}

macro_rules! byte_arrays {
    ( $( $size: expr ),+ ) => {
        $(
            impl ReadFromBytes for [u8; $size] {
                fn read_from(bytes: &[u8]) -> Result<Self, FontError> {
                    // This checks the length for the cast below,
                    // but doesn’t change the pointer’s address.
                    let bytes = bytes.get(..$size).ok_or(FontError::OffsetPlusLengthBeyondEof)?;

                    let ptr = bytes.as_ptr() as *const [u8; $size];
                    unsafe {
                        Ok(*ptr)
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
    fn read_from(bytes: &[u8]) -> Result<Self, FontError> {
        Ok(u16::from_be(u16_from_bytes(ReadFromBytes::read_from(bytes)?)))
    }
}

impl ReadFromBytes for u32 {
    fn read_from(bytes: &[u8]) -> Result<Self, FontError> {
        Ok(u32::from_be(u32_from_bytes(ReadFromBytes::read_from(bytes)?)))
    }
}

impl<T, Src, Dst> ReadFromBytes for euclid::TypedScale<T, Src, Dst> where T: ReadFromBytes {
    fn read_from(bytes: &[u8]) -> Result<Self, ::fonts::FontError> {
        ReadFromBytes::read_from(bytes).map(euclid::TypedScale::new)
    }
}

impl Slice<u8> {
    pub(in fonts2) fn read_from<'a>(&self, bytes: &'a [u8]) -> Result<&'a [u8], FontError> {
        bytes.get(self.start.byte_position as usize..).ok_or(FontError::OffsetBeyondEof)?
             .get(..self.count as usize).ok_or(FontError::OffsetPlusLengthBeyondEof)
    }
}

impl<T> Slice<T> {
    pub(in fonts2) fn new<C: Into<u32>>(start: Position<T>, count: C) -> Self {
        Slice { start, count: count.into() }
    }

    pub(in fonts2) fn start(&self) -> Position<T> {
        self.start
    }

    pub(in fonts2) fn followed_by<U>(&self) -> Position<U> {
        self.start.offset(mem::size_of::<T>() as u32 * self.count)
    }

    /// This is not an `unsafe fn` because invalid `Position`s are safe,
    /// they might just panic when reading or return nonsense values.
    pub(in fonts2) fn get_unchecked(&self, index: u32) -> Position<T> {
        self.start.offset(mem::size_of::<T>() as u32 * index)
    }

    #[inline]
    pub(in fonts2) fn binary_search_by_key<V, F>(&self, value: &V, mut f: F)
                                                 -> Result<Option<Position<T>>, FontError>
        where F: FnMut(Position<T>) -> Result<V, FontError>,
              V: Ord
    {
        self.binary_search_by(|index| Ok(f(self.get_unchecked(index))?.cmp(value)))
            .map(|opt| opt.map(|index| self.get_unchecked(index)))
    }

    /// Adapted from https://github.com/rust-lang/rust/blob/1.23.0/src/libcore/slice/mod.rs#L391-L413
    pub(in fonts2) fn binary_search_by<'a, F, E>(&self, mut f: F) -> Result<Option<u32>, E>
        where F: FnMut(u32) -> Result<::std::cmp::Ordering, E>
    {
        use std::cmp::Ordering::*;
        let mut size = self.count;
        if size == 0 {
            return Ok(None);
        }
        let mut base: u32 = 0;
        while size > 1 {
            let half = size / 2;
            let mid = base + half;
            // mid is always in [0, size), that means mid is >= 0 and < size.
            // mid >= 0: by definition
            // mid < size: mid = size / 2 + size / 4 + size / 8 ...
            let cmp = f(mid)?;
            base = if cmp == Greater { base } else { mid };
            size -= half;
        }
        // base is always in [0, size) because base <= mid.
        let cmp = f(base)?;
        if cmp == Equal { Ok(Some(base)) } else { Ok(None) }
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
            end: self.followed_by(),
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

impl<T> DoubleEndedIterator for SliceIter<T> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.start != self.end {
            let next = self.end;
            let byte_position = next.byte_position - mem::size_of::<T>() as u32;
            self.end = Position { byte_position, ty: PhantomData };
            Some(next)
        } else {
            None
        }
    }
}
