use super::*;
use anyhow::Context;
use rocksdb::DB;
use std::fs;
use std::path::{Path, PathBuf};

pub struct SimChain {
    root_path: PathBuf,
    param: Parameter,
    block_header_db: DB,
    block_data_db: DB,
    intra_index_db: DB,
    skip_list_db: DB,
    obj_db: DB,
}

impl SimChain {
    pub fn new(path: &Path) -> Result<Self> {
        info!("open db at {:?}", path);
        fs::create_dir_all(path).context(format!("failed to create dir {:?}", path))?;
        Ok(Self {
            root_path: path.to_owned(),
            param: serde_json::from_str::<Parameter>(&fs::read_to_string(
                path.join("param.json"),
            )?)?,
            block_header_db: DB::open_default(path.join("blk_header.db"))?,
            block_data_db: DB::open_default(path.join("blk_data.db"))?,
            intra_index_db: DB::open_default(path.join("intra_index.db"))?,
            skip_list_db: DB::open_default(path.join("skiplist.db"))?,
            obj_db: DB::open_default(path.join("obj.db"))?,
        })
    }

}

impl ReadInterface for SimChain {
    fn get_parameter(&self) -> Result<Parameter> {
        Ok(self.param.clone())
    }
    fn read_block_header(&self, id: IdType) -> Result<BlockHeader> {
        let data = self
            .block_header_db
            .get(id.to_le_bytes())?
            .context("failed to read block header")?;
        Ok(bincode::deserialize::<BlockHeader>(&data[..])?)
    }
    fn read_block_data(&self, id: IdType) -> Result<BlockData> {
        let data = self
            .block_data_db
            .get(id.to_le_bytes())?
            .context("failed to read block data")?;
        Ok(bincode::deserialize::<BlockData>(&data[..])?)
    }
    fn read_intra_index_node(&self, id: IdType) -> Result<IntraIndexNode> {
        let data = self
            .intra_index_db
            .get(id.to_le_bytes())?
            .context("failed to read index node")?;
        Ok(bincode::deserialize::<IntraIndexNode>(&data[..])?)
    }
    fn read_skip_list_node(&self, id: IdType) -> Result<SkipListNode> {
        let data = self
            .skip_list_db
            .get(id.to_le_bytes())?
            .context("failed to read skip list")?;
        Ok(bincode::deserialize::<SkipListNode>(&data[..])?)
    }
    fn read_object(&self, id: IdType) -> Result<Object> {
        let data = self
            .obj_db
            .get(id.to_le_bytes())?
            .context("failed to read object")?;
        Ok(bincode::deserialize::<Object>(&data[..])?)
    }
}

impl WriteInterface for SimChain {
    fn set_parameter(&mut self, param: Parameter) -> Result<()> {
        self.param = param;
        let data = serde_json::to_string_pretty(&self.param)?;
        fs::write(self.root_path.join("param.json"), data)?;
        Ok(())
    }
    fn write_block_header(&mut self, header: BlockHeader) -> Result<()> {
        let bytes = bincode::serialize(&header)?;
        self.block_header_db
            .put(header.block_id.to_le_bytes(), bytes)?;
        Ok(())
    }
    fn write_block_data(&mut self, data: BlockData) -> Result<()> {
        let bytes = bincode::serialize(&data)?;
        self.block_data_db.put(data.block_id.to_le_bytes(), bytes)?;
        Ok(())
    }
    fn write_intra_index_node(&mut self, node: IntraIndexNode) -> Result<()> {
        let bytes = bincode::serialize(&node)?;
        self.intra_index_db.put(node.id().to_le_bytes(), bytes)?;
        Ok(())
    }
    fn write_skip_list_node(&mut self, node: SkipListNode) -> Result<()> {
        let bytes = bincode::serialize(&node)?;
        self.skip_list_db.put(node.id.to_le_bytes(), bytes)?;
        Ok(())
    }
    fn write_object(&mut self, obj: Object) -> Result<()> {
        let bytes = bincode::serialize(&obj)?;
        self.obj_db.put(obj.id.to_le_bytes(), bytes)?;
        Ok(())
    }
}
