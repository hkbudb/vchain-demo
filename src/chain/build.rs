use super::*;
use crate::digest::{concat_digest, Digest, Digestable};

pub fn build_block(
    block_id: u64,
    prev_hash: Digest,
    raw_objs: &[RawObject],
    chain: &mut (impl ReadInterface + WriteInterface),
) -> Result<()> {
    let param = chain.get_parameter()?;
    let objs: Vec<Object> = raw_objs.iter().map(|o| Object::create(o, &param)).collect();
    for obj in &objs {
        chain.write_object(obj.clone())?;
    }

    if param.intra_index {
    } else {
    }

    if param.skip_list_max_level > 0 {}

    todo!();
}
