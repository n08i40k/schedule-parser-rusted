use criterion::{Criterion, criterion_group, criterion_main};
use schedule_parser::parse_xls;

pub fn bench_parse_xls(c: &mut Criterion) {
    let buffer: Vec<u8> = include_bytes!("../../../schedule.xls").to_vec();

    c.bench_function("parse_xls", |b| b.iter(|| parse_xls(&buffer)));
}

criterion_group!(benches, bench_parse_xls);
criterion_main!(benches);
