#![allow(clippy::cognitive_complexity)]

use super::*;
use crate::acc::{Accumulator, AccumulatorProof};
use anyhow::Result;

pub fn historical_query<AP: AccumulatorProof>(
    q: &Query,
    chain: &impl ReadInterface,
) -> Result<ResultObjsandVO<AP>> {
    info!("process query {:?}", q);
    todo!();
}
