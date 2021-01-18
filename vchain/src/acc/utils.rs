use crate::digest::Digest;
use ark_ec::ProjectiveCurve;
use ark_ff::{BigInteger, FpParameters, PrimeField, Zero};
use ark_poly::{
    univariate::{DenseOrSparsePolynomial, DensePolynomial},
    UVPolynomial,
};
use itertools::unfold;

pub fn try_digest_to_prime_field<F: PrimeField>(input: &Digest) -> Option<F> {
    let mut num = F::from_be_bytes_mod_order(&input.0).into_repr();
    // ensure the result is at most in 248 bits. so PUB_Q - Fr and Fr + PUB_Q - Fr never overflow.
    for v in num.as_mut().iter_mut().skip(3) {
        *v = 0;
    }
    num.as_mut().get_mut(3).map(|v| *v &= 0x00ff_ffff_ffff_ffff);
    F::from_repr(num)
}

pub fn digest_to_prime_field<F: PrimeField>(input: &Digest) -> F {
    try_digest_to_prime_field(input).expect("failed to convert digest to prime field")
}

/// Return (g, x, y) s.t. a*x + b*y = g = gcd(a, b)
pub fn xgcd<'a, F: PrimeField>(
    a: impl Into<DenseOrSparsePolynomial<'a, F>>,
    b: impl Into<DenseOrSparsePolynomial<'a, F>>,
) -> Option<(DensePolynomial<F>, DensePolynomial<F>, DensePolynomial<F>)> {
    let mut a = a.into();
    let mut b = b.into();
    let mut x0 = DensePolynomial::<F>::zero();
    let mut x1 = DensePolynomial::<F>::from_coefficients_vec(vec![F::one()]);
    let mut y0 = DensePolynomial::<F>::from_coefficients_vec(vec![F::one()]);
    let mut y1 = DensePolynomial::<F>::zero();
    while !a.is_zero() {
        let (q, r) = b.divide_with_q_and_r(&a)?;
        b = a.into();
        a = r.into();
        let y1old = y1;
        y1 = &y0 - &(&q * &y1old);
        y0 = y1old;
        let x1old = x1;
        x1 = &x0 - &(&q * &x1old);
        x0 = x1old;
    }
    Some((b.into(), x0, y0))
}

// Ref: https://github.com/blynn/pbc/blob/fbf4589036ce4f662e2d06905862c9e816cf9d08/arith/field.c#L251-L330
pub struct FixedBaseCurvePow<G: ProjectiveCurve> {
    table: Vec<Vec<G>>,
}

impl<G: ProjectiveCurve> FixedBaseCurvePow<G> {
    const K: usize = 5;

    pub fn build(base: &G) -> Self {
        let bits =
            <<G as ProjectiveCurve>::ScalarField as PrimeField>::Params::MODULUS_BITS as usize;
        let num_lookups = bits / Self::K + 1;
        let lookup_size = (1 << Self::K) - 1;
        let last_lookup_size = (1 << (bits - (num_lookups - 1) * Self::K)) - 1;

        let mut table: Vec<Vec<G>> = Vec::with_capacity(num_lookups);

        let mut multiplier = *base;
        for i in 0..num_lookups {
            let table_size = if i == num_lookups - 1 {
                last_lookup_size
            } else {
                lookup_size
            };
            let sub_table: Vec<G> = unfold(multiplier, |last| {
                let ret = *last;
                last.add_assign(&multiplier);
                Some(ret)
            })
            .take(table_size)
            .collect();
            table.push(sub_table);
            if i != num_lookups - 1 {
                let last = *table.last().unwrap().last().unwrap();
                multiplier.add_assign(&last);
            }
        }
        Self { table }
    }

    pub fn apply(&self, input: &<G as ProjectiveCurve>::ScalarField) -> G {
        let mut res = G::zero();
        let input_repr = input.into_repr();
        let num_lookups = input_repr.num_bits() as usize / Self::K + 1;
        for i in 0..num_lookups {
            let mut word: usize = 0;
            for j in 0..Self::K {
                if input_repr.get_bit(i * Self::K + j) {
                    word |= 1 << j;
                }
            }
            if word > 0 {
                res.add_assign(&self.table[i][word - 1]);
            }
        }
        res
    }
}

pub struct FixedBaseScalarPow<F: PrimeField> {
    table: Vec<Vec<F>>,
}

impl<F: PrimeField> FixedBaseScalarPow<F> {
    const K: usize = 8;

    pub fn build(base: &F) -> Self {
        let bits = <F as PrimeField>::Params::MODULUS_BITS as usize;
        let num_lookups = bits / Self::K + 1;
        let lookup_size = (1 << Self::K) - 1;
        let last_lookup_size = (1 << (bits - (num_lookups - 1) * Self::K)) - 1;

        let mut table: Vec<Vec<F>> = Vec::with_capacity(num_lookups);

        let mut multiplier = *base;
        for i in 0..num_lookups {
            let table_size = if i == num_lookups - 1 {
                last_lookup_size
            } else {
                lookup_size
            };
            let sub_table: Vec<F> = unfold(multiplier, |last| {
                let ret = *last;
                last.mul_assign(&multiplier);
                Some(ret)
            })
            .take(table_size)
            .collect();
            table.push(sub_table);
            if i != num_lookups - 1 {
                let last = *table.last().unwrap().last().unwrap();
                multiplier.mul_assign(&last);
            }
        }
        Self { table }
    }

    pub fn apply(&self, input: &F) -> F {
        let mut res = F::one();
        let input_repr = input.into_repr();
        let num_lookups = input_repr.num_bits() as usize / Self::K + 1;
        for i in 0..num_lookups {
            let mut word: usize = 0;
            for j in 0..Self::K {
                if input_repr.get_bit(i * Self::K + j) {
                    word |= 1 << j;
                }
            }
            if word > 0 {
                res.mul_assign(&self.table[i][word - 1]);
            }
        }
        res
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ark_bls12_381::{Fr, G1Projective, G2Projective};
    use ark_ff::Field;
    use ark_poly::Polynomial;
    use core::ops::MulAssign;
    use rand::Rng;

    #[test]
    fn test_xgcd() {
        let poly1 = DensePolynomial::from_coefficients_vec(vec![Fr::from(1u32), Fr::from(1u32)]);
        let poly2 = DensePolynomial::from_coefficients_vec(vec![Fr::from(2u32), Fr::from(1u32)]);
        let (g, x, y) = xgcd(&poly1, &poly2).unwrap();
        assert_eq!(g.degree(), 0);
        let gcd = &(&poly1 * &x) + &(&poly2 * &y);
        assert_eq!(gcd, g);
    }

    #[test]
    fn test_pow_g1() {
        let g1p = FixedBaseCurvePow::build(&G1Projective::prime_subgroup_generator());
        let mut rng = rand::thread_rng();
        let num: Fr = rng.gen();
        let mut expect = G1Projective::prime_subgroup_generator();
        expect.mul_assign(num);
        assert_eq!(g1p.apply(&num), expect);
    }

    #[test]
    fn test_pow_g2() {
        let g2p = FixedBaseCurvePow::build(&G2Projective::prime_subgroup_generator());
        let mut rng = rand::thread_rng();
        let num: Fr = rng.gen();
        let mut expect = G2Projective::prime_subgroup_generator();
        expect.mul_assign(num);
        assert_eq!(g2p.apply(&num), expect);
    }

    #[test]
    fn test_pow_fr() {
        let mut rng = rand::thread_rng();
        let base: Fr = rng.gen();
        let num: Fr = rng.gen();
        let frp = FixedBaseScalarPow::build(&base);
        let expect = base.pow(num.into_repr());
        assert_eq!(frp.apply(&num), expect);
    }
}
