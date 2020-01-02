use super::*;
use anyhow::Context;
use std::collections::HashMap;

#[derive(Debug, Default)]
struct FakeInMemChain {
    param: Option<Parameter>,
    block_headers: HashMap<IdType, BlockHeader>,
    block_data: HashMap<IdType, BlockData>,
    intra_index_nodes: HashMap<IdType, IntraIndexNode>,
    skip_list_nodes: HashMap<IdType, SkipListNode>,
    objects: HashMap<IdType, Object>,
}

impl ReadInterface for FakeInMemChain {
    fn get_parameter(&self) -> Result<Parameter> {
        self.param.clone().context("failed to get param")
    }
    fn read_block_header(&self, id: IdType) -> Result<BlockHeader> {
        self.block_headers
            .get(&id)
            .cloned()
            .context("failed to read block header")
    }
    fn read_block_data(&self, id: IdType) -> Result<BlockData> {
        self.block_data
            .get(&id)
            .cloned()
            .context("failed to read block data")
    }
    fn read_intra_index_node(&self, id: IdType) -> Result<IntraIndexNode> {
        self.intra_index_nodes
            .get(&id)
            .cloned()
            .context("failed to read intra index")
    }
    fn read_skip_list_node(&self, id: IdType) -> Result<SkipListNode> {
        self.skip_list_nodes
            .get(&id)
            .cloned()
            .context("failed to read skip list")
    }
    fn read_object(&self, id: IdType) -> Result<Object> {
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
    fn write_block_data(&mut self, data: BlockData) -> Result<()> {
        let id = data.block_id;
        self.block_data.insert(id, data);
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
