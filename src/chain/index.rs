use super::{IdType, SetElementType, SkipLstLvlType};
use crate::acc::curve::G1Affine;
use crate::digest::{blake2, concat_digest_ref, Digest, Digestable};
use crate::set::MultiSet;
use core::sync::atomic::{AtomicU64, Ordering};
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

static INTRA_INDEX_ID_CNT: AtomicU64 = AtomicU64::new(0);
static SKIP_LIST_ID_CNT: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum IntraIndexNode {
    NonLeaf(Box<IntraIndexNonLeaf>),
    Leaf(Box<IntraIndexLeaf>),
}

impl IntraIndexNode {
    pub fn id(&self) -> IdType {
        match self {
            Self::NonLeaf(x) => x.id,
            Self::Leaf(x) => x.id,
        }
    }
    pub fn block_id(&self) -> IdType {
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
    pub id: IdType,
    pub block_id: IdType,
    pub set_data: MultiSet<SetElementType>,
    #[serde(with = "crate::acc::serde_impl")]
    pub acc_value: G1Affine,
    pub child_hash_digest: Digest,
    pub child_hashes: SmallVec<[Digest; 2]>,
    pub child_ids: SmallVec<[IdType; 2]>,
}

impl IntraIndexNonLeaf {
    pub fn create(
        block_id: IdType,
        set_data: MultiSet<SetElementType>,
        acc_value: G1Affine,
        child_hashes: SmallVec<[Digest; 2]>,
        child_ids: SmallVec<[IdType; 2]>,
    ) -> Self {
        let id = INTRA_INDEX_ID_CNT.fetch_add(1, Ordering::SeqCst) as IdType;
        Self {
            id,
            block_id,
            set_data,
            acc_value,
            child_hash_digest: concat_digest_ref(child_hashes.iter()),
            child_hashes,
            child_ids,
        }
    }
}

impl Digestable for IntraIndexNonLeaf {
    fn to_digest(&self) -> Digest {
        concat_digest_ref([self.acc_value.to_digest(), self.child_hash_digest].iter())
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct IntraIndexLeaf {
    pub id: IdType,
    pub block_id: IdType,
    pub set_data: MultiSet<SetElementType>,
    #[serde(with = "crate::acc::serde_impl")]
    pub acc_value: G1Affine,
    pub obj_id: IdType,
    pub obj_hash: Digest,
}

impl IntraIndexLeaf {
    pub fn create(
        block_id: IdType,
        set_data: MultiSet<SetElementType>,
        acc_value: G1Affine,
        obj_id: IdType,
        obj_hash: Digest,
    ) -> Self {
        let id = INTRA_INDEX_ID_CNT.fetch_add(1, Ordering::SeqCst) as IdType;
        Self {
            id,
            block_id,
            set_data,
            acc_value,
            obj_id,
            obj_hash,
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
    pub id: IdType,
    pub block_id: IdType,
    pub level: SkipLstLvlType,
    pub set_data: MultiSet<SetElementType>,
    #[serde(with = "crate::acc::serde_impl")]
    pub acc_value: G1Affine,
    pub pre_skipped_hash: Digest,
    pub digest: Digest,
}

impl SkipListNode {
    pub fn create(
        block_id: IdType,
        level: SkipLstLvlType,
        set_data: MultiSet<SetElementType>,
        acc_value: G1Affine,
        pre_skipped_hash: Digest,
    ) -> Self {
        let id = SKIP_LIST_ID_CNT.fetch_add(1, Ordering::SeqCst) as IdType;
        let digest = concat_digest_ref([acc_value.to_digest(), pre_skipped_hash].iter());
        Self {
            id,
            block_id,
            level,
            set_data,
            acc_value,
            pre_skipped_hash,
            digest,
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum IntraData {
    // List of object ids
    Flat(Vec<IdType>),
    // IntraIndexNode root id
    Index(IdType),
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct BlockData {
    pub block_id: IdType,
    pub data: IntraData,
    pub set_data: MultiSet<SetElementType>,
    #[serde(with = "crate::acc::serde_impl")]
    pub acc_value: G1Affine,
    pub skip_list_ids: Vec<IdType>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Default, Serialize, Deserialize)]
pub struct BlockHeader {
    pub block_id: IdType,
    pub prev_hash: Digest,
    pub data_root: Digest,
    pub skip_list_root: Option<Digest>,
}

impl Digestable for BlockHeader {
    fn to_digest(&self) -> Digest {
        let mut state = blake2().to_state();
        state.update(&self.block_id.to_le_bytes());
        state.update(&self.prev_hash.0);
        state.update(&self.data_root.0);
        if let Some(d) = self.skip_list_root {
            state.update(&d.0);
        }
        Digest::from(state.finalize())
    }
}
