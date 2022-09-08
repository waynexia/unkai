use std::collections::{HashMap, VecDeque};

use tikv_jemallocator::Jemalloc;
use unkai::UnkaiGlobalAlloc;

#[global_allocator]
static UNKAI: UnkaiGlobalAlloc<Jemalloc> = UnkaiGlobalAlloc::new(Jemalloc {});

fn main() {
    let mut container = Vec::with_capacity(10000);
    for _ in 0..10000 {
        let item = Vec::<usize>::with_capacity(1000);
        container.push(item);
    }

    let mut container2 = Vec::with_capacity(10000);
    for _ in 0..10000 {
        let item = HashMap::<usize, usize>::with_capacity(1000);
        container2.push(item);
    }

    let mut container3 = Vec::with_capacity(10000);
    for _ in 0..10000 {
        let item = VecDeque::<usize>::with_capacity(1000);
        container3.push(item);
    }

    println!("{:#?}", UNKAI.report_addr());
    println!();
    for (bt, count) in UNKAI.report_symbol() {
        println!("{} bytes are allocated from:", count);
        println!("{}", bt);
    }
}
