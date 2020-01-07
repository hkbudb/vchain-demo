#[macro_use]
extern crate log;

use anyhow::{Result};
use exonum::{crypto, runtime::rust::Transaction};
use serde::{Serialize, Deserialize};
use std::path::{PathBuf};
use structopt::StructOpt;
use vchain_exonum::transactions::{TxAddObjs, RawObject};
use vchain::{IdType, load_raw_obj_from_file};
use std::collections::{BTreeMap, };
use serde_json::json;
use std::thread::sleep;
use std::time::Duration;

#[derive(StructOpt, Debug)]
#[structopt(name = "simchain-build")]
struct Opts {
    /// input data path
    #[structopt(short, long, parse(from_os_str))]
    input: PathBuf,

    /// api address
    #[structopt(short, long, default_value = "http://127.0.0.1:5000")]
    api_address: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TxResponse {
    tx_hash: String,
}

#[actix_rt::main]
async fn main() -> Result<()> {
    env_logger::init_from_env(env_logger::Env::default().filter_or("RUST_LOG", "info"));
    let opts = Opts::from_args();
    let tx_url = format!("{}/api/explorer/v1/transactions", opts.api_address);

    info!("read data from {:?}", opts.input);
    warn!("blk id from data file will be ignored");

    let raw_objs = load_raw_obj_from_file(&opts.input)?;
    let mut txs: BTreeMap<IdType, TxAddObjs> = BTreeMap::new();
    for (&id, objs) in raw_objs.iter() {
        let tx_objs: Vec<_> = objs.iter().map(|o| RawObject::create(o)).collect();
        txs.insert(id, TxAddObjs { objs: tx_objs });
    }

    let keypair = crypto::gen_keypair();
    let client = reqwest::Client::new();
    for (_, tx) in txs.into_iter() {
        let tx_message = tx.sign(1, keypair.0, &keypair.1).into_raw();
        let res = client
            .post(&tx_url)
            .json(&json!({ "tx_body": tx_message }))
            .send()
            .await?;
        debug!("response: {:?}", &res);
        let tx_res = res.json::<TxResponse>().await?;
        info!("tx_hash={:?}", tx_res.tx_hash);

        loop {
            let res2 = client.get(&tx_url).query(&[("hash", tx_res.tx_hash.clone())]).send().await?;
            debug!("response: {:?}", &res2);
            let tx_info = res2.json::<serde_json::Value>().await?;
            if tx_info.get("type").unwrap() == &json!("committed") {
                break;
            }
            sleep(Duration::from_millis(100));
        }
    }

    Ok(())
}
