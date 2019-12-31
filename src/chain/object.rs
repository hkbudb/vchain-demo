use crate::acc::{self, curve::G1Affine, Accumulator};
use crate::digest::{blake2, Digest, Digestable};
use crate::set::MultiSet;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Debug, Default, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct RawObject {
    pub block_id: u64,
    pub v_data: Vec<u32>,
    pub w_data: HashSet<String>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Object {
    pub id: u64,
    pub block_id: u64,
    pub v_data: Vec<u32>,
    pub w_data: HashSet<String>,
    pub set_data: MultiSet<SetElementType>,
    #[serde(with = "crate::acc::serde_impl")]
    pub acc_value: G1Affine,
}

impl Object {
    pub fn create(obj: &RawObject, v_bit_len: &[u32], acc_type: acc::Type, use_sk: bool) -> Self {
        static OBJECT_ID_CNT: AtomicU64 = AtomicU64::new(0);
        let id = OBJECT_ID_CNT.fetch_add(1, Ordering::SeqCst);
        let set_data = obj
            .w_data
            .iter()
            .map(|w| SetElementType::W(w.clone()))
            .collect::<MultiSet<_>>()
            + v_data_to_set(&obj.v_data, v_bit_len);
        let acc_value = match (acc_type, use_sk) {
            (acc::Type::ACC1, true) => acc::Acc1::cal_acc_g1_sk(&set_data),
            (acc::Type::ACC1, false) => acc::Acc1::cal_acc_g1(&set_data),
            (acc::Type::ACC2, true) => acc::Acc2::cal_acc_g1_sk(&set_data),
            (acc::Type::ACC2, false) => acc::Acc2::cal_acc_g1(&set_data),
        };
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

impl Digestable for Object {
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
    V { dim: u32, val: u32, mask: u32 },
    W(String),
}

impl Digestable for SetElementType {
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

pub fn v_data_to_set(input: &[u32], bit_len: &[u32]) -> MultiSet<SetElementType> {
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
