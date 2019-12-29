use crate::acc::field::Fr;
use crate::acc::utils::digest_to_fr;
use crate::set::{MultiSet, SetElement};
use rayon::prelude::*;

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

    pub fn expand_to_poly(&self) {}
}
