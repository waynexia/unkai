use tikv_jemallocator::Jemalloc;

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use unkai::UnkaiGlobalAlloc;

#[global_allocator]
static UNKAI: UnkaiGlobalAlloc<Jemalloc> = UnkaiGlobalAlloc::new(Jemalloc {}, 99, 5, 10, 0);

pub fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("unkai_jemalloc_global", |b| {
        b.iter(|| {
            let item = Vec::<usize>::with_capacity(1000);
            black_box(item);
        })
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
