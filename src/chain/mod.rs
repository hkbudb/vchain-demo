use crate::acc;
use serde::{Deserialize, Serialize};

pub mod object;
pub use object::*;

pub mod index;
pub use index::*;

#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct Parameter {
    v_bit_len: Vec<u8>,
    acc_type: acc::Type,
    use_sk: bool,
    intra_index: bool,
    skip_list_max_level: u16,
}
