use criterion::{Criterion, criterion_group, criterion_main};
use schedule_parser::parse_xls;
use std::path::Path;

pub fn bench_parse_xls(c: &mut Criterion) {
    c.bench_function("parse_xls", |b| {
        b.iter(|| parse_xls(Path::new("../../schedule.xls")))
    });
}

criterion_group!(benches, bench_parse_xls);
criterion_main!(benches);
