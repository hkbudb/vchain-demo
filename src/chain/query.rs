use crate::set::{MultiSet, SetElement};
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct BoolExp<T: SetElement> {
    pub(crate) inner: Vec<MultiSet<T>>,
}

#[derive(Debug, Default, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct Range(pub(crate) [Vec<Option<u32>>; 2]);

#[derive(Debug, Default, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Query {
    pub start_block: u64,
    pub end_block: u64,
    pub q_range: Option<Range>,
    pub q_bool: Option<BoolExp<String>>,
}
