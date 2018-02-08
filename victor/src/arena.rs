use std::cell::{Cell, RefCell};
use std::mem;
use std::ptr;
use std::slice;

pub struct Arena<T> {
    full_chunks: RefCell<Vec<Box<[T]>>>,
    current_chunk: PartiallyFullChunk,
}

/// In number of items
const INITIAL_CHUNK_LENGTH: usize = 4;

// Not parameterizing over `T` so that we can both implement Drop
// and allow T values in the same arena to have `&'arena T` reference cycles.
// Rust’s drop checking normally forbids that unless we use the `#[may_dangle]` attribute,
// but that attribute is not stable yet: https://github.com/rust-lang/rust/issues/34761
//
// We effectively own some `T` values without telling the compiler about it.
struct PartiallyFullChunk {
    start: Cell<*mut u8>,
    next: Cell<*mut u8>,
    end: Cell<*mut u8>,
    // Storing a function pointer allows the non-generic Drop impl to call generic code,
    // such as `Vec::<T>::from_raw_parts` and `Vec::<T>::drop`
    drop: fn(start: *mut u8, length_bytes: usize, capacity_bytes: usize),
}

impl<T> Arena<T> {
    pub fn new() -> Self {
        assert!(mem::size_of::<T>() != 0, "this arena cannot be used with zero-sized types");
        Arena {
            full_chunks: RefCell::new(Vec::new()),
            current_chunk: PartiallyFullChunk {
                // An empty arena doesn’t allocate
                start: Cell::new(ptr::null_mut()),
                next: Cell::new(ptr::null_mut()),
                end: Cell::new(ptr::null_mut()),
                drop: drop_partially_full_chunk::<T>,
            }
        }
    }

    pub fn allocate(&self, item: T) -> &T {
        if self.current_chunk.next.get() == self.current_chunk.end.get() {
            self.new_chunk()
        }
        let next = self.current_chunk.next.get() as *mut T;
        unsafe {
            ptr::write(next, item);
            self.current_chunk.next.set(next.offset(1) as *mut u8);
            &*next
        }
    }

    #[inline(never)]
    #[cold]
    fn new_chunk(&self) {
        let start = self.current_chunk.start.get();
        let end = self.current_chunk.end.get();
        let new_capacity;
        // start and end are both NULL in empty arenas
        if start != end {
            // `Arena::new()` panics in `assert!` if this would divide by zero
            let len = ((end as usize) - (start as usize)) / mem::size_of::<T>();
            let full_chunk = unsafe {
                Box::from_raw(slice::from_raw_parts_mut(start as *mut T, len))
            };
            self.full_chunks.borrow_mut().push(full_chunk);
            new_capacity = len * 2
        } else {
            new_capacity = INITIAL_CHUNK_LENGTH
        }

        let mut vec = Vec::<T>::with_capacity(new_capacity);
        let start = vec.as_mut_ptr();
        mem::forget(vec);

        let end = unsafe {
            start.offset(new_capacity as isize)
        };
        self.current_chunk.start.set(start as *mut u8);
        self.current_chunk.next.set(start as *mut u8);
        self.current_chunk.end.set(end as *mut u8);
    }
}

impl Drop for PartiallyFullChunk {
    fn drop(&mut self) {
        let start = self.start.get();
        let length_bytes = (self.next.get() as usize) - (start as usize);
        let capacity_bytes = (self.end.get() as usize) - (start as usize);
        (self.drop)(start, length_bytes, capacity_bytes)
    }
}

fn drop_partially_full_chunk<T>(start: *mut u8, length_bytes: usize, capacity_bytes: usize) {
    // `Arena::new()` panics in `assert!` if this would divide by zero
    let length = length_bytes / mem::size_of::<T>();
    let capacity = capacity_bytes / mem::size_of::<T>();
    unsafe {
        drop(Vec::<T>::from_raw_parts(start as *mut T, length, capacity))
    }
}

#[test]
fn track_drop() {
    #[derive(PartialEq, Debug)]
    struct AssertDropOrder<'a> {
        drop_counter: &'a Cell<u32>,
        value: u32,
    }

    impl<'a> Drop for AssertDropOrder<'a> {
        fn drop(&mut self) {
            let value = self.drop_counter.get();
            if !::std::thread::panicking() {
                assert_eq!(value, self.value)
            }
            self.drop_counter.set(value + 1);
        }
    }

    #[derive(PartialEq, Debug)]
    struct Node<'a, 'b: 'a> {
        next: Option<&'a Node<'a, 'b>>,
        drop: AssertDropOrder<'b>,
    }

    let drop_counter = Cell::new(0);
    let drop_counter = &drop_counter;

    {
        let arena = Arena::new();
        let new = |value, next| Node { next, drop: AssertDropOrder { value, drop_counter } };

        let mut node = arena.allocate(new(0, None));
        node = arena.allocate(new(1, Some(node)));
        node = arena.allocate(new(2, Some(node)));
        node = arena.allocate(new(3, Some(node)));
        assert_eq!(arena.full_chunks.borrow().len(), 0);

        node = arena.allocate(new(4, Some(node)));
        assert_eq!(arena.full_chunks.borrow().len(), 1);  // assumes INITIAL_CHUNK_LENGTH == 4

        assert_eq!(node.drop.value, 4);
        assert_eq!(node.next.unwrap().drop.value, 3);
        assert_eq!(node.next.unwrap().next.unwrap().drop.value, 2);
        assert_eq!(node.next.unwrap().next.unwrap().next.unwrap().drop.value, 1);
        assert_eq!(node.next.unwrap().next.unwrap().next.unwrap().next.unwrap().drop.value, 0);
        assert_eq!(node.next.unwrap().next.unwrap().next.unwrap().next.unwrap().next, None);

        drop(node);

        // Nodes are now all unreachable, but not dropped/deallocated yet
        assert_eq!(drop_counter.get(), 0);
    }

    assert_eq!(drop_counter.get(), 5);
}

#[test]
fn cycle() {
    struct Node<'a>(Cell<Option<&'a Node<'a>>>, Box<u32>);

    let arena = Arena::new();
    let a = arena.allocate(Node(Cell::new(None), Box::new(42)));
    let b = arena.allocate(Node(Cell::new(None), Box::new(7)));
    a.0.set(Some(b));
    b.0.set(Some(a));
    let mut nums = Vec::new();
    let mut node = &*a;
    for _ in 0..10 {
        nums.push(*node.1);
        node = node.0.get().unwrap();
    }
    assert_eq!(nums, [42, 7, 42, 7, 42, 7, 42, 7, 42, 7])
}

#[test]
fn dropck() {
    struct Foo<'a>(&'a String);

    // Uncommenting this should fail to borrow/drop-check:
//    impl<'a> Drop for Foo<'a> {
//        fn drop(&mut self) {
//            assert_eq!(self.0, "alive")
//        }
//    }

    let (y, x);
    x = "alive".to_string();
    y = Arena::new();
    y.allocate(Foo(&x));
}

#[test]
fn size_of() {
    assert_eq!(mem::size_of::<Arena<()>>(), 8 * mem::size_of::<usize>())
}
