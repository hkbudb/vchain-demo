pub use algebra::curves::bls12_381 as curve;
pub use algebra::fields::bls12_381 as field;
pub use curve::Bls12_381 as Curve;

pub mod digest_set;
pub mod serde_impl;
pub mod utils;

use crate::set::{MultiSet, SetElement};
use algebra::{
    msm::VariableBaseMSM, AffineCurve, Field, PairingCurve, PairingEngine, PrimeField,
    ProjectiveCurve,
};
use anyhow::{self, bail, ensure, Context};
use curve::{G1Affine, G1Projective, G2Affine, G2Projective};
use digest_set::DigestSet;
use ff_fft::DensePolynomial;
use field::{Fq12, Fr};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use utils::{xgcd, FixedBaseCurvePow, FixedBaseScalarPow};

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
    static ref E_G_G: Fq12 = Curve::pairing(
        G1Affine::prime_subgroup_generator(),
        G2Affine::prime_subgroup_generator()
    );
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
    fn gen_proof(set1: &DigestSet, set2: &DigestSet) -> anyhow::Result<Self::Proof>;
}

pub struct Acc1;

impl Acc1 {
    fn poly_to_g1(poly: DensePolynomial<Fr>) -> G1Affine {
        let mut bases: Vec<G1Affine> = Vec::with_capacity(poly.degree() + 1);
        let mut scalars: Vec<<Fr as PrimeField>::BigInt> = Vec::with_capacity(poly.degree() + 1);
        for (i, coeff) in poly.coeffs.iter().enumerate() {
            if coeff.is_zero() {
                continue;
            }
            let gs = G1_S_VEC
                .get(i)
                .copied()
                .unwrap_or_else(|| get_g1s(Fr::from(i as u64)).into_affine());
            bases.push(gs);
            scalars.push(coeff.into_repr());
        }
        VariableBaseMSM::multi_scalar_mul(&bases[..], &scalars[..]).into_affine()
    }

    fn poly_to_g2(poly: DensePolynomial<Fr>) -> G2Affine {
        let mut bases: Vec<G2Affine> = Vec::with_capacity(poly.degree() + 1);
        let mut scalars: Vec<<Fr as PrimeField>::BigInt> = Vec::with_capacity(poly.degree() + 1);
        for (i, coeff) in poly.coeffs.iter().enumerate() {
            if coeff.is_zero() {
                continue;
            }
            let gs = G2_S_VEC
                .get(i)
                .copied()
                .unwrap_or_else(|| get_g2s(Fr::from(i as u64)).into_affine());
            bases.push(gs);
            scalars.push(coeff.into_repr());
        }
        VariableBaseMSM::multi_scalar_mul(&bases[..], &scalars[..]).into_affine()
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Acc1Proof {
    #[serde(with = "serde_impl")]
    f1: G2Affine,
    #[serde(with = "serde_impl")]
    f2: G2Affine,
}

impl Acc1Proof {
    pub fn verify(&self, acc1: &G1Affine, acc2: &G1Affine) -> bool {
        Curve::product_of_pairings(&[
            (&acc1.prepare(), &self.f1.prepare()),
            (&acc2.prepare(), &self.f2.prepare()),
        ]) == *E_G_G
    }
}

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
        Self::poly_to_g1(poly)
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
        Self::poly_to_g2(poly)
    }
    fn gen_proof(set1: &DigestSet, set2: &DigestSet) -> anyhow::Result<Self::Proof> {
        let poly1 = set1.expand_to_poly();
        let poly2 = set2.expand_to_poly();
        let (g, x, y) = xgcd(poly1, poly2).context("failed to compute xgcd")?;
        ensure!(g.degree() == 0, "cannot generate proof");
        Ok(Acc1Proof {
            f1: Self::poly_to_g2(&x / &g),
            f2: Self::poly_to_g2(&y / &g),
        })
    }
}

pub struct Acc2;

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Acc2Proof {
    #[serde(with = "serde_impl")]
    f: G1Affine,
}

impl Acc2Proof {
    pub fn verify(&self, acc1: &G1Affine, acc2: &G2Affine) -> bool {
        let a = Curve::pairing(*acc1, *acc2);
        let b = Curve::pairing(self.f, G2Affine::prime_subgroup_generator());
        a == b
    }
}

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
            .reduce(G1Projective::zero, |a, b| a + &b)
            .into_affine()
    }
    fn cal_acc_g2_sk_d(set: &DigestSet) -> G2Affine {
        let mut x = Fr::zero();
        for (a, b) in set.iter() {
            let s = PRI_S_POWER.apply(&(*PUB_Q - a));
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
            .reduce(G2Projective::zero, |a, b| a + &b)
            .into_affine()
    }
    fn gen_proof(set1: &DigestSet, set2: &DigestSet) -> anyhow::Result<Self::Proof> {
        let produce_size = set1.len() * set2.len();
        let mut product: Vec<(Fr, u32)> = Vec::with_capacity(produce_size);
        (0..produce_size)
            .into_par_iter()
            .map(|i| {
                let set1idx = i / set2.len();
                let set2idx = i % set2.len();
                let (s1, q1) = set1[set1idx];
                let (s2, q2) = set2[set2idx];
                (*PUB_Q + &s1 - &s2, q1 * q2)
            })
            .collect_into_vec(&mut product);
        if product.par_iter().any(|(x, _)| *x == *PUB_Q) {
            bail!("cannot generate proof");
        }
        let f = product
            .par_iter()
            .map(|(a, b)| {
                let mut sa = get_g1s(*a);
                sa.mul_assign(*b as u64);
                sa
            })
            .reduce(G1Projective::zero, |a, b| a + &b)
            .into_affine();
        Ok(Acc2Proof { f })
    }
}

pub enum Proof {
    ACC1(Acc1Proof),
    ACC2(Acc2Proof),
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

    #[test]
    fn test_acc1_proof() {
        let set1 = DigestSet::new(&MultiSet::from_vec(vec![1, 2, 3]));
        let set2 = DigestSet::new(&MultiSet::from_vec(vec![4, 5, 6]));
        let set3 = DigestSet::new(&MultiSet::from_vec(vec![1, 1]));
        let proof = Acc1::gen_proof(&set1, &set2).unwrap();
        let acc1 = Acc1::cal_acc_g1_sk_d(&set1);
        let acc2 = Acc1::cal_acc_g1_sk_d(&set2);
        assert!(proof.verify(&acc1, &acc2));
        assert!(Acc1::gen_proof(&set1, &set3).is_err());
    }

    #[test]
    fn test_acc2_proof() {
        let set1 = DigestSet::new(&MultiSet::from_vec(vec![1, 2, 3]));
        let set2 = DigestSet::new(&MultiSet::from_vec(vec![4, 5, 6]));
        let set3 = DigestSet::new(&MultiSet::from_vec(vec![1, 1]));
        let proof = Acc2::gen_proof(&set1, &set2).unwrap();
        let acc1 = Acc2::cal_acc_g1_sk_d(&set1);
        let acc2 = Acc2::cal_acc_g2_sk_d(&set2);
        assert!(proof.verify(&acc1, &acc2));
        assert!(Acc2::gen_proof(&set1, &set3).is_err());
    }
}
