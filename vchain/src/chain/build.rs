#![allow(clippy::cognitive_complexity)]

use super::*;
use crate::digest::{concat_digest, concat_digest_ref, Digest, Digestable};
use crate::set::MultiSet;
use algebra::curves::{AffineCurve, ProjectiveCurve};
use smallvec::smallvec;

pub fn build_block<'a>(
    block_id: IdType,
    prev_hash: Digest,
    raw_objs: impl Iterator<Item = &'a RawObject>,
    chain: &mut (impl ReadInterface + WriteInterface),
) -> Result<BlockHeader> {
    debug!("build block #{}", block_id);

    let param = chain.get_parameter()?;
    let objs: Vec<Object> = raw_objs.map(|o| Object::create(o, &param)).collect();
    for obj in &objs {
        chain.write_object(obj.clone())?;
    }

    let mut block_header = BlockHeader {
        block_id,
        prev_hash,
        ..Default::default()
    };

    let mut block_data = if param.intra_index {
        let mut leaves: Vec<IntraIndexLeaf> = Vec::with_capacity(objs.len());
        for obj in &objs {
            let node = IntraIndexLeaf::create(
                block_id,
                obj.set_data.clone(),
                obj.acc_value,
                obj.id,
                obj.to_digest(),
            );
            leaves.push(node.clone());
            chain.write_intra_index_node(IntraIndexNode::Leaf(Box::new(node)))?;
        }

        let mut non_leaves: Vec<IntraIndexNonLeaf> = Vec::with_capacity(leaves.len());
        while !leaves.is_empty() {
            let left_idx = leaves
                .iter()
                .enumerate()
                .max_by_key(|(_i, n)| n.set_data.len())
                .unwrap()
                .0;
            let left = leaves.remove(left_idx);

            if leaves.is_empty() {
                let node = IntraIndexNonLeaf::create(
                    block_id,
                    left.set_data.clone(),
                    left.acc_value,
                    smallvec![left.to_digest()],
                    smallvec![left.id],
                );
                non_leaves.push(node.clone());
                chain.write_intra_index_node(IntraIndexNode::NonLeaf(Box::new(node)))?;
                break;
            }

            let mut right_idx = 0;
            let mut min_set = &left.set_data | &leaves[0].set_data;
            let mut max_sim =
                (&left.set_data & &leaves[0].set_data).len() as f64 / min_set.len() as f64;
            for (i, n) in leaves.iter().enumerate().skip(1) {
                let s = &left.set_data | &n.set_data;
                let sim = (&left.set_data & &n.set_data).len() as f64 / s.len() as f64;
                if sim > max_sim {
                    max_sim = sim;
                    min_set = s;
                    right_idx = i;
                }
            }
            let right = leaves.remove(right_idx);
            let min_set_acc_value = multiset_to_g1(&min_set, &param);
            let node = IntraIndexNonLeaf::create(
                block_id,
                min_set,
                min_set_acc_value,
                smallvec![left.to_digest(), right.to_digest()],
                smallvec![left.id, right.id],
            );
            non_leaves.push(node.clone());
            chain.write_intra_index_node(IntraIndexNode::NonLeaf(Box::new(node)))?;
        }

        while non_leaves.len() > 1 {
            let mut new_non_leaves: Vec<IntraIndexNonLeaf> = Vec::with_capacity(non_leaves.len());
            while non_leaves.len() > 1 {
                let left_idx = non_leaves
                    .iter()
                    .enumerate()
                    .max_by_key(|(_i, n)| n.set_data.len())
                    .unwrap()
                    .0;
                let left = non_leaves.remove(left_idx);

                let mut right_idx = 0;
                let mut min_set = &left.set_data | &non_leaves[0].set_data;
                let mut max_sim =
                    (&left.set_data & &non_leaves[0].set_data).len() as f64 / min_set.len() as f64;
                for (i, n) in non_leaves.iter().enumerate().skip(1) {
                    let s = &left.set_data | &n.set_data;
                    let sim = (&left.set_data & &n.set_data).len() as f64 / s.len() as f64;
                    if sim > max_sim {
                        max_sim = sim;
                        min_set = s;
                        right_idx = i;
                    }
                }
                let right = non_leaves.remove(right_idx);
                let min_set_acc_value = multiset_to_g1(&min_set, &param);
                let node = IntraIndexNonLeaf::create(
                    block_id,
                    min_set,
                    min_set_acc_value,
                    smallvec![left.to_digest(), right.to_digest()],
                    smallvec![left.id, right.id],
                );
                new_non_leaves.push(node.clone());
                chain.write_intra_index_node(IntraIndexNode::NonLeaf(Box::new(node)))?;
            }
            non_leaves.append(&mut new_non_leaves);
        }

        // no objs in this block
        if non_leaves.is_empty() {
            let empty_set: MultiSet<SetElementType> = MultiSet::new();
            let acc_value = multiset_to_g1(&empty_set, &param);
            let node =
                IntraIndexNonLeaf::create(block_id, empty_set, acc_value, smallvec![], smallvec![]);
            non_leaves.push(node.clone());
            chain.write_intra_index_node(IntraIndexNode::NonLeaf(Box::new(node)))?;
        }

        let root = non_leaves.pop().unwrap();
        block_header.data_root = root.to_digest();
        BlockData {
            block_id,
            data: IntraData::Index(root.id),
            set_data: root.set_data,
            acc_value: root.acc_value,
            skip_list_ids: Vec::new(),
        }
    } else {
        let mut hs: Vec<Digest> = Vec::with_capacity(objs.len());
        let mut set_data: MultiSet<SetElementType> = MultiSet::new();
        for obj in &objs {
            let h = concat_digest_ref([obj.acc_value.to_digest(), obj.to_digest()].iter());
            hs.push(h);
            set_data = &set_data | &obj.set_data;
        }
        block_header.data_root = concat_digest(hs.into_iter());
        let acc_value = multiset_to_g1(&set_data, &param);
        BlockData {
            block_id,
            data: IntraData::Flat(objs.iter().map(|o| o.id).collect::<Vec<_>>()),
            set_data,
            acc_value,
            skip_list_ids: Vec::new(),
        }
    };

    if param.skip_list_max_level > 0 && block_id >= 1 {
        let mut prev_blk_id = block_id - 1;
        let mut skipped_blk_num = 1;
        let mut set_data_to_skip = block_data.set_data.clone();
        let mut acc_value_to_skip = block_data.acc_value.into_projective();
        let mut skip_list_ids: Vec<IdType> = Vec::with_capacity(param.skip_list_max_level as usize);
        let mut skip_list_digests: Vec<Digest> =
            Vec::with_capacity(param.skip_list_max_level as usize);
        let mut hash_to_skip = Digest::default();

        'outer: for level in 0..param.skip_list_max_level {
            let blk_num = skipped_blocks_num(level);
            while skipped_blk_num < blk_num {
                if prev_blk_id == 0 {
                    break 'outer;
                }
                let prev_blk_header = match chain.read_block_header(prev_blk_id) {
                    Ok(header) => header,
                    _ => break 'outer,
                };
                hash_to_skip = prev_blk_header.prev_hash;
                let prev_blk = chain.read_block_data(prev_blk_id)?;
                match param.acc_type {
                    acc::Type::ACC1 => {
                        set_data_to_skip = &set_data_to_skip | &prev_blk.set_data;
                    }
                    acc::Type::ACC2 => {
                        set_data_to_skip = &set_data_to_skip + &prev_blk.set_data;
                        acc_value_to_skip.add_assign_mixed(&prev_blk.acc_value);
                    }
                }

                skipped_blk_num += 1;
                prev_blk_id -= 1;
            }

            let acc_value_to_skip = match param.acc_type {
                acc::Type::ACC1 => multiset_to_g1(&set_data_to_skip, &param),
                acc::Type::ACC2 => acc_value_to_skip.into_affine(),
            };

            let skip_node = SkipListNode::create(
                block_id,
                level,
                set_data_to_skip.clone(),
                acc_value_to_skip,
                hash_to_skip,
            );
            skip_list_ids.push(skip_node.id);
            skip_list_digests.push(skip_node.digest);
            chain.write_skip_list_node(skip_node)?;
        }

        if !skip_list_ids.is_empty() {
            block_header.skip_list_root = Some(concat_digest(skip_list_digests.into_iter()));
            block_data.skip_list_ids = skip_list_ids;
        }
    }

    chain.write_block_header(block_header)?;
    chain.write_block_data(block_data)?;

    Ok(block_header)
}
