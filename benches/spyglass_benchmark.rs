use criterion::{black_box, criterion_group, criterion_main, Criterion};

use spyglass::SuffixTrie;

fn benchmark_find(c: &mut Criterion) {
    let trie = SuffixTrie::from_file(black_box("./resources/tests/large_100/para.txt")).unwrap();
    c.bench_function("find_exact_para", |b| b.iter(|| black_box(&trie).find_exact("that")));
    c.bench_function("find_edit_0_para", |b| b.iter(|| black_box(&trie).find_edit_distance("that", 0)));
    c.bench_function("find_edit_0_para", |b| b.iter(|| black_box(&trie).find_edit_distance("loss ofEdEN", 2)));
}

fn benchmark_dir_100(c: &mut Criterion) {
    c.bench_function("large_100", |b| b.iter(|| SuffixTrie::from_directory(black_box("./resources/tests/large_100/"))));
}

fn benchmark_shakespeare_100(c: &mut Criterion) {
    c.bench_function("shakespeare_100", |b| b.iter(|| SuffixTrie::from_file(black_box("./resources/tests/large_100/shakespeare.txt"))));
    c.bench_function("shakespeare_100_sent", |b| b.iter(|| SuffixTrie::from_file(black_box("./resources/tests/untokenized/shakespeare.txt"))));
}

criterion_group!(benches, benchmark_dir_100, benchmark_shakespeare_100, benchmark_find);
criterion_group!(benches_quick, benchmark_shakespeare_100);// benchmark_find);
criterion_main!(benches);
