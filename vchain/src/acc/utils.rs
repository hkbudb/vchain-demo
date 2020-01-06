use super::field::Fr;
use crate::digest::Digest;
use algebra::{BigInteger, FpParameters, PrimeField, ProjectiveCurve};
use ff_fft::{DenseOrSparsePolynomial, DensePolynomial};
use itertools::unfold;

pub fn digest_to_fr(input: &Digest) -> Fr {
    let mut data = input.0;
    // drop the last two bits to ensure it is less than the modular
    *data.last_mut().unwrap() &= 0x3f;
    let mut num = Fr::from_random_bytes(&data).unwrap().into_repr();
    // ensure the Fr is at most in 248 bits. so PUB_Q - Fr and Fr + PUB_Q - Fr never overflow.
    num.0[3] &= 0x00ff_ffff_ffff_ffff;
    Fr::from_repr(num)
}

/// Return (g, x, y) s.t. a*x + b*y = g = gcd(a, b)
pub fn xgcd<F: PrimeField>(
    mut a: DensePolynomial<F>,
    mut b: DensePolynomial<F>,
) -> Option<(DensePolynomial<F>, DensePolynomial<F>, DensePolynomial<F>)> {
    let mut x0 = DensePolynomial::<F>::zero();
    let mut x1 = DensePolynomial::<F>::from_coefficients_vec(vec![F::one()]);
    let mut y0 = DensePolynomial::<F>::from_coefficients_vec(vec![F::one()]);
    let mut y1 = DensePolynomial::<F>::zero();
    while !a.is_zero() {
        let a_poly: DenseOrSparsePolynomial<F> = a.clone().into();
        let b_poly: DenseOrSparsePolynomial<F> = b.clone().into();
        let (q, r) = b_poly.divide_with_q_and_r(&a_poly)?;
        b = a;
        a = r;
        let y1old = y1.clone();
        y1 = &y0 - &(&q * &y1);
        y0 = y1old;
        let x1old = x1.clone();
        x1 = &x0 - &(&q * &x1);
        x0 = x1old;
    }
    Some((b, x0, y0))
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
    use crate::acc::curve::{G1Projective as G1, G2Projective as G2};
    use algebra::Field;
    use rand::Rng;

    #[test]
    fn test_xgcd() {
        let poly1 = DensePolynomial::from_coefficients_vec(vec![Fr::from(1u32), Fr::from(1u32)]);
        let poly2 = DensePolynomial::from_coefficients_vec(vec![Fr::from(2u32), Fr::from(1u32)]);
        let (g, x, y) = xgcd(poly1.clone(), poly2.clone()).unwrap();
        assert_eq!(g.degree(), 0);
        let mut gcd = &(&poly1 * &x) + &(&poly2 * &y);
        while gcd.coeffs.last().map_or(false, |c| c.is_zero()) {
            gcd.coeffs.pop();
        }
        assert_eq!(gcd, g);
    }

    #[test]
    fn test_pow_g1() {
        let g1p = FixedBaseCurvePow::build(&G1::prime_subgroup_generator());
        let mut rng = rand::thread_rng();
        let num: Fr = rng.gen();
        let mut expect = G1::prime_subgroup_generator();
        expect.mul_assign(num);
        assert_eq!(g1p.apply(&num), expect);
    }

    #[test]
    fn test_pow_g2() {
        let g2p = FixedBaseCurvePow::build(&G2::prime_subgroup_generator());
        let mut rng = rand::thread_rng();
        let num: Fr = rng.gen();
        let mut expect = G2::prime_subgroup_generator();
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
