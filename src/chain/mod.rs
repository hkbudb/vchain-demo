use crate::acc;
use anyhow::Result;
use serde::{Deserialize, Serialize};

pub mod utils;
pub use utils::*;

pub mod object;
pub use object::*;

pub mod index;
pub use index::*;

pub mod build;
pub use build::*;

pub mod query;
pub use query::*;

pub mod query_result;
pub use query_result::*;

pub mod historical_query;
pub use historical_query::*;

pub mod sim_chain;
pub use sim_chain::*;

pub type IdType = u32;
pub type SkipLstLvlType = u8;

#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct Parameter {
    pub v_bit_len: Vec<u8>,
    pub acc_type: acc::Type,
    pub use_sk: bool, // only for debug purpose
    pub intra_index: bool,
    pub skip_list_max_level: SkipLstLvlType,
}

pub trait ReadInterface {
    fn get_parameter(&self) -> Result<Parameter>;
    fn read_block_header(&self, id: IdType) -> Result<BlockHeader>;
    fn read_block_data(&self, id: IdType) -> Result<BlockData>;
    fn read_intra_index_node(&self, id: IdType) -> Result<IntraIndexNode>;
    fn read_skip_list_node(&self, id: IdType) -> Result<SkipListNode>;
    fn read_object(&self, id: IdType) -> Result<Object>;
}

pub trait WriteInterface {
    fn set_parameter(&mut self, param: Parameter) -> Result<()>;
    fn write_block_header(&mut self, header: BlockHeader) -> Result<()>;
    fn write_block_data(&mut self, data: BlockData) -> Result<()>;
    fn write_intra_index_node(&mut self, node: IntraIndexNode) -> Result<()>;
    fn write_skip_list_node(&mut self, node: SkipListNode) -> Result<()>;
    fn write_object(&mut self, obj: Object) -> Result<()>;
}

#[cfg(test)]
mod tests;
