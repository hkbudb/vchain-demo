#[macro_use]
extern crate log;

use anyhow::{bail, Result};
use exonum::{crypto, runtime::rust::Transaction};
use serde_json::json;
use std::path::{Path, PathBuf};
use structopt::StructOpt;
use vchain_exonum::transactions::{TxAddObjs};

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

#[actix_rt::main]
async fn main() -> Result<()> {
    env_logger::init_from_env(env_logger::Env::default().filter_or("RUST_LOG", "info"));

    let opts = Opts::from_args();
    // let tx_param = TxSetParam {
    //     v_bit_len: opts.bit_len.to_vec(),
    //     is_acc2: opts.acc == acc::Type::ACC2,
    //     intra_index: !opts.no_intra_index,
    //     skip_list_max_level: opts.skip_list_max_level,
    // };
    // info!("param: {:?}", tx_param);

    // let keypair = crypto::gen_keypair();
    // let tx = tx_param.sign(1, keypair.0, &keypair.1).into_raw();

    // let client = reqwest::Client::new();
    // let res = client
    //     .post(format!("{}/api/explorer/v1/transactions", opts.api).as_str())
    //     .json(&json!({ "tx_body": tx }))
    //     .send()
    //     .await?;
    // dbg!(res);

    Ok(())
}
