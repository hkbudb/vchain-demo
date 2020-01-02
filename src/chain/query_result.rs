use super::*;
use crate::acc::curve::{G1Affine, G1Projective};
use crate::acc::{self, Accumulator, AccumulatorProof};
use crate::digest::{blake2, concat_digest, concat_digest_ref, Digest, Digestable};
use crate::set::MultiSet;
use algebra::curves::ProjectiveCurve;
use core::ops::Deref;
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;
use std::collections::HashMap;

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum VerifyResult {
    Ok,
    InvalidSetIdx(usize),
    InvalidAccIdx(AccProofIdxType),
    InvalidAccProof(AccProofIdxType),
    InvalidMatchObj(IdType),
    InvalidQuery,
    InvalidHash,
}

#[derive(Debug, Default, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct ResultObjs(pub HashMap<IdType, Object>);

impl Deref for ResultObjs {
    type Target = HashMap<IdType, Object>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug, Default, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct ObjAcc(#[serde(with = "crate::acc::serde_impl")] pub G1Affine);

// set_idx, [  acc_idx / proof_idx ]
pub type AccProofIdxType = (usize, usize);

#[derive(Debug, Default, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct ResultVOAcc<AP: AccumulatorProof> {
    pub query_exp_sets: Vec<MultiSet<SetElementType>>,
    // <query_exp_set idx, [proof ...]>
    pub proofs: HashMap<usize, Vec<AP>>,
    // <query_exp_set idx, [obj_acc ...]>
    pub object_accs: HashMap<usize, Vec<ObjAcc>>,
}

impl<AP: AccumulatorProof> ResultVOAcc<AP> {
    pub fn get_object_acc(&self, proof_idx: AccProofIdxType) -> Option<&G1Affine> {
        Some(&self.object_accs.get(&proof_idx.0)?.get(proof_idx.1)?.0)
    }

    pub fn verify(&self) -> VerifyResult {
        match AP::TYPE {
            acc::Type::ACC1 => {
                for (&i, proofs) in self.proofs.iter() {
                    let query_acc = match self.query_exp_sets.get(i) {
                        Some(set) => acc::Acc1::cal_acc_g1(set),
                        None => return VerifyResult::InvalidSetIdx(i),
                    };
                    for (j, proof) in proofs.iter().enumerate() {
                        let acc_proof_idx = (i, j);
                        let proof = match proof.as_any().downcast_ref::<acc::Acc1Proof>() {
                            Some(proof) => proof,
                            None => return VerifyResult::InvalidAccIdx(acc_proof_idx),
                        };
                        let obj_acc = match self.get_object_acc(acc_proof_idx) {
                            Some(acc) => acc,
                            None => return VerifyResult::InvalidAccIdx(acc_proof_idx),
                        };
                        if !proof.verify(obj_acc, &query_acc) {
                            return VerifyResult::InvalidAccProof(acc_proof_idx);
                        }
                    }
                }
            }
            acc::Type::ACC2 => {
                for (&i, proofs) in self.proofs.iter() {
                    let query_acc = match self.query_exp_sets.get(i) {
                        Some(set) => acc::Acc2::cal_acc_g2(set),
                        None => return VerifyResult::InvalidSetIdx(i),
                    };
                    let obj_accs = match self.object_accs.get(&i) {
                        Some(accs) => accs,
                        None => return VerifyResult::InvalidSetIdx(i),
                    };
                    debug_assert_eq!(proofs.len(), 1);
                    let acc_proof_idx = (i, 0);
                    let proof = match proofs[0].as_any().downcast_ref::<acc::Acc2Proof>() {
                        Some(proof) => proof,
                        None => return VerifyResult::InvalidAccIdx(acc_proof_idx),
                    };
                    let mut g1 = G1Projective::zero();
                    for obj_acc in obj_accs.iter() {
                        g1.add_assign_mixed(&obj_acc.0);
                    }
                    if !proof.verify(&g1.into_affine(), &query_acc) {
                        return VerifyResult::InvalidAccProof(acc_proof_idx);
                    }
                }
            }
        }
        VerifyResult::Ok
    }

    pub fn add_proof(
        &mut self,
        query_exp_set: &MultiSet<SetElementType>,
        query_exp_set_d: &acc::DigestSet,
        object_set_d: &acc::DigestSet,
        object_acc: &G1Affine,
    ) -> Result<AccProofIdxType> {
        let query_exp_set_idx = match self.query_exp_sets.iter().position(|s| s == query_exp_set) {
            Some(idx) => idx,
            None => {
                self.query_exp_sets.push(query_exp_set.clone());
                self.query_exp_sets.len() - 1
            }
        };
        let object_acc = ObjAcc(*object_acc);
        let proof = AP::gen_proof(object_set_d, query_exp_set_d)?;

        match AP::TYPE {
            acc::Type::ACC1 => {
                let proof_ptr = self
                    .proofs
                    .entry(query_exp_set_idx)
                    .or_insert_with(Vec::new);
                proof_ptr.push(proof);
                let acc_ptr = self
                    .object_accs
                    .entry(query_exp_set_idx)
                    .or_insert_with(Vec::new);
                acc_ptr.push(object_acc);
                debug_assert_eq!(proof_ptr.len(), acc_ptr.len());
                Ok((query_exp_set_idx, proof_ptr.len() - 1))
            }
            acc::Type::ACC2 => {
                let proof_ptr = self
                    .proofs
                    .entry(query_exp_set_idx)
                    .or_insert_with(Vec::new);
                let acc_ptr = self
                    .object_accs
                    .entry(query_exp_set_idx)
                    .or_insert_with(Vec::new);
                acc_ptr.push(object_acc);
                if proof_ptr.is_empty() {
                    proof_ptr.push(proof);
                    Ok((query_exp_set_idx, 0))
                } else {
                    debug_assert_eq!(proof_ptr.len(), 1);
                    proof_ptr[0].combine_proof(&proof)?;
                    Ok((query_exp_set_idx, acc_ptr.len() - 1))
                }
            }
        }
    }
}

#[derive(Debug, Default, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct ResultVOTree(pub Vec<vo::ResultVONode>);

impl ResultVOTree {
    pub fn compute_digest<AP: AccumulatorProof>(
        &self,
        res_objs: &ResultObjs,
        vo_acc: &ResultVOAcc<AP>,
        prev_hash: &Digest,
    ) -> Option<Digest> {
        let mut hash_root = *prev_hash;
        for n in &self.0 {
            hash_root = n.compute_digest(res_objs, vo_acc, &hash_root)?;
        }
        Some(hash_root)
    }
}

#[derive(Debug, Default, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct ResultVO<AP: AccumulatorProof> {
    pub vo_t: ResultVOTree,
    pub vo_acc: ResultVOAcc<AP>,
}

#[derive(Debug, Default, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct ResultObjsandVO<AP: AccumulatorProof> {
    pub res_objs: ResultObjs,
    pub res_vo: ResultVO<AP>,
}

impl<AP: AccumulatorProof> ResultObjsandVO<AP> {
    pub fn verify(&self, query: &Query, chain: &impl ReadInterface) -> Result<VerifyResult> {
        let param = chain.get_parameter()?;
        let query_exp = query.to_bool_exp(&param.v_bit_len);
        for (id, obj) in self.res_objs.iter() {
            if !query_exp.is_match(&obj.set_data) {
                return Ok(VerifyResult::InvalidMatchObj(*id));
            }
        }
        if self.res_vo.vo_acc.query_exp_sets != query_exp.inner {
            return Ok(VerifyResult::InvalidQuery);
        }
        match self.res_vo.vo_acc.verify() {
            VerifyResult::Ok => {}
            x => return Ok(x),
        }
        let prev_hash = chain.read_block_header(query.start_block)?.prev_hash;
        let hash_root = chain.read_block_header(query.end_block)?.to_digest();
        if self
            .res_vo
            .vo_t
            .compute_digest(&self.res_objs, &self.res_vo.vo_acc, &prev_hash)
            != Some(hash_root)
        {
            return Ok(VerifyResult::InvalidHash);
        }
        todo!();
    }
}

pub mod vo {
    use super::*;

    #[derive(Debug, Default, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
    pub struct MatchObjNode {
        pub obj_id: IdType,
    }

    impl MatchObjNode {
        pub fn create(o: &Object) -> Self {
            Self { obj_id: o.id }
        }
        pub fn compute_digest<AP: AccumulatorProof>(
            self,
            res_objs: &ResultObjs,
            _vo_acc: &ResultVOAcc<AP>,
        ) -> Option<Digest> {
            let obj = res_objs.get(&self.obj_id)?;
            Some(concat_digest_ref(
                [obj.acc_value.to_digest(), obj.to_digest()].iter(),
            ))
        }
    }

    #[derive(Debug, Default, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
    pub struct NoMatchObjNode {
        pub obj_hash: Digest,
        pub proof_idx: AccProofIdxType,
    }

    impl NoMatchObjNode {
        pub fn create(o: &Object, proof_idx: AccProofIdxType) -> Self {
            Self {
                obj_hash: o.to_digest(),
                proof_idx,
            }
        }
        pub fn compute_digest<AP: AccumulatorProof>(
            &self,
            _res_objs: &ResultObjs,
            vo_acc: &ResultVOAcc<AP>,
        ) -> Option<Digest> {
            let acc_value = vo_acc.get_object_acc(self.proof_idx)?;
            Some(concat_digest_ref(
                [acc_value.to_digest(), self.obj_hash].iter(),
            ))
        }
    }

    #[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
    pub enum ObjNode {
        Match(MatchObjNode),
        NoMatch(NoMatchObjNode),
    }

    impl ObjNode {
        pub fn compute_digest<AP: AccumulatorProof>(
            &self,
            res_objs: &ResultObjs,
            vo_acc: &ResultVOAcc<AP>,
        ) -> Option<Digest> {
            match self {
                Self::Match(n) => n.compute_digest(res_objs, vo_acc),
                Self::NoMatch(n) => n.compute_digest(res_objs, vo_acc),
            }
        }
    }

    #[derive(Debug, Default, Clone, Eq, PartialEq, Serialize, Deserialize)]
    pub struct FlatBlkNode {
        pub block_id: IdType,
        pub skip_list_root: Option<Digest>,
        pub sub_nodes: Vec<ObjNode>,
    }

    impl FlatBlkNode {
        pub fn compute_digest<AP: AccumulatorProof>(
            &self,
            res_objs: &ResultObjs,
            vo_acc: &ResultVOAcc<AP>,
            prev_hash: &Digest,
        ) -> Option<Digest> {
            let mut hs: Vec<Digest> = Vec::with_capacity(self.sub_nodes.len());
            for sub_node in &self.sub_nodes {
                hs.push(sub_node.compute_digest(res_objs, vo_acc)?);
            }
            let data_root = concat_digest(hs.into_iter());

            let mut state = blake2().to_state();
            state.update(&self.block_id.to_le_bytes());
            state.update(&prev_hash.0);
            state.update(&data_root.0);
            if let Some(d) = self.skip_list_root {
                state.update(&d.0);
            }
            Some(Digest::from(state.finalize()))
        }
    }

    #[derive(Debug, Default, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
    pub struct NoMatchIntraNonLeaf {
        pub child_hash_digest: Digest,
        pub proof_idx: AccProofIdxType,
    }

    impl NoMatchIntraNonLeaf {
        pub fn create(n: &IntraIndexNonLeaf, proof_idx: AccProofIdxType) -> Self {
            Self {
                child_hash_digest: n.child_hash_digest,
                proof_idx,
            }
        }
        pub fn compute_digest<AP: AccumulatorProof>(
            &self,
            _res_objs: &ResultObjs,
            vo_acc: &ResultVOAcc<AP>,
        ) -> Option<Digest> {
            let acc_value = vo_acc.get_object_acc(self.proof_idx)?;
            Some(concat_digest_ref(
                [acc_value.to_digest(), self.child_hash_digest].iter(),
            ))
        }
    }

    #[derive(Debug, Default, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
    pub struct NoMatchIntraLeaf {
        pub obj_hash: Digest,
        pub proof_idx: AccProofIdxType,
    }

    impl NoMatchIntraLeaf {
        pub fn create(n: &IntraIndexLeaf, proof_idx: AccProofIdxType) -> Self {
            Self {
                obj_hash: n.obj_hash,
                proof_idx,
            }
        }
        pub fn compute_digest<AP: AccumulatorProof>(
            &self,
            _res_objs: &ResultObjs,
            vo_acc: &ResultVOAcc<AP>,
        ) -> Option<Digest> {
            let acc_value = vo_acc.get_object_acc(self.proof_idx)?;
            Some(concat_digest_ref(
                [acc_value.to_digest(), self.obj_hash].iter(),
            ))
        }
    }

    #[derive(Debug, Default, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
    pub struct MatchIntraLeaf {
        pub obj_id: IdType,
    }

    impl MatchIntraLeaf {
        pub fn create(o: &Object) -> Self {
            Self { obj_id: o.id }
        }
        pub fn compute_digest<AP: AccumulatorProof>(
            self,
            res_objs: &ResultObjs,
            _vo_acc: &ResultVOAcc<AP>,
        ) -> Option<Digest> {
            let obj = res_objs.get(&self.obj_id)?;
            Some(concat_digest_ref(
                [obj.acc_value.to_digest(), obj.to_digest()].iter(),
            ))
        }
    }

    #[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
    pub enum IntraNode {
        NoMatchIntraLeaf(Box<NoMatchIntraLeaf>),
        NoMatchIntraNonLeaf(Box<NoMatchIntraNonLeaf>),
        MatchIntraLeaf(Box<MatchIntraLeaf>),
        IntraNonLeaf(Box<IntraNonLeaf>),
    }

    impl IntraNode {
        pub fn compute_digest<AP: AccumulatorProof>(
            &self,
            res_objs: &ResultObjs,
            vo_acc: &ResultVOAcc<AP>,
        ) -> Option<Digest> {
            match self {
                Self::NoMatchIntraLeaf(n) => n.compute_digest(res_objs, vo_acc),
                Self::NoMatchIntraNonLeaf(n) => n.compute_digest(res_objs, vo_acc),
                Self::MatchIntraLeaf(n) => n.compute_digest(res_objs, vo_acc),
                Self::IntraNonLeaf(n) => n.compute_digest(res_objs, vo_acc),
            }
        }
    }

    #[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
    pub struct IntraNonLeaf {
        #[serde(with = "crate::acc::serde_impl")]
        pub acc_value: G1Affine,
        pub children: SmallVec<[IntraNode; 2]>,
    }

    impl IntraNonLeaf {
        pub fn create(n: &IntraIndexNonLeaf) -> Self {
            Self {
                acc_value: n.acc_value,
                children: SmallVec::new(),
            }
        }
        pub fn compute_digest<AP: AccumulatorProof>(
            &self,
            res_objs: &ResultObjs,
            vo_acc: &ResultVOAcc<AP>,
        ) -> Option<Digest> {
            let mut child_hashes: SmallVec<[Digest; 2]> = SmallVec::new();
            for child in &self.children {
                child_hashes.push(child.compute_digest(res_objs, vo_acc)?);
            }
            let child_hash_digest = concat_digest_ref(child_hashes.iter());
            Some(concat_digest_ref(
                [self.acc_value.to_digest(), child_hash_digest].iter(),
            ))
        }
    }

    #[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
    pub struct BlkNode {
        pub block_id: IdType,
        pub skip_list_root: Option<Digest>,
        pub sub_node: IntraNode,
    }

    impl BlkNode {
        pub fn compute_digest<AP: AccumulatorProof>(
            &self,
            res_objs: &ResultObjs,
            vo_acc: &ResultVOAcc<AP>,
            prev_hash: &Digest,
        ) -> Option<Digest> {
            let data_root = self.sub_node.compute_digest(res_objs, vo_acc)?;
            let mut state = blake2().to_state();
            state.update(&self.block_id.to_le_bytes());
            state.update(&prev_hash.0);
            state.update(&data_root.0);
            if let Some(d) = self.skip_list_root {
                state.update(&d.0);
            }
            Some(Digest::from(state.finalize()))
        }
    }

    #[derive(Debug, Default, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
    pub struct JumpNode {
        pub proof_idx: AccProofIdxType,
    }

    impl JumpNode {
        pub fn create(proof_idx: AccProofIdxType) -> Self {
            Self { proof_idx }
        }
        pub fn compute_digest<AP: AccumulatorProof>(
            &self,
            _res_objs: &ResultObjs,
            vo_acc: &ResultVOAcc<AP>,
            prev_hash: &Digest,
        ) -> Option<Digest> {
            let acc_value = vo_acc.get_object_acc(self.proof_idx)?;
            Some(concat_digest_ref(
                [acc_value.to_digest(), *prev_hash].iter(),
            ))
        }
    }

    #[derive(Debug, Default, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
    pub struct NoJumpNode {
        pub digest: Digest,
    }

    impl NoJumpNode {
        pub fn create(n: &SkipListNode) -> Self {
            Self { digest: n.digest }
        }
        pub fn compute_digest<AP: AccumulatorProof>(
            &self,
            _res_objs: &ResultObjs,
            _vo_acc: &ResultVOAcc<AP>,
            _prev_hash: &Digest,
        ) -> Option<Digest> {
            Some(self.digest)
        }
    }

    #[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
    pub enum JumpOrNoJumpNode {
        Jump(Box<JumpNode>),
        NoJump(Box<NoJumpNode>),
    }

    impl JumpOrNoJumpNode {
        pub fn compute_digest<AP: AccumulatorProof>(
            &self,
            res_objs: &ResultObjs,
            vo_acc: &ResultVOAcc<AP>,
            prev_hash: &Digest,
        ) -> Option<Digest> {
            match self {
                Self::Jump(n) => n.compute_digest(res_objs, vo_acc, prev_hash),
                Self::NoJump(n) => n.compute_digest(res_objs, vo_acc, prev_hash),
            }
        }
    }

    #[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
    pub struct SkipListRoot {
        pub block_id: IdType,
        pub blk_prev_hash: Digest,
        pub blk_data_root: Digest,
        pub sub_nodes: Vec<JumpOrNoJumpNode>,
    }

    impl SkipListRoot {
        pub fn compute_digest<AP: AccumulatorProof>(
            &self,
            res_objs: &ResultObjs,
            vo_acc: &ResultVOAcc<AP>,
            prev_hash: &Digest,
        ) -> Option<Digest> {
            let mut hs: Vec<Digest> = Vec::with_capacity(self.sub_nodes.len());
            for sub_node in &self.sub_nodes {
                hs.push(sub_node.compute_digest(res_objs, vo_acc, prev_hash)?);
            }
            let skip_list_root = concat_digest(hs.into_iter());
            let mut state = blake2().to_state();
            state.update(&self.block_id.to_le_bytes());
            state.update(&self.blk_prev_hash.0);
            state.update(&self.blk_data_root.0);
            state.update(&skip_list_root.0);
            Some(Digest::from(state.finalize()))
        }
    }

    #[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
    pub enum ResultVONode {
        FlatBlkNode(FlatBlkNode),
        BlkNode(BlkNode),
        SkipListRoot(SkipListRoot),
    }

    impl ResultVONode {
        pub fn compute_digest<AP: AccumulatorProof>(
            &self,
            res_objs: &ResultObjs,
            vo_acc: &ResultVOAcc<AP>,
            prev_hash: &Digest,
        ) -> Option<Digest> {
            match self {
                Self::FlatBlkNode(n) => n.compute_digest(res_objs, vo_acc, prev_hash),
                Self::BlkNode(n) => n.compute_digest(res_objs, vo_acc, prev_hash),
                Self::SkipListRoot(n) => n.compute_digest(res_objs, vo_acc, prev_hash),
            }
        }
    }
}
