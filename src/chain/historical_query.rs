#![allow(clippy::cognitive_complexity)]

use super::*;
use crate::acc::AccumulatorProof;
use anyhow::Result;

pub fn historical_query<AP: AccumulatorProof>(
    q: &Query,
    chain: &impl ReadInterface,
) -> Result<OverallResult<AP>> {
    info!("process query {:?}", q);
    todo!();
}

