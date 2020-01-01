use crate::acc::{
    self,
    curve::{G1Affine, G2Affine},
    Accumulator,
};
use crate::set::MultiSet;
use anyhow::Result;
use serde::{Deserialize, Serialize};

pub mod object;
pub use object::*;

pub mod index;
pub use index::*;

pub mod build;
pub use build::*;

pub mod query;
pub use query::*;

#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct Parameter {
    pub v_bit_len: Vec<u8>,
    pub acc_type: acc::Type,
    pub use_sk: bool, // only for debug purpose
    pub intra_index: bool,
    pub skip_list_max_level: u16,
}

pub trait ReadInterface {
    fn get_parameter(&self) -> Result<Parameter>;
    fn read_block_header(&self, id: u64) -> Result<BlockHeader>;
    fn read_block_data(&self, id: u64) -> Result<BlockData>;
    fn read_intra_index_node(&self, id: u64) -> Result<IntraIndexNode>;
    fn read_skip_list_node(&self, id: u64) -> Result<SkipListNode>;
    fn read_object(&self, id: u64) -> Result<Object>;
}

pub trait WriteInterface {
    fn set_parameter(&mut self, param: Parameter) -> Result<()>;
    fn write_block_header(&mut self, header: BlockHeader) -> Result<()>;
    fn write_block_data(&mut self, data: BlockData) -> Result<()>;
    fn write_intra_index_node(&mut self, node: IntraIndexNode) -> Result<()>;
    fn write_skip_list_node(&mut self, node: SkipListNode) -> Result<()>;
    fn write_object(&mut self, obj: Object) -> Result<()>;
}

pub fn multiset_to_g1(set: &MultiSet<SetElementType>, param: &Parameter) -> G1Affine {
    match (param.acc_type, param.use_sk) {
        (acc::Type::ACC1, true) => acc::Acc1::cal_acc_g1_sk(&set),
        (acc::Type::ACC1, false) => acc::Acc1::cal_acc_g1(&set),
        (acc::Type::ACC2, true) => acc::Acc2::cal_acc_g1_sk(&set),
        (acc::Type::ACC2, false) => acc::Acc2::cal_acc_g1(&set),
    }
}

pub fn multiset_to_g2(set: &MultiSet<SetElementType>, param: &Parameter) -> G2Affine {
    match (param.acc_type, param.use_sk) {
        (acc::Type::ACC1, true) => acc::Acc1::cal_acc_g2_sk(&set),
        (acc::Type::ACC1, false) => acc::Acc1::cal_acc_g2(&set),
        (acc::Type::ACC2, true) => acc::Acc2::cal_acc_g2_sk(&set),
        (acc::Type::ACC2, false) => acc::Acc2::cal_acc_g2(&set),
    }
}

#[cfg(test)]
mod tests;
