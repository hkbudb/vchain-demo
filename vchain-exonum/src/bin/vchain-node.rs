#[macro_use]
extern crate log;

use anyhow::{bail, Result};
use exonum::{
    api::backends::actix::AllowOrigin,
    blockchain::{config::GenesisConfigBuilder, ConsensusConfig, ValidatorKeys},
    keys::Keys,
    node::{Node, NodeApiConfig, NodeConfig},
    runtime::{rust::ServiceFactory, RuntimeInstance},
};
use exonum_merkledb::{DbOptions, RocksDB};
use std::path::PathBuf;
use structopt::StructOpt;
use vchain::acc;
use vchain_exonum::contracts::VChainService;

fn node_config(api_address: String, peer_address: String) -> Result<NodeConfig> {
    info!("api address: {}", &api_address);
    info!("peer address: {}", &peer_address);

    let (consensus_public_key, consensus_secret_key) = exonum::crypto::gen_keypair();
    let (service_public_key, service_secret_key) = exonum::crypto::gen_keypair();

    let consensus = ConsensusConfig {
        validator_keys: vec![ValidatorKeys {
            consensus_key: consensus_public_key,
            service_key: service_public_key,
        }],
        ..ConsensusConfig::default()
    };

    let api_cfg = NodeApiConfig {
        public_api_address: Some(api_address.parse()?),
        public_allow_origin: Some(AllowOrigin::Any),
        ..Default::default()
    };

    Ok(NodeConfig {
        listen_address: peer_address.parse()?,
        consensus,
        external_address: peer_address.to_owned(),
        network: Default::default(),
        connect_list: Default::default(),
        api: api_cfg,
        mempool: Default::default(),
        services_configs: Default::default(),
        database: Default::default(),
        thread_pool_size: Default::default(),
        master_key_path: Default::default(),
        keys: Keys::from_keys(
            consensus_public_key,
            consensus_secret_key,
            service_public_key,
            service_secret_key,
        ),
    })
}

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
#[structopt(name = "vchain-node")]
struct Opts {
    /// db path
    #[structopt(short = "-i", long, parse(from_os_str))]
    db: PathBuf,

    /// acc type to be used
    #[structopt(long, default_value = "acc2", parse(try_from_str = parse_acc))]
    acc: acc::Type,

    /// bit len for each dimension of the v data (e.g. 16,8)
    #[structopt(long, parse(try_from_str = parse_v_bit_len))]
    #[allow(clippy::box_vec)]
    bit_len: Box<Vec<u8>>,

    /// don't build intra index
    #[structopt(short = "-f", long)]
    no_intra_index: bool,

    /// max skip list level, 0 means no skip list.
    #[structopt(long, default_value = "0")]
    skip_list_max_level: vchain::SkipLstLvlType,

    /// API Address
    #[structopt(long, default_value = "127.0.0.1:5000")]
    api_address: String,

    /// Peer Address
    #[structopt(long, default_value = "127.0.0.1:2000")]
    peer_address: String,
}

fn main() -> Result<()> {
    env_logger::init_from_env(
        env_logger::Env::default().filter_or("RUST_LOG", "vchain=info,vchain_exonum=info"),
    );

    let opts = Opts::from_args();
    let param = vchain::Parameter {
        v_bit_len: opts.bit_len.to_vec(),
        acc_type: opts.acc,
        use_sk: false,
        intra_index: !opts.no_intra_index,
        skip_list_max_level: opts.skip_list_max_level,
    };
    info!("param: {:?}", param);
    info!("open db: {:?}", opts.db);
    let db = RocksDB::open(opts.db, &DbOptions::default()).map_err(anyhow::Error::msg)?;

    let external_runtimes: Vec<RuntimeInstance> = vec![];
    let service = VChainService;
    let artifact_id = service.artifact_id();
    let services = vec![service.into()];
    let node_config = node_config(opts.api_address, opts.peer_address)?;
    let genesis_config = GenesisConfigBuilder::with_consensus_config(node_config.consensus.clone())
        .with_artifact(artifact_id.clone())
        .with_instance(artifact_id.into_default_instance(1, "vchain"))
        .build();

    let node = Node::new(
        db,
        external_runtimes,
        services,
        node_config,
        genesis_config,
        None,
    );
    info!("Starting a single node...");
    info!("Blockchain is ready for transactions!");
    node.run().map_err(anyhow::Error::msg)
}
