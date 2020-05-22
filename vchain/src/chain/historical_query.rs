#![allow(clippy::cognitive_complexity)]

use super::*;
use crate::acc::{AccumulatorProof, DigestSet};
use anyhow::{bail, Result};
use std::collections::VecDeque;

pub fn historical_query<AP: AccumulatorProof + Serialize>(
    q: &Query,
    chain: &impl ReadInterface,
) -> Result<OverallResult<AP>> {
    info!("process query {:?}", q);
    let param = chain.get_parameter()?;
    let cpu_timer = howlong::ProcessCPUTimer::new();
    let timer = howlong::HighResolutionTimer::new();

    let query_exp = q.to_bool_exp(&param.v_bit_len);
    let mut res = OverallResult {
        res_objs: ResultObjs::new(),
        res_vo: ResultVO::<AP>::new(),
        query: q.clone(),
        query_exp_set: query_exp,
        query_time_in_ms: 0,
        v_bit_len: param.v_bit_len.clone(),
        vo_size: 0,
        vo_stats: VOStatistic::default(),
    };
    let query_exp_digest_set = query_exp
        .inner
        .iter()
        .map(|s| DigestSet::new(s))
        .collect::<Vec<_>>();

    let mut block_id = q.end_block;
    while block_id >= q.start_block {
        let blk_data = chain.read_block_data(block_id)?;
        let blk_header = chain.read_block_header(block_id)?;

        if !blk_data.skip_list_ids.is_empty() {
            let mut vo_skip = vo::SkipListRoot {
                block_id,
                blk_prev_hash: blk_header.prev_hash,
                blk_data_root: blk_header.data_root,
                sub_nodes: Vec::new(),
            };
            let mut jmp_level: Option<SkipLstLvlType> = None;

            for (lvl, &skip_list_id) in blk_data.skip_list_ids.iter().enumerate().rev() {
                let jmp_node = chain.read_skip_list_node(skip_list_id)?;
                if jmp_level.is_some()
                    || q.start_block + skipped_blocks_num(lvl as SkipLstLvlType) > block_id
                {
                    vo_skip
                        .sub_nodes
                        .push(vo::NoJumpNode::create(&jmp_node).into_jump_or_no_jump_node());
                } else {
                    let mismatch_idx = query_exp.mismatch_idx(&jmp_node.set_data);
                    if let Some(mismatch_idx) = mismatch_idx {
                        jmp_level = Some(lvl as SkipLstLvlType);
                        let proof_idx = res.res_vo.vo_acc.add_proof(
                            mismatch_idx,
                            &query_exp_digest_set[mismatch_idx],
                            &DigestSet::new(&jmp_node.set_data),
                            &jmp_node.acc_value,
                        )?;
                        vo_skip.sub_nodes.push(
                            vo::JumpNode::create(&jmp_node, proof_idx).into_jump_or_no_jump_node(),
                        );
                    } else {
                        vo_skip
                            .sub_nodes
                            .push(vo::NoJumpNode::create(&jmp_node).into_jump_or_no_jump_node());
                    }
                }
            }

            if let Some(jmp_level) = jmp_level {
                vo_skip.sub_nodes.reverse();
                res.res_vo.vo_t.0.push(vo_skip.into_result_vo_node());
                block_id -= skipped_blocks_num(jmp_level);
                continue;
            }
        } // skip list

        if param.intra_index {
            query_block_intra_index(
                &query_exp,
                &query_exp_digest_set,
                &blk_header,
                &blk_data,
                chain,
                &mut res,
            )?;
        } else {
            query_block_no_intra_index(
                &query_exp,
                &query_exp_digest_set,
                &blk_header,
                &blk_data,
                chain,
                &mut res,
            )?;
        }

        block_id -= 1;
    }

    res.res_vo.vo_t.0.reverse();
    res.query_time_in_ms = timer.elapsed().as_millis() as u64;
    res.compute_stats()?;
    info!("used time: {}", cpu_timer.elapsed());
    Ok(res)
}

fn query_block_intra_index<AP: AccumulatorProof>(
    query_exp: &BoolExp<SetElementType>,
    query_exp_digest_set: &[DigestSet],
    block_header: &BlockHeader,
    block_data: &BlockData,
    chain: &impl ReadInterface,
    res: &mut OverallResult<AP>,
) -> Result<()> {
    let mut vo_blk = vo::BlkNode {
        block_id: block_header.block_id,
        skip_list_root: block_header.skip_list_root,
        sub_node: vo::IntraNode::Empty,
    };

    let root = match &block_data.data {
        IntraData::Index(id) => match chain.read_intra_index_node(*id)? {
            IntraIndexNode::NonLeaf(n) => n,
            IntraIndexNode::Leaf(_) => bail!("invalid data"),
        },
        _ => bail!("invalid data"),
    };

    let mut intra_index_q: VecDeque<(Box<IntraIndexNonLeaf>, *mut vo::IntraNode)> = VecDeque::new();
    intra_index_q.push_back((root, &mut vo_blk.sub_node as *mut vo::IntraNode));
    while let Some((node, ptr)) = intra_index_q.pop_front() {
        let mismatch_idx = query_exp.mismatch_idx(&node.set_data);
        if let Some(mismatch_idx) = mismatch_idx {
            let proof_idx = res.res_vo.vo_acc.add_proof(
                mismatch_idx,
                &query_exp_digest_set[mismatch_idx],
                &DigestSet::new(&node.set_data),
                &node.acc_value,
            )?;
            unsafe {
                *ptr = vo::NoMatchIntraNonLeaf::create(&node, proof_idx).into_intra_node();
            }
        } else {
            let intra_non_leaf = unsafe {
                *ptr = vo::IntraNonLeaf::create(&node).into_intra_node();
                match &mut *ptr {
                    vo::IntraNode::IntraNonLeaf(x) => x,
                    _ => unreachable!(),
                }
            };
            for &child_id in &node.child_ids {
                match chain.read_intra_index_node(child_id)? {
                    IntraIndexNode::NonLeaf(n) => {
                        intra_non_leaf.children.push(vo::IntraNode::Empty);
                        intra_index_q.push_back((
                            n,
                            intra_non_leaf.children.last_mut().unwrap() as *mut vo::IntraNode,
                        ));
                    }
                    IntraIndexNode::Leaf(n) => {
                        let mismatch_idx = query_exp.mismatch_idx(&n.set_data);
                        if let Some(mismatch_idx) = mismatch_idx {
                            let proof_idx = res.res_vo.vo_acc.add_proof(
                                mismatch_idx,
                                &query_exp_digest_set[mismatch_idx],
                                &DigestSet::new(&n.set_data),
                                &n.acc_value,
                            )?;
                            intra_non_leaf.children.push(
                                vo::NoMatchIntraLeaf::create(&n, proof_idx).into_intra_node(),
                            );
                        } else {
                            let obj = chain.read_object(n.obj_id)?;
                            res.res_objs.insert(obj);
                            intra_non_leaf
                                .children
                                .push(vo::MatchIntraLeaf::create(&n).into_intra_node());
                        }
                    }
                }
            }
        }
    }

    res.res_vo.vo_t.0.push(vo_blk.into_result_vo_node());
    Ok(())
}

fn query_block_no_intra_index<AP: AccumulatorProof>(
    query_exp: &BoolExp<SetElementType>,
    query_exp_digest_set: &[DigestSet],
    block_header: &BlockHeader,
    block_data: &BlockData,
    chain: &impl ReadInterface,
    res: &mut OverallResult<AP>,
) -> Result<()> {
    let mut vo_blk = vo::FlatBlkNode {
        block_id: block_header.block_id,
        skip_list_root: block_header.skip_list_root,
        sub_nodes: Vec::new(),
    };

    let objs = match &block_data.data {
        IntraData::Flat(ids) => ids
            .iter()
            .map(|&id| chain.read_object(id))
            .collect::<Result<Vec<_>>>()?,
        _ => bail!("invalid data"),
    };

    for obj in &objs {
        let mismatch_idx = query_exp.mismatch_idx(&obj.set_data);
        if let Some(mismatch_idx) = mismatch_idx {
            let proof_idx = res.res_vo.vo_acc.add_proof(
                mismatch_idx,
                &query_exp_digest_set[mismatch_idx],
                &DigestSet::new(&obj.set_data),
                &obj.acc_value,
            )?;
            vo_blk
                .sub_nodes
                .push(vo::NoMatchObjNode::create(obj, proof_idx).into_obj_node());
        } else {
            vo_blk
                .sub_nodes
                .push(vo::MatchObjNode::create(obj).into_obj_node());
            res.res_objs.insert(obj.clone());
        }
    }

    res.res_vo.vo_t.0.push(vo_blk.into_result_vo_node());
    Ok(())
}
