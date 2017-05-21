use std::cell::RefCell;
use std::mem;

pub struct Arena<T> {
    refcell: RefCell<ArenaInner<T>>,
}

struct ArenaInner<T> {
    current_block: Vec<T>,
    previous_blocks: Vec<Vec<T>>,
}

impl<T> Arena<T> {
    pub fn new() -> Self {
        Self::with_capacity(8)
    }

    pub fn with_capacity(mut capacity: usize) -> Self {
        if capacity == 0 {
            capacity = 1;  // So that it grows with `* 2`
        }
        Arena {
            refcell: RefCell::new(ArenaInner {
                current_block: Vec::with_capacity(capacity),
                previous_blocks: Vec::new(),
            })
        }
    }

    pub fn push(&self, item: T) -> &mut T {
        let mut inner = self.refcell.borrow_mut();
        if inner.current_block.len() == inner.current_block.capacity() {
            #[inline(never)]
            #[cold]
            fn new_block<T>(inner: &mut ArenaInner<T>) {
                let new_capacity = inner.current_block.capacity().saturating_mul(2);
                let new_block = Vec::with_capacity(new_capacity);
                inner.previous_blocks.push(mem::replace(&mut inner.current_block, new_block));
            }
            new_block(&mut inner)
        }
        inner.current_block.push(item);
        let last_mut = inner.current_block.last_mut().unwrap();

        // Extend the reference’s lifetime from that of `inner` to that of `self`.
        // This is safe because:
        //
        // * We’re careful to never push a block’s `Vec` beyond its initial capacity
        //   (creating new blocks as necessary, instead),
        //   so that it never reallocates and its items are never moved:
        //   the pointer’s address remains valid until the `Vec` is dropped,
        //   which is when `self` is dropped.
        // * We never give out another reference to the same item:
        //   the reference returned from `push` is exclusive.
        //   If a mechanism (such as indexing) is ever added that gives out references to items,
        //   the reference returned from `push` would no longer be exclusive,
        //   its type would need to be changed from `&mut T` to `&T`.
        unsafe {
            mem::transmute::<&mut T, &mut T>(last_mut)
        }
    }
}

impl<T> ArenaInner<T> {
}

#[test]
fn track_drop() {
    use std::cell::Cell;

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
        assert_eq!(arena.refcell.borrow().previous_blocks.len(), 0);

        node = arena.push(Node(Some(node), 2, DropTracker(&drop_counter)));
        assert_eq!(arena.refcell.borrow().previous_blocks.len(), 0);

        node = arena.push(Node(Some(node), 3, DropTracker(&drop_counter)));
        assert_eq!(arena.refcell.borrow().previous_blocks.len(), 1);

        node = arena.push(Node(Some(node), 4, DropTracker(&drop_counter)));
        assert_eq!(arena.refcell.borrow().previous_blocks.len(), 1);

        assert_eq!(node.1, 4);
        assert_eq!(node.0.unwrap().1, 3);
        assert_eq!(node.0.unwrap().0.unwrap().1, 2);
        assert_eq!(node.0.unwrap().0.unwrap().0.unwrap().1, 1);
        assert_eq!(node.0.unwrap().0.unwrap().0.unwrap().0, None);

        mem::drop(node);
        assert_eq!(drop_counter.get(), 0);

        let mut node: &Node = arena.push(Node(None, 5, DropTracker(&drop_counter)));
        assert_eq!(arena.refcell.borrow().previous_blocks.len(), 1);

        node = arena.push(Node(Some(node), 6, DropTracker(&drop_counter)));
        assert_eq!(arena.refcell.borrow().previous_blocks.len(), 1);

        node = arena.push(Node(Some(node), 7, DropTracker(&drop_counter)));
        assert_eq!(arena.refcell.borrow().previous_blocks.len(), 2);

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
    use std::cell::Cell;

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
