pub use self::service::{
    BlockData, BlockHeader, Config, IntraIndexNode, Object, Parameter, RawObject, SkipListNode,
    TxAddObjs, TxSetParam,
};

include!(concat!(env!("OUT_DIR"), "/protobuf_mod.rs"));
