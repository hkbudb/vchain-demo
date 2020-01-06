pub use self::service::{
    BlockData, BlockHeader, IntraIndexNode, Object, Parameter, RawObject, SkipListNode, TxNewBlock,
};

include!(concat!(env!("OUT_DIR"), "/protobuf_mod.rs"));
