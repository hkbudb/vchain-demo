pub use self::service::{
    BlockData, BlockHeader, InitParam, IntraIndexNode, Object, Parameter, RawObject, SkipListNode,
    TxAddObjs,
};

include!(concat!(env!("OUT_DIR"), "/protobuf_mod.rs"));
