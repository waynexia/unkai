#![feature(allocator_api)]
#![feature(btreemap_alloc)]

use std::{
    alloc::Global,
    collections::{BTreeSet, VecDeque},
};

use unkai::{Unkai, UnkaiGlobal};

fn main() {
    // `Unkai` implements `Default` for convenience, which uses `Global` as underlying allocator.
    // There is a type alias `UnkaiGlobal` for `Unkai<Global>`
    let mut vec_container: Vec<usize, UnkaiGlobal> = Vec::with_capacity_in(10000, Unkai::default());
    // This is the expanded version without type alias and `Default::default`
    let mut vec_deque_container: VecDeque<usize, Unkai<Global>> =
        VecDeque::with_capacity_in(10000, Unkai::new(Global));
    // BTreeSet doesn't provides an API to access its allocator.
    // `Unkai` implements `Clone` when the underlying allocator implements `Clone`.
    // For this case we need to keep a handle of unkai to report usage.
    let btree_set_allocator = Unkai::new(Global);
    let mut btree_set_container = BTreeSet::new_in(btree_set_allocator.clone());
    for i in 0..10000usize {
        btree_set_container.insert(i);
    }

    println!(
        "Vec defined at '{}' uses {} bytes",
        vec_container.allocator().report_caller(),
        vec_container.allocator().report_usage()
    );
    println!(
        "VecDeque defined at '{}' uses {} bytes",
        vec_deque_container.allocator().report_caller(),
        vec_deque_container.allocator().report_usage()
    );
    println!(
        "BTreeSet defined at '{}' uses {} bytes",
        btree_set_allocator.report_caller(),
        btree_set_allocator.report_usage()
    );

    // The output is something like:
    //
    // ```
    // Vec defined at 'examples/allocator.rs:12:83' uses 80000 bytes
    // VecDeque defined at 'examples/allocator.rs:14:43' uses 80000 bytes
    // BTreeSet defined at 'examples/allocator.rs:16:31' uses 196112 bytes
    // ```
}
