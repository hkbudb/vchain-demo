pub use algebra::curves::bls12_381 as curve;
pub use algebra::fields::bls12_381 as field;

pub mod digest_set;
pub mod utils;

use crate::set::{MultiSet, SetElement};
use algebra::{msm::VariableBaseMSM, AffineCurve, Field, PrimeField, ProjectiveCurve};
use curve::{G1Affine, G1Projective, G2Affine, G2Projective};
use digest_set::DigestSet;
use field::Fr;
use rayon::prelude::*;
use std::str::FromStr;
use utils::{FixedBaseCurvePow, FixedBaseScalarPow};

const GS_VEC_LEN: usize = 500;

lazy_static! {
    static ref PUB_Q: Fr = Fr::from_str("173169506511432145374212744878663118934").unwrap();
    static ref PRI_S: Fr = Fr::from_str("259535143263514268207918833918737523409").unwrap();
    static ref G1_POWER: FixedBaseCurvePow<G1Projective> =
        FixedBaseCurvePow::build(&G1Projective::prime_subgroup_generator());
    static ref G2_POWER: FixedBaseCurvePow<G2Projective> =
        FixedBaseCurvePow::build(&G2Projective::prime_subgroup_generator());
    static ref PRI_S_POWER: FixedBaseScalarPow<Fr> = FixedBaseScalarPow::build(&PRI_S);
    static ref G1_S_VEC: Vec<G1Affine> = {
        let mut res: Vec<G1Affine> = Vec::with_capacity(GS_VEC_LEN);
        (0..GS_VEC_LEN)
            .into_par_iter()
            .map(|i| get_g1s(Fr::from(i as u64)).into_affine())
            .collect_into_vec(&mut res);
        res
    };
    static ref G2_S_VEC: Vec<G2Affine> = {
        let mut res: Vec<G2Affine> = Vec::with_capacity(GS_VEC_LEN);
        (0..GS_VEC_LEN)
            .into_par_iter()
            .map(|i| get_g2s(Fr::from(i as u64)).into_affine())
            .collect_into_vec(&mut res);
        res
    };
}

fn get_g1s(coeff: Fr) -> G1Projective {
    let si = PRI_S_POWER.apply(&coeff);
    G1_POWER.apply(&si)
}

fn get_g2s(coeff: Fr) -> G2Projective {
    let si = PRI_S_POWER.apply(&coeff);
    G2_POWER.apply(&si)
}

pub enum Type {
    ACC1,
    ACC2,
}

pub trait Accumulator {
    const TYPE: Type;
    type Proof;

    fn cal_acc_g1_sk<T: SetElement>(set: &MultiSet<T>) -> G1Affine {
        Self::cal_acc_g1_sk_d(&DigestSet::new(set))
    }
    fn cal_acc_g1<T: SetElement>(set: &MultiSet<T>) -> G1Affine {
        Self::cal_acc_g1_d(&DigestSet::new(set))
    }
    fn cal_acc_g2_sk<T: SetElement>(set: &MultiSet<T>) -> G2Affine {
        Self::cal_acc_g2_sk_d(&DigestSet::new(set))
    }
    fn cal_acc_g2<T: SetElement>(set: &MultiSet<T>) -> G2Affine {
        Self::cal_acc_g2_d(&DigestSet::new(set))
    }
    fn cal_acc_g1_sk_d(set: &DigestSet) -> G1Affine;
    fn cal_acc_g1_d(set: &DigestSet) -> G1Affine;
    fn cal_acc_g2_sk_d(set: &DigestSet) -> G2Affine;
    fn cal_acc_g2_d(set: &DigestSet) -> G2Affine;
    fn gen_proof(set1: &DigestSet, set2: &DigestSet) -> Self::Proof;
}

pub struct Acc1;

pub struct Acc1Proof {}

impl Accumulator for Acc1 {
    const TYPE: Type = Type::ACC1;
    type Proof = Acc1Proof;

    fn cal_acc_g1_sk_d(set: &DigestSet) -> G1Affine {
        let mut x = Fr::one();
        for (v, exp) in set.iter() {
            let s = *PRI_S + v;
            let exp = [*exp as u64];
            x *= &s.pow(&exp);
        }
        G1_POWER.apply(&x).into_affine()
    }
    fn cal_acc_g1_d(set: &DigestSet) -> G1Affine {
        let poly = set.expand_to_poly();
        let mut bases: Vec<G1Affine> = Vec::with_capacity(poly.degree() + 1);
        let mut scalars: Vec<<Fr as PrimeField>::BigInt> = Vec::with_capacity(poly.degree() + 1);
        for (i, coeff) in poly.coeffs.iter().enumerate() {
            if coeff.is_zero() {
                continue;
            }
            let gs = G1_S_VEC
                .get(i)
                .map(|v| *v)
                .unwrap_or(get_g1s(Fr::from(i as u64)).into_affine());
            bases.push(gs);
            scalars.push(coeff.into_repr());
        }
        VariableBaseMSM::multi_scalar_mul(&bases[..], &scalars[..]).into_affine()
    }
    fn cal_acc_g2_sk_d(set: &DigestSet) -> G2Affine {
        let mut x = Fr::one();
        for (v, exp) in set.iter() {
            let s = *PRI_S + v;
            let exp = [*exp as u64];
            x *= &s.pow(&exp);
        }
        G2_POWER.apply(&x).into_affine()
    }
    fn cal_acc_g2_d(set: &DigestSet) -> G2Affine {
        let poly = set.expand_to_poly();
        let mut bases: Vec<G2Affine> = Vec::with_capacity(poly.degree() + 1);
        let mut scalars: Vec<<Fr as PrimeField>::BigInt> = Vec::with_capacity(poly.degree() + 1);
        for (i, coeff) in poly.coeffs.iter().enumerate() {
            if coeff.is_zero() {
                continue;
            }
            let gs = G2_S_VEC
                .get(i)
                .map(|v| *v)
                .unwrap_or(get_g2s(Fr::from(i as u64)).into_affine());
            bases.push(gs);
            scalars.push(coeff.into_repr());
        }
        VariableBaseMSM::multi_scalar_mul(&bases[..], &scalars[..]).into_affine()
    }
    fn gen_proof(set1: &DigestSet, set2: &DigestSet) -> Self::Proof {
        todo!();
    }
}

pub struct Acc2;

pub struct Acc2Proof {}

impl Accumulator for Acc2 {
    const TYPE: Type = Type::ACC2;
    type Proof = Acc2Proof;

    fn cal_acc_g1_sk_d(set: &DigestSet) -> G1Affine {
        let mut x = Fr::zero();
        for (a, b) in set.iter() {
            let s = PRI_S_POWER.apply(a);
            x += &(s * &Fr::from(*b));
        }
        G1_POWER.apply(&x).into_affine()
    }
    fn cal_acc_g1_d(set: &DigestSet) -> G1Affine {
        set.par_iter()
            .map(|(a, b)| {
                let mut sa = get_g1s(*a);
                sa.mul_assign(*b as u64);
                sa
            })
            .reduce(|| G1Projective::zero(), |a, b| a + &b)
            .into_affine()
    }
    fn cal_acc_g2_sk_d(set: &DigestSet) -> G2Affine {
        let mut x = Fr::zero();
        for (a, b) in set.iter() {
            let s = PRI_S_POWER.apply(&(*PUB_Q - &a));
            x += &(s * &Fr::from(*b));
        }
        G2_POWER.apply(&x).into_affine()
    }
    fn cal_acc_g2_d(set: &DigestSet) -> G2Affine {
        set.par_iter()
            .map(|(a, b)| {
                let mut sa = get_g2s(*PUB_Q - a);
                sa.mul_assign(*b as u64);
                sa
            })
            .reduce(|| G2Projective::zero(), |a, b| a + &b)
            .into_affine()
    }
    fn gen_proof(set1: &DigestSet, set2: &DigestSet) -> Self::Proof {
        todo!();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cal_acc() {
        let set = MultiSet::from_vec(vec![1, 1, 2, 3, 4, 4, 5, 6, 6, 7, 8, 9]);
        assert_eq!(Acc1::cal_acc_g1(&set), Acc1::cal_acc_g1_sk(&set));
        assert_eq!(Acc1::cal_acc_g2(&set), Acc1::cal_acc_g2_sk(&set));
        assert_eq!(Acc2::cal_acc_g1(&set), Acc2::cal_acc_g1_sk(&set));
        assert_eq!(Acc2::cal_acc_g2(&set), Acc2::cal_acc_g2_sk(&set));
    }
}
