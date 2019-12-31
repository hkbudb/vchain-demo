use super::{field::Fr, utils::digest_to_fr};
use crate::set::{MultiSet, SetElement};
use algebra::Field;
use ff_fft::DensePolynomial;
use rayon::{self, prelude::*};
use std::ops::Deref;

#[derive(Debug, Clone, Default)]
pub struct DigestSet {
    pub(crate) inner: Vec<(Fr, u32)>,
}

impl DigestSet {
    pub fn new<T: SetElement>(input: &MultiSet<T>) -> Self {
        let mut inner: Vec<(Fr, u32)> = Vec::with_capacity(input.len());
        (0..input.len())
            .into_par_iter()
            .map(|i| {
                let (k, v) = input.iter().nth(i).unwrap();
                let d = k.to_digest();
                (digest_to_fr(&d), *v)
            })
            .collect_into_vec(&mut inner);
        Self { inner }
    }

    pub fn expand_to_poly(&self) -> DensePolynomial<Fr> {
        let mut inputs = Vec::new();
        for (k, v) in &self.inner {
            for _ in 0..*v {
                inputs.push(DensePolynomial::from_coefficients_vec(vec![*k, Fr::one()]));
            }
        }
        fn expand(polys: &[DensePolynomial<Fr>]) -> DensePolynomial<Fr> {
            if polys.is_empty() {
                return DensePolynomial::from_coefficients_vec(vec![Fr::one()]);
            } else if polys.len() == 1 {
                return polys[0].clone();
            }
            let mid = polys.len() / 2;
            let (left, right) = rayon::join(|| expand(&polys[..mid]), || expand(&polys[mid..]));
            &left * &right
        }
        expand(&inputs)
    }
}

impl Deref for DigestSet {
    type Target = Vec<(Fr, u32)>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_digest_to_poly() {
        let set = DigestSet {
            inner: vec![
                (Fr::from(1u32), 2),
                (Fr::from(2u32), 1),
                (Fr::from(3u32), 1),
            ],
        };
        let expect = DensePolynomial::from_coefficients_vec(vec![
            Fr::from(6u32),
            Fr::from(17u32),
            Fr::from(17u32),
            Fr::from(7u32),
            Fr::from(1u32),
        ]);
        assert_eq!(set.expand_to_poly(), expect);
    }
}
