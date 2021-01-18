use super::{multiset_to_g1, IdType, Parameter};
use crate::acc::G1Affine;
use crate::digest::{blake2, Digest, Digestible};
use crate::set::MultiSet;
use core::sync::atomic::{AtomicU64, Ordering};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

static OBJECT_ID_CNT: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Default, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct RawObject {
    pub block_id: IdType,
    pub v_data: Vec<u32>,
    pub w_data: HashSet<String>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Object {
    pub id: IdType,
    pub block_id: IdType,
    pub v_data: Vec<u32>,
    pub w_data: HashSet<String>,
    pub set_data: MultiSet<SetElementType>,
    #[serde(with = "crate::acc::serde_impl")]
    pub acc_value: G1Affine,
}

impl Object {
    pub fn create(obj: &RawObject, param: &Parameter) -> Self {
        let id = OBJECT_ID_CNT.fetch_add(1, Ordering::SeqCst) as IdType;
        let set_v = v_data_to_set(&obj.v_data, &param.v_bit_len);
        let set_w = obj
            .w_data
            .iter()
            .map(|w| SetElementType::W(w.clone()))
            .collect::<MultiSet<_>>();
        let set_data = &set_v + &set_w;
        let acc_value = multiset_to_g1(&set_data, param);
        Self {
            id,
            block_id: obj.block_id,
            v_data: obj.v_data.clone(),
            w_data: obj.w_data.clone(),
            set_data,
            acc_value,
        }
    }
}

impl Digestible for Object {
    fn to_digest(&self) -> Digest {
        let mut state = blake2().to_state();
        state.update(&self.id.to_le_bytes());
        state.update(&self.block_id.to_le_bytes());
        for v in &self.v_data {
            state.update(&v.to_le_bytes());
        }
        let mut ws: Vec<_> = self.w_data.iter().collect();
        ws.par_sort_unstable();
        for w in &ws {
            state.update(w.as_bytes());
        }
        Digest::from(state.finalize())
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum SetElementType {
    // To transform V to range: [val, val + ~mask & (mask - 1)]
    V { dim: u32, val: u32, mask: u32 },
    W(String),
}

impl Digestible for SetElementType {
    fn to_digest(&self) -> Digest {
        match self {
            SetElementType::V { dim, val, mask } => {
                let mut state = blake2().to_state();
                state.update(&dim.to_le_bytes());
                state.update(&val.to_le_bytes());
                state.update(&mask.to_le_bytes());
                Digest::from(state.finalize())
            }
            SetElementType::W(s) => s.to_digest(),
        }
    }
}

pub fn v_data_to_set(input: &[u32], bit_len: &[u8]) -> MultiSet<SetElementType> {
    input
        .iter()
        .enumerate()
        .flat_map(|(i, &v)| {
            let m: u32 = !(0xffff_ffff << bit_len[i]);
            (0..bit_len[i]).map(move |j| {
                let mask = (0xffff_ffff << j) & m;
                let val = v & mask;
                SetElementType::V {
                    dim: i as u32,
                    val,
                    mask,
                }
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_v_data_to_set() {
        use SetElementType::V;
        let res = v_data_to_set(&[4, 2], &[3, 3]);
        #[rustfmt::skip]
        let expect = MultiSet::from_vec(vec![
            V { dim: 0, val: 0b100, mask: 0b100 },
            V { dim: 0, val: 0b100, mask: 0b110 },
            V { dim: 0, val: 0b100, mask: 0b111 },
            V { dim: 1, val: 0b000, mask: 0b100 },
            V { dim: 1, val: 0b010, mask: 0b110 },
            V { dim: 1, val: 0b010, mask: 0b111 },
        ]);
        assert_eq!(res, expect)
    }
}
