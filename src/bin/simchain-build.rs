#[macro_use]
extern crate log;

use anyhow::{bail, Result};
use std::path::{Path, PathBuf};
use structopt::StructOpt;
use vchain::acc;
use vchain::chain::*;
use vchain::digest::{Digest, Digestable};

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
fn parse_v_bit_len(input: &str) -> Result<Box<Vec<u8>>> {
    let x = input
        .split(',')
        .map(|s| s.trim().parse::<u8>().map_err(anyhow::Error::msg))
        .collect::<Result<Vec<u8>>>()?;
    Ok(Box::new(x))
}

#[derive(StructOpt, Debug)]
#[structopt(name = "simchain-build")]
struct Opts {
    /// input data path
    #[structopt(short, long, parse(from_os_str))]
    input: PathBuf,

    /// output db path, should be a directory
    #[structopt(short, long, parse(from_os_str))]
    output: PathBuf,

    /// acc type to be used
    #[structopt(long, default_value = "acc2", parse(try_from_str = parse_acc))]
    acc: acc::Type,

    /// bit len for each dimension of the v data (e.g. 16,8)
    #[structopt(long, parse(try_from_str = parse_v_bit_len))]
    #[allow(clippy::box_vec)]
    bit_len: Box<Vec<u8>>,

    /// use sk to build chain
    #[structopt(short = "-s", long)]
    use_sk: bool,

    /// don't build intra index
    #[structopt(short = "-f", long)]
    no_intra_index: bool,

    /// max skip list level, 0 means no skip list.
    #[structopt(long, default_value = "0")]
    skip_list_max_level: SkipLstLvlType,
}

fn build_chain(data_path: &Path, out_path: &Path, param: &Parameter) -> Result<()> {
    info!("build chain using data from {:?}", data_path);
    info!("out path: {:?}", out_path);
    info!("param: {:?}", param);

    let raw_objs = load_raw_obj_from_file(data_path)?;
    let mut chain = SimChain::create(out_path, param.clone())?;
    chain.set_parameter(param.clone())?;

    let mut prev_hash = Digest::default();
    for (id, objs) in raw_objs.iter() {
        if id % 1000 == 0 {
            info!("build blk #{}", id);
        }
        let header = build_block(*id, prev_hash, objs.iter(), &mut chain)?;
        prev_hash = header.to_digest();
    }

    // overwrite use_sk
    if param.use_sk {
        let mut new_param = param.clone();
        new_param.use_sk = false;
        chain.set_parameter(new_param)?;
    }
    Ok(())
}

fn main() -> Result<()> {
    env_logger::init_from_env(env_logger::Env::default().filter_or("RUST_LOG", "info"));

    let opts = Opts::from_args();
    let param = Parameter {
        v_bit_len: opts.bit_len.to_vec(),
        acc_type: opts.acc,
        use_sk: opts.use_sk,
        intra_index: !opts.no_intra_index,
        skip_list_max_level: opts.skip_list_max_level,
    };

    build_chain(&opts.input, &opts.output, &param)?;

    Ok(())
}
