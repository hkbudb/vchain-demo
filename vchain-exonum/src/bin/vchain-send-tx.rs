#[macro_use]
extern crate log;

use anyhow::{bail, Result};
use exonum::{crypto, runtime::rust::Transaction};
use serde_json::json;
use std::path::{Path, PathBuf};
use structopt::StructOpt;
use vchain::acc;
use vchain_exonum::transactions::{TxAddObjs, TxSetParam};

fn parse_acc(input: &str) -> Result<acc::Type> {
    let input = input.to_ascii_lowercase();
    if input == "acc1" {
        Ok(acc::Type::ACC1)
    } else if input == "acc2" {
        Ok(acc::Type::ACC2)
    } else {
        bail!("invalid acc type, please specify as acc1 or acc2.");
    }
}

#[allow(clippy::box_vec)]
fn parse_v_bit_len(input: &str) -> Result<Box<Vec<i32>>> {
    let x = input
        .split(',')
        .map(|s| s.trim().parse::<i32>().map_err(anyhow::Error::msg))
        .collect::<Result<Vec<i32>>>()?;
    Ok(Box::new(x))
}

#[derive(StructOpt, Debug)]
#[structopt(name = "simchain-build")]
struct Opts {
    /// input data path
    #[structopt(short, long, parse(from_os_str))]
    input: PathBuf,

    /// api address
    #[structopt(short, long, default_value = "http://127.0.0.1:5000")]
    api: String,

    /// acc type to be used
    #[structopt(long, default_value = "acc2", parse(try_from_str = parse_acc))]
    acc: acc::Type,

    /// bit len for each dimension of the v data (e.g. 16,8)
    #[structopt(long, parse(try_from_str = parse_v_bit_len))]
    #[allow(clippy::box_vec)]
    bit_len: Box<Vec<i32>>,

    /// don't build intra index
    #[structopt(short = "-f", long)]
    no_intra_index: bool,

    /// max skip list level, 0 means no skip list.
    #[structopt(long, default_value = "0")]
    skip_list_max_level: i32,
}

#[actix_rt::main]
async fn main() -> Result<()> {
    env_logger::init_from_env(env_logger::Env::default().filter_or("RUST_LOG", "info"));

    let opts = Opts::from_args();
    let tx_param = TxSetParam {
        v_bit_len: opts.bit_len.to_vec(),
        is_acc2: opts.acc == acc::Type::ACC2,
        intra_index: !opts.no_intra_index,
        skip_list_max_level: opts.skip_list_max_level,
    };
    info!("param: {:?}", tx_param);

    let keypair = crypto::gen_keypair();
    let tx = tx_param.sign(1, keypair.0, &keypair.1).into_raw();

    let client = reqwest::Client::new();
    let res = client
        .post(format!("{}/api/explorer/v1/transactions", opts.api).as_str())
        .json(&json!({ "tx_body": tx }))
        .send()
        .await?;
    dbg!(res);

    Ok(())
}
