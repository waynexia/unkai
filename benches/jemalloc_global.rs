use tikv_jemallocator::Jemalloc;

use criterion::{black_box, criterion_group, criterion_main, Criterion};

#[global_allocator]
static JEMALLOC: Jemalloc = Jemalloc {};

pub fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("jemalloc_global", |b| {
        b.iter(|| {
            let item = Vec::<usize>::with_capacity(1000);
            black_box(item);
        })
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
