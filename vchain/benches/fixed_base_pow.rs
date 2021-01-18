use ark_ec::ProjectiveCurve;
use ark_ff::{Field, PrimeField};
use core::ops::MulAssign;
use core::str::FromStr;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use vchain::acc::utils::*;
use vchain::acc::{Fr, G1Projective as G1, G2Projective as G2};

pub fn bench_pow_g1(c: &mut Criterion) {
    let mut group = c.benchmark_group("pow_g1");
    let num = Fr::from_str("1050806240378915932164293810269605748").unwrap();
    let g1p = FixedBaseCurvePow::build(&G1::prime_subgroup_generator());
    group.bench_function("normal", |b| {
        b.iter(|| {
            let mut ans = G1::prime_subgroup_generator();
            ans.mul_assign(black_box(num));
        })
    });
    group.bench_function("optimized", |b| b.iter(|| g1p.apply(black_box(&num))));
    group.finish();
}

pub fn bench_pow_g2(c: &mut Criterion) {
    let mut group = c.benchmark_group("pow_g2");
    let num = Fr::from_str("1050806240378915932164293810269605748").unwrap();
    let g2p = FixedBaseCurvePow::build(&G2::prime_subgroup_generator());
    group.bench_function("nomral", |b| {
        b.iter(|| {
            let mut ans = G2::prime_subgroup_generator();
            ans.mul_assign(black_box(num));
        })
    });
    group.bench_function("optimized", |b| b.iter(|| g2p.apply(black_box(&num))));
    group.finish();
}

pub fn bench_pow_fr(c: &mut Criterion) {
    let mut group = c.benchmark_group("pow_fr");
    let base = Fr::from_str("186375271183577333671420248211302045980").unwrap();
    let num = Fr::from_str("1050806240378915932164293810269605748").unwrap();
    let frp = FixedBaseScalarPow::build(&base);
    group.bench_function("nomral", |b| {
        b.iter(|| base.pow(black_box(num.into_repr())))
    });
    group.bench_function("optimized", |b| b.iter(|| frp.apply(black_box(&num))));
    group.finish();
}

criterion_group!(benches, bench_pow_g1, bench_pow_g2, bench_pow_fr);
criterion_main!(benches);
