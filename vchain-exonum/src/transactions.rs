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
#[protobuf_convert(source = "proto::TxAddObjs")]
pub struct TxAddObjs {
    pub objs: Vec<RawObject>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ProtobufConvert, BinaryValue, ObjectHash)]
#[protobuf_convert(source = "proto::TxSetParam")]
pub struct TxSetParam {
    pub v_bit_len: Vec<i32>,
    pub is_acc2: bool,
    pub intra_index: bool,
    pub skip_list_max_level: i32,
}

impl TxSetParam {
    pub fn into_vchain_type(self) -> vchain::Parameter {
        vchain::Parameter {
            v_bit_len: self.v_bit_len.iter().map(|x| *x as u8).collect(),
            acc_type: if self.is_acc2 {
                vchain::acc::Type::ACC2
            } else {
                vchain::acc::Type::ACC1
            },
            use_sk: false,
            intra_index: self.intra_index,
            skip_list_max_level: self.skip_list_max_level as vchain::SkipLstLvlType,
        }
    }
}
