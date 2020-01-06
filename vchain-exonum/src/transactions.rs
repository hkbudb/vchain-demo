use exonum_derive::{BinaryValue, ObjectHash};
use exonum_proto::ProtobufConvert;
use std::collections::HashSet;
use std::iter::FromIterator;
use vchain::IdType;

use super::proto;

#[derive(Clone, Debug, Serialize, Deserialize, ProtobufConvert, BinaryValue, ObjectHash)]
#[protobuf_convert(source = "proto::RawObject")]
pub struct RawObject {
    pub v_data: Vec<u32>,
    pub w_data: Vec<String>,
}

impl RawObject {
    pub fn into_vchain_type(self, block_id: IdType) -> vchain::RawObject {
        vchain::RawObject {
            block_id,
            v_data: self.v_data,
            w_data: HashSet::from_iter(self.w_data.into_iter()),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, ProtobufConvert, BinaryValue, ObjectHash)]
#[protobuf_convert(source = "proto::TxNewBlock")]
pub struct TxNewBlock {
    pub objs: Vec<RawObject>,
}
