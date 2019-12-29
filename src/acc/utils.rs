use crate::acc::field::Fr;
use crate::digest::Digest;
use algebra::{BigInteger, Field, FpParameters, PrimeField, ProjectiveCurve};
use itertools::unfold;

pub fn digest_to_fr(input: &Digest) -> Fr {
    let mut repr = Fr::zero().into_repr();
    let mut input_iter = input.0.iter().rev();
    'outer: for limb in &mut repr.0 {
        for i in 0..8 {
            if let Some(&v) = input_iter.next() {
                *limb |= (v as u64) << (i * 8);
            } else {
                break 'outer;
            }
        }
    }
    // drop the last two bits to ensure it is less than the modular
    *repr.0.last_mut().unwrap() &= 0x3fff_ffff_ffff_ffff;
    Fr::from_repr(repr)
}

// Ref: https://github.com/blynn/pbc/blob/fbf4589036ce4f662e2d06905862c9e816cf9d08/arith/field.c#L251-L330

pub struct CurvePow<G: ProjectiveCurve> {
    table: Vec<Vec<G>>,
}

impl<G: ProjectiveCurve> CurvePow<G> {
    const K: usize = 5;

    pub fn build(base: &G) -> Self {
        let bits =
            <<G as ProjectiveCurve>::ScalarField as PrimeField>::Params::MODULUS_BITS as usize;
        let num_lookups = bits / Self::K + 1;
        let lookup_size = (1 << Self::K) - 1;

        let mut table: Vec<Vec<G>> = Vec::with_capacity(num_lookups);

        let mut multiplier = *base;
        for _ in 0..num_lookups {
            let sub_table: Vec<G> = unfold(multiplier, |last| {
                let ret = *last;
                last.add_assign(&multiplier);
                Some(ret)
            })
            .take(lookup_size)
            .collect();
            let last = *sub_table.last().unwrap();
            table.push(sub_table);
            multiplier.add_assign(&last);
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

pub struct FieldPow<F: PrimeField> {
    table: Vec<Vec<F>>,
}

impl<F: PrimeField> FieldPow<F> {
    const K: usize = 8;

    pub fn build(base: &F) -> Self {
        let bits = <F as PrimeField>::Params::MODULUS_BITS as usize;
        let num_lookups = bits / Self::K + 1;
        let lookup_size = (1 << Self::K) - 1;

        let mut table: Vec<Vec<F>> = Vec::with_capacity(num_lookups);

        let mut multiplier = *base;
        for _ in 0..num_lookups {
            let sub_table: Vec<F> = unfold(multiplier, |last| {
                let ret = *last;
                last.mul_assign(&multiplier);
                Some(ret)
            })
            .take(lookup_size)
            .collect();
            let last = *sub_table.last().unwrap();
            table.push(sub_table);
            multiplier.mul_assign(&last);
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
    use rand::Rng;
    use std::str::FromStr;

    #[test]
    fn test_digest_to_fr() {
        let expect = Fr::from_str(
            "27829188660842407121959431004685384086581924103735368447862915877590343257091",
        )
        .unwrap();
        let d = Digest(*b"\xbd\x86\xc3\x39\x7e\x8f\x3a\x9f\xc6\x95\xd1\xba\x57\x40\x86\xa1\x34\x55\x4c\xea\x08\xec\x9c\x9e\x65\xdd\xbb\x5b\x82\x3e\x8c\x03");
        assert_eq!(digest_to_fr(&d), expect);

        let expect = Fr::from_str(
            "28948022309329048855892746252171976963317496166410141009864396001978282409983",
        )
        .unwrap();
        let d = Digest(*b"\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff");
        assert_eq!(digest_to_fr(&d), expect);
    }

    #[test]
    fn test_pow_g1() {
        let g1p = CurvePow::build(&G1::prime_subgroup_generator());
        let mut rng = rand::thread_rng();
        let num: Fr = rng.gen();
        let mut expect = G1::prime_subgroup_generator();
        expect.mul_assign(num);
        assert_eq!(g1p.apply(&num), expect);
    }

    #[test]
    fn test_pow_g2() {
        let g2p = CurvePow::build(&G2::prime_subgroup_generator());
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
        let frp = FieldPow::build(&base);
        let expect = base.pow(num.into_repr());
        assert_eq!(frp.apply(&num), expect);
    }
}
