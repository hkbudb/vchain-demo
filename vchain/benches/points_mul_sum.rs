use ark_ec::{msm::VariableBaseMSM, AffineCurve, ProjectiveCurve};
use ark_ff::{PrimeField, UniformRand, Zero};
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use rand::SeedableRng;
use rayon::prelude::*;
use vchain::acc::utils::*;
use vchain::acc::{Fr, G1Projective as G1};

fn naive<G: AffineCurve>(
    bases: &[G],
    scalars: &[<G::ScalarField as PrimeField>::BigInt],
) -> G::Projective {
    let mut acc = G::Projective::zero();

    for (base, scalar) in bases.iter().zip(scalars.iter()) {
        acc += &base.mul(*scalar);
    }
    acc
}

fn fixed_base_pow<G: ProjectiveCurve>(
    bases: &[FixedBaseCurvePow<G>],
    scalars: &[<G::ScalarField as PrimeField>::BigInt],
) -> G {
    let mut acc = G::zero();

    for (base, scalar) in bases.iter().zip(scalars.iter()) {
        acc += &base.apply(
            &<G::ScalarField as PrimeField>::from_repr(*scalar)
                .expect("failed to convert to prime field"),
        );
    }
    acc
}

pub fn bench_points_mul_sum(c: &mut Criterion) {
    const SAMPLES: usize = 1 << 10;
    let mut rng = rand::rngs::StdRng::seed_from_u64(123_456_789u64);

    let v = (0..SAMPLES)
        .map(|_| Fr::rand(&mut rng).into_repr())
        .collect::<Vec<_>>();
    let g = (0..SAMPLES)
        .map(|_| G1::rand(&mut rng).into_affine())
        .collect::<Vec<_>>();
    let mut gp: Vec<FixedBaseCurvePow<G1>> = Vec::with_capacity(g.len());
    (0..g.len())
        .into_par_iter()
        .map(|i| FixedBaseCurvePow::build(&g[i].into_projective()))
        .collect_into_vec(&mut gp);

    let mut group = c.benchmark_group("points_mul_sum");
    group.sample_size(10);
    group.bench_function("naive", |b| {
        b.iter(|| black_box(naive(g.as_slice(), v.as_slice())))
    });
    group.bench_function("multi_scalar_mul", |b| {
        b.iter(|| {
            black_box(VariableBaseMSM::multi_scalar_mul(
                g.as_slice(),
                v.as_slice(),
            ))
        })
    });
    group.bench_function("fixed_base_pow", |b| {
        b.iter(|| black_box(fixed_base_pow(gp.as_slice(), v.as_slice())))
    });
    group.finish();
}

criterion_group!(benches, bench_points_mul_sum);
criterion_main!(benches);
