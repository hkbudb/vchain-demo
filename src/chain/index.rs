use super::{multiset_to_g1, Parameter, SetElementType};
use crate::acc::curve::G1Affine;
use crate::digest::{concat_digest_ref, Digest, Digestable};
use crate::set::MultiSet;
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;
use std::sync::atomic::{AtomicU64, Ordering};

static INTRA_INDEX_ID_CNT: AtomicU64 = AtomicU64::new(0);
static SKIP_LIST_ID_CNT: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum IntraIndexNode {
    NonLeaf(Box<IntraIndexNonLeaf>),
    Leaf(Box<IntraIndexLeaf>),
}

impl IntraIndexNode {
    pub fn id(&self) -> u64 {
        match self {
            Self::NonLeaf(x) => x.id,
            Self::Leaf(x) => x.id,
        }
    }
    pub fn block_id(&self) -> u64 {
        match self {
            Self::NonLeaf(x) => x.block_id,
            Self::Leaf(x) => x.block_id,
        }
    }
    pub fn set_data(&self) -> &MultiSet<SetElementType> {
        match self {
            Self::NonLeaf(x) => &x.set_data,
            Self::Leaf(x) => &x.set_data,
        }
    }
    pub fn acc_value(&self) -> &G1Affine {
        match self {
            Self::NonLeaf(x) => &x.acc_value,
            Self::Leaf(x) => &x.acc_value,
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct IntraIndexNonLeaf {
    pub id: u64,
    pub block_id: u64,
    pub set_data: MultiSet<SetElementType>,
    #[serde(with = "crate::acc::serde_impl")]
    pub acc_value: G1Affine,
    pub child_hash_digest: Digest,
    pub child_hashes: SmallVec<[Digest; 2]>,
    pub child_ids: SmallVec<[u64; 2]>,
}

impl IntraIndexNonLeaf {
    pub fn create(
        block_id: u64,
        set_data: &MultiSet<SetElementType>,
        child_hashes: &SmallVec<[Digest; 2]>,
        child_ids: &SmallVec<[u64; 2]>,
        param: &Parameter,
    ) -> Self {
        let id = INTRA_INDEX_ID_CNT.fetch_add(1, Ordering::SeqCst);
        Self {
            id,
            block_id,
            set_data: set_data.clone(),
            acc_value: multiset_to_g1(&set_data, param),
            child_hash_digest: concat_digest_ref(child_hashes.iter()),
            child_hashes: child_hashes.clone(),
            child_ids: child_ids.clone(),
        }
    }
}

impl Digestable for IntraIndexNonLeaf {
    fn to_digest(&self) -> Digest {
        concat_digest_ref([self.child_hash_digest, self.acc_value.to_digest()].iter())
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct IntraIndexLeaf {
    pub id: u64,
    pub block_id: u64,
    pub set_data: MultiSet<SetElementType>,
    #[serde(with = "crate::acc::serde_impl")]
    pub acc_value: G1Affine,
    pub obj_id: u64,
    pub obj_hash: Digest,
}

impl IntraIndexLeaf {
    pub fn create(
        block_id: u64,
        set_data: &MultiSet<SetElementType>,
        obj_id: u64,
        obj_hash: &Digest,
        param: &Parameter,
    ) -> Self {
        let id = INTRA_INDEX_ID_CNT.fetch_add(1, Ordering::SeqCst);
        Self {
            id,
            block_id,
            set_data: set_data.clone(),
            acc_value: multiset_to_g1(&set_data, param),
            obj_id,
            obj_hash: *obj_hash,
        }
    }
}

impl Digestable for IntraIndexLeaf {
    fn to_digest(&self) -> Digest {
        concat_digest_ref([self.acc_value.to_digest(), self.obj_hash].iter())
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct SkipListNode {
    pub id: u64,
    pub block_id: u64,
    pub level: u16,
    pub set_data: MultiSet<SetElementType>,
    #[serde(with = "crate::acc::serde_impl")]
    pub acc_value: G1Affine,
    pub pre_skipped_hash: Digest,
    pub digest: Digest,
}

impl SkipListNode {
    pub fn create(
        block_id: u64,
        level: u16,
        set_data: &MultiSet<SetElementType>,
        acc_value: &G1Affine,
        pre_skipped_hash: &Digest,
    ) -> Self {
        let id = SKIP_LIST_ID_CNT.fetch_add(1, Ordering::SeqCst);
        let digest = concat_digest_ref([*pre_skipped_hash, acc_value.to_digest()].iter());
        Self {
            id,
            block_id,
            level,
            set_data: set_data.clone(),
            acc_value: *acc_value,
            pre_skipped_hash: *pre_skipped_hash,
            digest,
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum IntraData {
    // List of object ids
    Flat(Vec<u64>),
    // IntraIndexNode root id
    Index(u64),
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct BlockData {
    pub block_id: u64,
    pub data: IntraData,
    pub set_data: MultiSet<SetElementType>,
    #[serde(with = "crate::acc::serde_impl")]
    pub acc_value: G1Affine,
    pub skip_list_ids: Vec<u64>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct BlockHeader {
    pub block_id: u64,
    pub prev_hash: Digest,
    pub data_root: Digest,
    pub skip_list_root: Option<Digest>,
}
