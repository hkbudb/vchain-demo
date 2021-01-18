pub use algebra::curves::bls12_381 as curve;
pub use algebra::fields::bls12_381 as field;
pub use curve::Bls12_381 as Curve;

pub mod digest_set;
pub mod serde_impl;
pub mod utils;

pub use digest_set::DigestSet;

use crate::digest::{Digest, Digestible};
use crate::set::{MultiSet, SetElement};
use algebra::{
    bytes::ToBytes, msm::VariableBaseMSM, AffineCurve, Field, PairingCurve, PairingEngine,
    PrimeField, ProjectiveCurve,
};
use anyhow::{self, bail, ensure, Context};
use core::any::Any;
use core::str::FromStr;
use curve::{G1Affine, G1Projective, G2Affine, G2Projective};
use ff_fft::DensePolynomial;
use field::{Fq12, Fr};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use utils::{xgcd, FixedBaseCurvePow, FixedBaseScalarPow};

#[cfg(test)]
const GS_VEC_LEN: usize = 0;
#[cfg(not(test))]
const GS_VEC_LEN: usize = 5000;

lazy_static! {
    // 250 bits
    static ref PUB_Q: Fr = Fr::from_str("480721077433357505777975950918924200361380912084288598463024400624539293706").unwrap();
    // 128 bits
    static ref PRI_S: Fr = Fr::from_str("259535143263514268207918833918737523409").unwrap();
    static ref G1_POWER: FixedBaseCurvePow<G1Projective> =
        FixedBaseCurvePow::build(&G1Projective::prime_subgroup_generator());
    static ref G2_POWER: FixedBaseCurvePow<G2Projective> =
        FixedBaseCurvePow::build(&G2Projective::prime_subgroup_generator());
    static ref PRI_S_POWER: FixedBaseScalarPow<Fr> = FixedBaseScalarPow::build(&PRI_S);
    static ref G1_S_VEC: Vec<G1Affine> = {
        info!("Initialize G1_S_VEC...");
        let timer = howlong::ProcessCPUTimer::new();
        let mut res: Vec<G1Affine> = Vec::with_capacity(GS_VEC_LEN);
        (0..GS_VEC_LEN)
            .into_par_iter()
            .map(|i| get_g1s(Fr::from(i as u64)))
            .collect_into_vec(&mut res);
        info!("Done in {}.", timer.elapsed());
        res
    };
    static ref G2_S_VEC: Vec<G2Affine> = {
        info!("Initialize G2_S_VEC...");
        let timer = howlong::ProcessCPUTimer::new();
        let mut res: Vec<G2Affine> = Vec::with_capacity(GS_VEC_LEN);
        (0..GS_VEC_LEN)
            .into_par_iter()
            .map(|i| get_g2s(Fr::from(i as u64)))
            .collect_into_vec(&mut res);
        info!("Done in {}.", timer.elapsed());
        res
    };
    static ref E_G_G: Fq12 = Curve::pairing(
        G1Affine::prime_subgroup_generator(),
        G2Affine::prime_subgroup_generator()
    );
}

fn get_g1s(coeff: Fr) -> G1Affine {
    let si = PRI_S_POWER.apply(&coeff);
    G1_POWER.apply(&si).into_affine()
}

fn get_g2s(coeff: Fr) -> G2Affine {
    let si = PRI_S_POWER.apply(&coeff);
    G2_POWER.apply(&si).into_affine()
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
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

pub trait AccumulatorProof: Eq + PartialEq {
    const TYPE: Type;

    fn gen_proof(set1: &DigestSet, set2: &DigestSet) -> anyhow::Result<Self>
    where
        Self: core::marker::Sized;

    fn combine_proof(&mut self, other: &Self) -> anyhow::Result<()>;

    fn as_any(&self) -> &dyn Any;
}

pub struct Acc1;

impl Acc1 {
    fn poly_to_g1(poly: DensePolynomial<Fr>) -> G1Affine {
        let mut idxes: Vec<usize> = Vec::with_capacity(poly.degree() + 1);
        for (i, coeff) in poly.coeffs.iter().enumerate() {
            if coeff.is_zero() {
                continue;
            }
            idxes.push(i);
        }

        let mut bases: Vec<G1Affine> = Vec::with_capacity(idxes.len());
        let mut scalars: Vec<<Fr as PrimeField>::BigInt> = Vec::with_capacity(idxes.len());
        (0..idxes.len())
            .into_par_iter()
            .map(|i| {
                G1_S_VEC.get(i).copied().unwrap_or_else(|| {
                    trace!("access g1 pub key at {}", i);
                    get_g1s(Fr::from(i as u64))
                })
            })
            .collect_into_vec(&mut bases);
        (0..idxes.len())
            .into_par_iter()
            .map(|i| poly.coeffs[i].into_repr())
            .collect_into_vec(&mut scalars);

        VariableBaseMSM::multi_scalar_mul(&bases[..], &scalars[..]).into_affine()
    }

    fn poly_to_g2(poly: DensePolynomial<Fr>) -> G2Affine {
        let mut idxes: Vec<usize> = Vec::with_capacity(poly.degree() + 1);
        for (i, coeff) in poly.coeffs.iter().enumerate() {
            if coeff.is_zero() {
                continue;
            }
            idxes.push(i);
        }

        let mut bases: Vec<G2Affine> = Vec::with_capacity(idxes.len());
        let mut scalars: Vec<<Fr as PrimeField>::BigInt> = Vec::with_capacity(idxes.len());
        (0..idxes.len())
            .into_par_iter()
            .map(|i| {
                G2_S_VEC.get(i).copied().unwrap_or_else(|| {
                    trace!("access g2 pub key at {}", i);
                    get_g2s(Fr::from(i as u64))
                })
            })
            .collect_into_vec(&mut bases);
        (0..idxes.len())
            .into_par_iter()
            .map(|i| poly.coeffs[i].into_repr())
            .collect_into_vec(&mut scalars);

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

impl AccumulatorProof for Acc1Proof {
    const TYPE: Type = Type::ACC1;

    fn gen_proof(set1: &DigestSet, set2: &DigestSet) -> anyhow::Result<Self> {
        Acc1::gen_proof(set1, set2)
    }

    fn combine_proof(&mut self, _other: &Self) -> anyhow::Result<()> {
        bail!("invalid operation");
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
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
        let x = set
            .par_iter()
            .map(|(v, exp)| {
                let s = *PRI_S + v;
                let exp = [*exp as u64];
                s.pow(&exp)
            })
            .reduce(Fr::one, |a, b| a * &b);
        G1_POWER.apply(&x).into_affine()
    }
    fn cal_acc_g1_d(set: &DigestSet) -> G1Affine {
        let poly = set.expand_to_poly();
        Self::poly_to_g1(poly)
    }
    fn cal_acc_g2_sk_d(set: &DigestSet) -> G2Affine {
        let x = set
            .par_iter()
            .map(|(v, exp)| {
                let s = *PRI_S + v;
                let exp = [*exp as u64];
                s.pow(&exp)
            })
            .reduce(Fr::one, |a, b| a * &b);
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

impl AccumulatorProof for Acc2Proof {
    const TYPE: Type = Type::ACC2;

    fn gen_proof(set1: &DigestSet, set2: &DigestSet) -> anyhow::Result<Self> {
        Acc2::gen_proof(set1, set2)
    }

    fn combine_proof(&mut self, other: &Self) -> anyhow::Result<()> {
        let mut f = self.f.into_projective();
        f.add_assign_mixed(&other.f);
        self.f = f.into_affine();
        Ok(())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
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
        let x = set
            .par_iter()
            .map(|(a, b)| {
                let s = PRI_S_POWER.apply(a);
                s * &Fr::from(*b)
            })
            .reduce(Fr::zero, |a, b| a + &b);
        G1_POWER.apply(&x).into_affine()
    }
    fn cal_acc_g1_d(set: &DigestSet) -> G1Affine {
        let mut bases: Vec<G1Affine> = Vec::with_capacity(set.len());
        let mut scalars: Vec<<Fr as PrimeField>::BigInt> = Vec::with_capacity(set.len());
        (0..set.len())
            .into_par_iter()
            .map(|i| get_g1s(set[i].0))
            .collect_into_vec(&mut bases);
        (0..set.len())
            .into_par_iter()
            .map(|i| <Fr as PrimeField>::BigInt::from(set[i].1 as u64))
            .collect_into_vec(&mut scalars);
        VariableBaseMSM::multi_scalar_mul(&bases[..], &scalars[..]).into_affine()
    }
    fn cal_acc_g2_sk_d(set: &DigestSet) -> G2Affine {
        let x = set
            .par_iter()
            .map(|(a, b)| {
                let s = PRI_S_POWER.apply(&(*PUB_Q - a));
                s * &Fr::from(*b)
            })
            .reduce(Fr::zero, |a, b| a + &b);
        G2_POWER.apply(&x).into_affine()
    }
    fn cal_acc_g2_d(set: &DigestSet) -> G2Affine {
        let mut bases: Vec<G2Affine> = Vec::with_capacity(set.len());
        let mut scalars: Vec<<Fr as PrimeField>::BigInt> = Vec::with_capacity(set.len());
        (0..set.len())
            .into_par_iter()
            .map(|i| get_g2s(*PUB_Q - &set[i].0))
            .collect_into_vec(&mut bases);
        (0..set.len())
            .into_par_iter()
            .map(|i| <Fr as PrimeField>::BigInt::from(set[i].1 as u64))
            .collect_into_vec(&mut scalars);
        VariableBaseMSM::multi_scalar_mul(&bases[..], &scalars[..]).into_affine()
    }
    fn gen_proof(set1: &DigestSet, set2: &DigestSet) -> anyhow::Result<Self::Proof> {
        let produce_size = set1.len() * set2.len();
        let mut product: Vec<(Fr, u64)> = Vec::with_capacity(produce_size);
        (0..produce_size)
            .into_par_iter()
            .map(|i| {
                let set1idx = i / set2.len();
                let set2idx = i % set2.len();
                let (s1, q1) = set1[set1idx];
                let (s2, q2) = set2[set2idx];
                (*PUB_Q + &s1 - &s2, (q1 * q2) as u64)
            })
            .collect_into_vec(&mut product);
        if product.par_iter().any(|(x, _)| *x == *PUB_Q) {
            bail!("cannot generate proof");
        }

        let mut bases: Vec<G1Affine> = Vec::with_capacity(produce_size);
        let mut scalars: Vec<<Fr as PrimeField>::BigInt> = Vec::with_capacity(produce_size);
        (0..produce_size)
            .into_par_iter()
            .map(|i| get_g1s(product[i].0))
            .collect_into_vec(&mut bases);
        (0..produce_size)
            .into_par_iter()
            .map(|i| <Fr as PrimeField>::BigInt::from(product[i].1))
            .collect_into_vec(&mut scalars);
        let f = VariableBaseMSM::multi_scalar_mul(&bases[..], &scalars[..]).into_affine();
        Ok(Acc2Proof { f })
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum Proof {
    ACC1(Box<Acc1Proof>),
    ACC2(Box<Acc2Proof>),
}

impl Digestible for G1Affine {
    fn to_digest(&self) -> Digest {
        let mut buf = Vec::<u8>::new();
        self.write(&mut buf)
            .unwrap_or_else(|_| panic!("failed to serialize {:?}", self));
        buf.to_digest()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn init_logger() {
        let _ = env_logger::builder().is_test(true).try_init();
    }

    #[test]
    fn test_cal_acc() {
        init_logger();
        let set = MultiSet::from_vec(vec![1, 1, 2, 3, 4, 4, 5, 6, 6, 7, 8, 9]);
        assert_eq!(Acc1::cal_acc_g1(&set), Acc1::cal_acc_g1_sk(&set));
        assert_eq!(Acc1::cal_acc_g2(&set), Acc1::cal_acc_g2_sk(&set));
        assert_eq!(Acc2::cal_acc_g1(&set), Acc2::cal_acc_g1_sk(&set));
        assert_eq!(Acc2::cal_acc_g2(&set), Acc2::cal_acc_g2_sk(&set));
    }

    #[test]
    fn test_acc1_proof() {
        init_logger();
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
        init_logger();
        let set1 = DigestSet::new(&MultiSet::from_vec(vec![1, 2, 3]));
        let set2 = DigestSet::new(&MultiSet::from_vec(vec![4, 5, 6]));
        let set3 = DigestSet::new(&MultiSet::from_vec(vec![1, 1]));
        let proof = Acc2::gen_proof(&set1, &set2).unwrap();
        let acc1 = Acc2::cal_acc_g1_sk_d(&set1);
        let acc2 = Acc2::cal_acc_g2_sk_d(&set2);
        assert!(proof.verify(&acc1, &acc2));
        assert!(Acc2::gen_proof(&set1, &set3).is_err());
    }

    #[test]
    fn test_acc2_proof_sum() {
        init_logger();
        let set1 = DigestSet::new(&MultiSet::from_vec(vec![1, 2, 3]));
        let set2 = DigestSet::new(&MultiSet::from_vec(vec![4, 5, 6]));
        let set3 = DigestSet::new(&MultiSet::from_vec(vec![7, 8, 9]));
        let mut proof1 = Acc2::gen_proof(&set1, &set2).unwrap();
        let proof2 = Acc2::gen_proof(&set1, &set3).unwrap();
        proof1.combine_proof(&proof2).unwrap();
        let acc1 = Acc2::cal_acc_g1_sk_d(&set1);
        let acc2 = Acc2::cal_acc_g2_sk_d(&set2);
        let acc3 = Acc2::cal_acc_g2_sk_d(&set3);
        let acc4 = {
            let mut acc = acc2.into_projective();
            acc.add_assign_mixed(&acc3);
            acc.into_affine()
        };
        assert!(proof1.verify(&acc1, &acc4));
    }
}
