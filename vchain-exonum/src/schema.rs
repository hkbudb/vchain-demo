use crate::transactions::RawObject;
use anyhow::{Context, Error, Result};
use exonum::crypto::Hash;
use exonum_derive::{BinaryValue, FromAccess, ObjectHash};
use exonum_merkledb::{
    access::{Access, RawAccessMut},
    Entry, ListIndex, MapIndex, ObjectHash as _, ProofMapIndex,
};
use exonum_proto::ProtobufConvert;
use vchain::IdType;

use super::proto;

macro_rules! impl_schema_from_proto {
    ($type:ident) => {
        #[derive(Clone, Debug, Serialize, Deserialize, ProtobufConvert, BinaryValue, ObjectHash)]
        #[protobuf_convert(source = "proto::Parameter")]
        pub struct $type {
            pub data: Vec<u8>,
        }

        impl $type {
            pub fn create(input: &vchain::$type) -> Result<Self> {
                Ok(Self {
                    data: bincode::serialize(input)?,
                })
            }

            pub fn to_vchain_type(&self) -> Result<vchain::$type> {
                bincode::deserialize::<vchain::$type>(&self.data).map_err(Error::msg)
            }
        }
    };
}

impl_schema_from_proto!(Parameter);
impl_schema_from_proto!(Object);
impl_schema_from_proto!(BlockHeader);
impl_schema_from_proto!(BlockData);
impl_schema_from_proto!(IntraIndexNode);
impl_schema_from_proto!(SkipListNode);

#[derive(Debug, FromAccess)]
pub(crate) struct VChainSchema<T: Access> {
    pub param: Entry<T::Base, Parameter>,
    pub objects: MapIndex<T::Base, IdType, Object>,
    pub block_headers: ProofMapIndex<T::Base, IdType, BlockHeader>,
    pub block_data: MapIndex<T::Base, IdType, BlockData>,
    pub intra_index_nodes: MapIndex<T::Base, IdType, IntraIndexNode>,
    pub skip_list_nodes: MapIndex<T::Base, IdType, SkipListNode>,
    pub objs_in_this_round: ListIndex<T::Base, RawObject>,
}

impl<T: Access> VChainSchema<T> {
    pub fn state_hash(&self) -> Vec<Hash> {
        vec![self.block_headers.object_hash()]
    }
}

impl<T: Access> vchain::ReadInterface for VChainSchema<T> {
    fn get_parameter(&self) -> Result<vchain::Parameter> {
        self.param
            .get()
            .context("failed to get parameter")?
            .to_vchain_type()
    }
    fn read_block_header(&self, id: IdType) -> Result<vchain::BlockHeader> {
        self.block_headers
            .get(&id)
            .context("failed to read block header")?
            .to_vchain_type()
    }
    fn read_block_data(&self, id: IdType) -> Result<vchain::BlockData> {
        self.block_data
            .get(&id)
            .context("failed to read block data")?
            .to_vchain_type()
    }
    fn read_intra_index_node(&self, id: IdType) -> Result<vchain::IntraIndexNode> {
        self.intra_index_nodes
            .get(&id)
            .context("failed to read intra index node")?
            .to_vchain_type()
    }
    fn read_skip_list_node(&self, id: IdType) -> Result<vchain::SkipListNode> {
        self.skip_list_nodes
            .get(&id)
            .context("failed to read skip list node")?
            .to_vchain_type()
    }
    fn read_object(&self, id: IdType) -> Result<vchain::Object> {
        self.objects
            .get(&id)
            .context("failed to read object")?
            .to_vchain_type()
    }
}

impl<T: Access> vchain::WriteInterface for VChainSchema<T>
where
    T::Base: RawAccessMut,
{
    fn set_parameter(&mut self, param: vchain::Parameter) -> Result<()> {
        self.param.set(Parameter::create(&param)?);
        Ok(())
    }
    fn write_block_header(&mut self, header: vchain::BlockHeader) -> Result<()> {
        let id = header.block_id;
        self.block_headers.put(&id, BlockHeader::create(&header)?);
        Ok(())
    }
    fn write_block_data(&mut self, data: vchain::BlockData) -> Result<()> {
        let id = data.block_id;
        self.block_data.put(&id, BlockData::create(&data)?);
        Ok(())
    }
    fn write_intra_index_node(&mut self, node: vchain::IntraIndexNode) -> Result<()> {
        let id = node.id();
        self.intra_index_nodes
            .put(&id, IntraIndexNode::create(&node)?);
        Ok(())
    }
    fn write_skip_list_node(&mut self, node: vchain::SkipListNode) -> Result<()> {
        let id = node.id;
        self.skip_list_nodes.put(&id, SkipListNode::create(&node)?);
        Ok(())
    }
    fn write_object(&mut self, obj: vchain::Object) -> Result<()> {
        let id = obj.id;
        self.objects.put(&id, Object::create(&obj)?);
        Ok(())
    }
}
