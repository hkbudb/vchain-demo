use super::{IdType, SetElementType};
use crate::set::{MultiSet, SetElement};
use core::iter::FromIterator;
use core::ops::Deref;
use serde::{Deserialize, Serialize};
use std::collections::{HashSet, VecDeque};

#[derive(Debug, Default, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct BoolExp<T: SetElement> {
    pub(crate) inner: Vec<MultiSet<T>>,
}

impl<T: SetElement> BoolExp<T> {
    pub fn new() -> Self {
        Self { inner: Vec::new() }
    }

    pub fn from_vec(input: Vec<MultiSet<T>>) -> Self {
        Self::from_iter(input.into_iter())
    }

    pub fn is_match(&self, set: &MultiSet<T>) -> bool {
        self.mismatch_idx(set).is_none()
    }

    pub fn mismatch_idx(&self, set: &MultiSet<T>) -> Option<usize> {
        self.iter().position(|s| !s.is_intersected_with(set))
    }
}

impl<T: SetElement> Deref for BoolExp<T> {
    type Target = Vec<MultiSet<T>>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T: SetElement> FromIterator<MultiSet<T>> for BoolExp<T> {
    fn from_iter<I: IntoIterator<Item = MultiSet<T>>>(iter: I) -> Self {
        Self {
            inner: iter.into_iter().collect::<Vec<_>>(),
        }
    }
}

#[derive(Debug, Default, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct Range(pub(crate) [Vec<Option<u32>>; 2]);

impl Range {
    pub fn to_bool_exp(&self, bit_len: &[u8]) -> BoolExp<SetElementType> {
        let mut exp = BoolExp::new();
        let dim = self[0].len();
        for i in 0..dim {
            let (l, r) = match (self[0][i], self[1][i]) {
                (Some(x), Some(y)) => (x, y),
                _ => continue,
            };

            let mut set_data = MultiSet::<SetElementType>::new();

            let mut queue: VecDeque<(u32, u32)> = VecDeque::new();
            queue.push_back((0, 0));

            while let Some((mask, left)) = queue.pop_front() {
                let mask_inv = !mask;
                let right = left | mask_inv;

                if l <= left && right <= r {
                    set_data.inner.insert(
                        SetElementType::V {
                            dim: i as u32,
                            val: left,
                            mask: mask & !(0xffff_ffff << bit_len[i]),
                        },
                        1,
                    );
                    continue;
                }

                if right < l || r < left {
                    continue;
                }

                let new_mask = !(mask_inv >> 1);
                queue.push_back((new_mask, left));
                queue.push_back((new_mask, left | (new_mask & mask_inv)));
            }

            exp.inner.push(set_data);
        }
        exp
    }
}

impl Deref for Range {
    type Target = [Vec<Option<u32>>; 2];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug, Default, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Query {
    pub start_block: IdType,
    pub end_block: IdType,
    pub q_range: Option<Range>,
    pub q_bool: Option<Vec<HashSet<String>>>,
}

impl Query {
    pub fn to_bool_exp(&self, bit_len: &[u8]) -> BoolExp<SetElementType> {
        let mut exp = BoolExp::new();
        if let Some(q_range) = &self.q_range {
            exp.inner
                .extend(q_range.to_bool_exp(bit_len).iter().cloned());
        }
        if let Some(q_bool) = &self.q_bool {
            for s in q_bool.iter() {
                exp.inner.push(MultiSet::from_iter(
                    s.iter().map(|w| SetElementType::W(w.clone())),
                ));
            }
        }
        exp
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_boolexp() {
        let exp = BoolExp::from_vec(vec![
            MultiSet::from_vec(vec!["a".to_owned(), "b".to_owned()]),
            MultiSet::from_vec(vec!["c".to_owned()]),
        ]);
        let set1 = MultiSet::from_vec(vec!["a".to_owned(), "b".to_owned()]);
        let set2 = MultiSet::from_vec(vec!["a".to_owned(), "b".to_owned(), "c".to_owned()]);
        assert_eq!(exp.mismatch_idx(&set1), Some(1));
        assert!(exp.is_match(&set2));
    }

    #[test]
    fn test_range() {
        use SetElementType::V;

        let range = Range([vec![Some(0), None, Some(3)], vec![Some(6), None, Some(4)]]);
        #[rustfmt::skip]
        let expect = BoolExp::from_vec(vec![
            MultiSet::from_vec(vec![
                V { dim: 0, val: 0b000, mask: 0b100 },
                V { dim: 0, val: 0b100, mask: 0b110 },
                V { dim: 0, val: 0b110, mask: 0b111 },
            ]),
            MultiSet::from_vec(vec![
                V { dim: 2, val: 0b011, mask: 0b111 },
                V { dim: 2, val: 0b100, mask: 0b111 },
            ]),
        ]);
        assert_eq!(range.to_bool_exp(&[3, 3, 3]), expect);
    }
}
