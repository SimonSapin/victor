use std::cell::{Cell, RefCell};
use std::mem;
use std::ptr;
use std::slice;

pub struct Arena<T> {
    current_block_start: Cell<*mut T>,
    current_block_next: Cell<*mut T>,
    current_block_end: Cell<*mut T>,
    previous_blocks: RefCell<Vec<Box<[T]>>>
}

impl<T> Arena<T> {
    pub fn new() -> Self {
        Self::with_capacity(8)
    }

    pub fn with_capacity(mut capacity: usize) -> Self {
        assert!(mem::size_of::<T>() > 0, "Arena does not support zero-sized types");
        if capacity == 0 {
            capacity = 1;  // So that it grows with `* 2`
        }
        let start_ptr = allocate::<T>(capacity);
        let end_ptr = unsafe {
            start_ptr.offset(capacity as isize)
        };
        Arena {
            current_block_start: Cell::new(start_ptr),
            current_block_next: Cell::new(start_ptr),
            current_block_end: Cell::new(end_ptr),
            previous_blocks: RefCell::new(Vec::new()),
        }
    }

    pub fn push(&self, item: T) -> &mut T {
        unsafe {
            let next = self.current_block_next.get();
            if next != self.current_block_end.get() {
                self.current_block_next.set(next.offset(1));
                ptr::write(next, item);
                return &mut *next
            } else {
                self.push_into_new_block(item)
            }
        }
    }

    /// Must only be called when self.current_block_next == self.current_block_end
    #[inline(never)]
    #[cold]
    unsafe fn push_into_new_block(&self, item: T) -> &mut T {
        let start = self.current_block_start.get();
        let len = ptr_pair_len(start, self.current_block_end.get());
        let slice = slice::from_raw_parts_mut(start, len);
        self.previous_blocks.borrow_mut().push(Box::from_raw(slice));

        let new_len = len.saturating_mul(2);
        let new_start = allocate::<T>(new_len);
        self.current_block_start.set(new_start);
        self.current_block_next.set(new_start.offset(1));
        self.current_block_end.set(new_start.offset(new_len as isize));

        ptr::write(new_start, item);
        &mut *new_start
    }
}

impl<T> Drop for Arena<T> {
    // If unsafe_destructor_blind_to_params is OK for Vec::drop it’s probably OK here
    // where we only touch T by dropping a Vec.
    #[unsafe_destructor_blind_to_params]
    fn drop(&mut self) {
        let start = self.current_block_start.get();
        let length = ptr_pair_len(start, self.current_block_next.get());
        let capacity = ptr_pair_len(start, self.current_block_end.get());
        unsafe {
            mem::drop(Vec::from_raw_parts(start, length, capacity))
        }
        // No need to deal with self.previous_blocks: dropping Box<[T]> does the right thing.
    }
}

fn allocate<T>(capacity: usize) -> *mut T {
    let mut vec = Vec::<T>::with_capacity(capacity);
    let ptr = vec.as_mut_ptr();
    mem::forget(vec);
    ptr
}

fn ptr_pair_len<T>(start: *const T, end: *const T) -> usize {
    let diff = (end as usize) - (start as usize);
    diff / mem::size_of::<T>()
}

#[test]
fn track_drop() {
    #[derive(PartialEq, Debug)]
    struct DropTracker<'a>(&'a Cell<u32>);
    impl<'a> Drop for DropTracker<'a> {
        fn drop(&mut self) {
            self.0.set(self.0.get() + 1);
        }
    }

    #[derive(PartialEq, Debug)]
    struct Node<'a, 'b: 'a>(Option<&'a Node<'a, 'b>>, u32, DropTracker<'b>);

    let drop_counter = Cell::new(0);
    {
        let arena = Arena::with_capacity(2);

        let mut node: &Node = arena.push(Node(None, 1, DropTracker(&drop_counter)));
        assert_eq!(arena.previous_blocks.borrow().len(), 0);

        node = arena.push(Node(Some(node), 2, DropTracker(&drop_counter)));
        assert_eq!(arena.previous_blocks.borrow().len(), 0);

        node = arena.push(Node(Some(node), 3, DropTracker(&drop_counter)));
        assert_eq!(arena.previous_blocks.borrow().len(), 1);

        node = arena.push(Node(Some(node), 4, DropTracker(&drop_counter)));
        assert_eq!(arena.previous_blocks.borrow().len(), 1);

        assert_eq!(node.1, 4);
        assert_eq!(node.0.unwrap().1, 3);
        assert_eq!(node.0.unwrap().0.unwrap().1, 2);
        assert_eq!(node.0.unwrap().0.unwrap().0.unwrap().1, 1);
        assert_eq!(node.0.unwrap().0.unwrap().0.unwrap().0, None);

        mem::drop(node);
        assert_eq!(drop_counter.get(), 0);

        let mut node: &Node = arena.push(Node(None, 5, DropTracker(&drop_counter)));
        assert_eq!(arena.previous_blocks.borrow().len(), 1);

        node = arena.push(Node(Some(node), 6, DropTracker(&drop_counter)));
        assert_eq!(arena.previous_blocks.borrow().len(), 1);

        node = arena.push(Node(Some(node), 7, DropTracker(&drop_counter)));
        assert_eq!(arena.previous_blocks.borrow().len(), 2);

        assert_eq!(drop_counter.get(), 0);

        assert_eq!(node.1, 7);
        assert_eq!(node.0.unwrap().1, 6);
        assert_eq!(node.0.unwrap().0.unwrap().1, 5);
        assert_eq!(node.0.unwrap().0.unwrap().0, None);

        assert_eq!(drop_counter.get(), 0);
    }
    assert_eq!(drop_counter.get(), 7);
}

#[test]
fn cycle() {
    struct Node<'a>(Cell<Option<&'a Node<'a>>>, Box<u32>);
    let arena = Arena::new();
    let a = arena.push(Node(Cell::new(None), Box::new(1)));
    let b = arena.push(Node(Cell::new(None), Box::new(2)));
    a.0 = Cell::new(Some(b));
    a.1 = Box::new(3);
    b.0.set(Some(a));
    let mut nums = Vec::new();
    let mut node = &*a;
    for _ in 0..10 {
        nums.push(*node.1);
        node = node.0.get().unwrap();
    }
    assert_eq!(nums, [3, 2, 3, 2, 3, 2, 3, 2, 3, 2])
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
    y.push(Foo(&x));
}
