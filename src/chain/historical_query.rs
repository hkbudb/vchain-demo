#![allow(clippy::cognitive_complexity)]

use super::*;
use crate::acc::AccumulatorProof;
use anyhow::Result;

pub fn historical_query<AP: AccumulatorProof>(
    q: &Query,
    chain: &impl ReadInterface,
) -> Result<OverallResult<AP>> {
    info!("process query {:?}", q);
    let param = chain.get_parameter()?;
    let cpu_timer = howlong::ProcessCPUTimer::new();
    let timer = howlong::HighResolutionTimer::new();

    let mut res = OverallResult {
        res_objs: ResultObjs::new(),
        res_vo: ResultVO::<AP>::new(),
        query: q.clone(),
        query_time_in_ms: 0,
        v_bit_len: param.v_bit_len,
        vo_size: 0,
    };

    res.query_time_in_ms = timer.elapsed().as_millis();
    info!("used time: {}", cpu_timer.elapsed());
    Ok(res)
}
