use super::{IdType, SetElementType};
use crate::set::{MultiSet, SetElement};
use anyhow::{bail, Context, Error, Result};
use core::iter::FromIterator;
use core::ops::Deref;
use serde::{Deserialize, Serialize};
use serde_json::{json, value::Value as JsonValue};
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
    pub fn from_json(input: &JsonValue) -> Result<Self> {
        let err_msg = format!("invald input: {:?}", input);
        let p1 = input
            .get(0)
            .context(err_msg.clone())?
            .as_array()
            .context(err_msg.clone())?
            .iter()
            .map(|v| v.as_u64().map(|v| v as u32))
            .collect::<Vec<_>>();
        let p2 = input
            .get(1)
            .context(err_msg.clone())?
            .as_array()
            .context(err_msg)?
            .iter()
            .map(|v| v.as_u64().map(|v| v as u32))
            .collect::<Vec<_>>();
        Ok(Self([p1, p2]))
    }

    pub fn to_json(&self) -> JsonValue {
        let p1: Vec<JsonValue> = self.0[0]
            .iter()
            .map(|v| match v {
                Some(v) => json!(v),
                None => json!(null),
            })
            .collect();
        let p2: Vec<JsonValue> = self.0[1]
            .iter()
            .map(|v| match v {
                Some(v) => json!(v),
                None => json!(null),
            })
            .collect();
        json!([p1, p2])
    }

    pub fn to_bool_exp(&self, bit_len: &[u8]) -> BoolExp<SetElementType> {
        let mut exp = BoolExp::new();
        for (i, range) in self[0].iter().zip(self[1].iter()).enumerate() {
            let (l, r) = match (range.0, range.1) {
                (Some(x), Some(y)) => (*x, *y),
                _ => continue,
            };

            let mut set_data = MultiSet::<SetElementType>::new();

            let mut queue: VecDeque<(u32, u32)> = VecDeque::new();
            queue.push_back((0, 0));

            while let Some((mut mask, left)) = queue.pop_front() {
                let mask_inv = !mask;
                let right = left | mask_inv;

                if l <= left && right <= r {
                    if bit_len[i] < 32 {
                        mask &= !(0xffff_ffff << bit_len[i]);
                    }
                    set_data.inner.insert(
                        SetElementType::V {
                            dim: i as u32,
                            val: left,
                            mask,
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
    pub fn from_json(input: &JsonValue) -> Result<Self> {
        let err_msg = format!("invald input: {:?}", input);
        let start_block = input
            .get("start_block")
            .context(err_msg.clone())?
            .as_u64()
            .context(err_msg.clone())? as IdType;
        let end_block = input
            .get("end_block")
            .context(err_msg.clone())?
            .as_u64()
            .context(err_msg.clone())? as IdType;
        let q_range = match input.get("range") {
            Some(range) => Some(Range::from_json(range)?),
            None => None,
        };
        let q_bool = match input.get("bool") {
            Some(exp) => {
                let parsed_exp = match exp.as_array() {
                    Some(exp_array) => exp_array
                        .iter()
                        .map(|sub_array| match sub_array.as_array() {
                            Some(sub_array) => sub_array
                                .iter()
                                .map(|s| s.as_str().map(|s| s.to_owned()).context(err_msg.clone()))
                                .collect::<Result<HashSet<_>>>(),
                            None => Err(Error::msg(err_msg.clone())),
                        })
                        .collect::<Result<Vec<_>>>()?,
                    None => bail!(err_msg),
                };
                Some(parsed_exp)
            }
            None => None,
        };
        Ok(Self {
            start_block,
            end_block,
            q_range,
            q_bool,
        })
    }

    pub fn to_json(&self) -> JsonValue {
        let mut res = json!({
            "start_block": self.start_block,
            "end_block": self.end_block,
        });
        if let Some(q_range) = &self.q_range {
            res["range"] = q_range.to_json();
        }
        if let Some(q_bool) = &self.q_bool {
            res["bool"] = json!(q_bool
                .iter()
                .map(|sub_exp| {
                    let mut sub_exp = sub_exp.iter().collect::<Vec<_>>();
                    sub_exp.sort_unstable();
                    sub_exp.iter().map(|w| json!(w)).collect::<Vec<_>>()
                })
                .collect::<Vec<_>>());
        }
        res
    }

    pub fn to_bool_exp(&self, bit_len: &[u8]) -> BoolExp<SetElementType> {
        let mut exp = BoolExp::new();
        if let Some(q_range) = &self.q_range {
            exp.inner
                .extend(q_range.to_bool_exp(bit_len).iter().cloned());
        }
        if let Some(q_bool) = &self.q_bool {
            for sub_exp in q_bool.iter() {
                exp.inner.push(MultiSet::from_iter(
                    sub_exp.iter().map(|w| SetElementType::W(w.clone())),
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

    #[test]
    fn test_query() {
        let data = json!({
            "start_block": 1,
            "end_block": 2,
            "range": [
                [0, null, 3],
                [6, null, 4],
            ],
            "bool": [
                ["a", "b"],
                ["c"],
            ],
        });
        let expect = Query {
            start_block: 1,
            end_block: 2,
            q_range: Some(Range([
                vec![Some(0), None, Some(3)],
                vec![Some(6), None, Some(4)],
            ])),
            q_bool: Some(vec![
                ["a".to_owned(), "b".to_owned()]
                    .iter()
                    .cloned()
                    .collect::<HashSet<_>>(),
                ["c".to_owned()].iter().cloned().collect::<HashSet<_>>(),
            ]),
        };
        assert_eq!(Query::from_json(&data).unwrap(), expect);
        assert_eq!(data, expect.to_json());
    }
}
