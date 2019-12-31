use crate::acc;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub mod object;
pub use object::*;

pub mod index;
pub use index::*;

#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct Parameter {
    v_bit_len: Vec<u8>,
    acc_type: acc::Type,
    use_sk: bool, // only for debug purpose
    intra_index: bool,
    skip_list_max_level: u16,
}

pub trait ReadInterface {
    fn get_parameter(&self) -> Result<Parameter>;
    fn read_block_header(&self, id: u64) -> Result<BlockHeader>;
    fn read_intra_index_node(&self, id: u64) -> Result<IntraIndexNode>;
    fn read_skip_list_node(&self, id: u64) -> Result<SkipListNode>;
    fn read_object(&self, id: u64) -> Result<Object>;
}

pub trait WriteInterface {
    fn set_parameter(&mut self, param: Parameter) -> Result<()>;
    fn write_block_header(&mut self, header: BlockHeader) -> Result<()>;
    fn write_intra_index_node(&mut self, node: IntraIndexNode) -> Result<()>;
    fn write_skip_list_node(&mut self, node: SkipListNode) -> Result<()>;
    fn write_object(&mut self, obj: Object) -> Result<()>;
}

#[derive(Debug, Default)]
pub struct FakeInMemChain {
    param: Option<Parameter>,
    block_headers: HashMap<u64, BlockHeader>,
    intra_index_nodes: HashMap<u64, IntraIndexNode>,
    skip_list_nodes: HashMap<u64, SkipListNode>,
    objects: HashMap<u64, Object>,
}

impl ReadInterface for FakeInMemChain {
    fn get_parameter(&self) -> Result<Parameter> {
        self.param.clone().context("failed to get param")
    }
    fn read_block_header(&self, id: u64) -> Result<BlockHeader> {
        self.block_headers
            .get(&id)
            .cloned()
            .context("failed to read block")
    }
    fn read_intra_index_node(&self, id: u64) -> Result<IntraIndexNode> {
        self.intra_index_nodes
            .get(&id)
            .cloned()
            .context("failed to read intra index")
    }
    fn read_skip_list_node(&self, id: u64) -> Result<SkipListNode> {
        self.skip_list_nodes
            .get(&id)
            .cloned()
            .context("failed to read skip list")
    }
    fn read_object(&self, id: u64) -> Result<Object> {
        self.objects
            .get(&id)
            .cloned()
            .context("failed to read object")
    }
}

impl WriteInterface for FakeInMemChain {
    fn set_parameter(&mut self, param: Parameter) -> Result<()> {
        self.param = Some(param);
        Ok(())
    }
    fn write_block_header(&mut self, header: BlockHeader) -> Result<()> {
        let id = header.block_id;
        self.block_headers.insert(id, header);
        Ok(())
    }
    fn write_intra_index_node(&mut self, node: IntraIndexNode) -> Result<()> {
        let id = node.id();
        self.intra_index_nodes.insert(id, node);
        Ok(())
    }
    fn write_skip_list_node(&mut self, node: SkipListNode) -> Result<()> {
        let id = node.id;
        self.skip_list_nodes.insert(id, node);
        Ok(())
    }
    fn write_object(&mut self, obj: Object) -> Result<()> {
        let id = obj.id;
        self.objects.insert(id, obj);
        Ok(())
    }
}
