#[macro_use]
extern crate log;

use anyhow::{bail, Error, Result};
use exonum::{
    api::backends::actix::AllowOrigin,
    blockchain::{config::GenesisConfigBuilder, ConsensusConfig, ValidatorKeys},
    crypto::{self, PublicKey, SecretKey},
    keys::Keys,
    node::{Node, NodeApiConfig, NodeConfig},
    runtime::{rust::ServiceFactory, RuntimeInstance},
};
use exonum_merkledb::{DbOptions, RocksDB};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use structopt::StructOpt;
use vchain::acc;
use vchain_exonum::{service::VChainService, transactions::InitParam};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct NodeKeys {
    consensus_key: (PublicKey, SecretKey),
    service_key: (PublicKey, SecretKey),
}

impl NodeKeys {
    fn new() -> Self {
        Self {
            consensus_key: crypto::gen_keypair(),
            service_key: crypto::gen_keypair(),
        }
    }

    fn load_from_file(path: &Path) -> Result<Self> {
        let data = fs::read_to_string(path)?;
        serde_json::from_str::<Self>(&data).map_err(Error::msg)
    }

    fn save_to_file(&self, path: &Path) -> Result<()> {
        let data = serde_json::to_string_pretty(self)?;
        fs::write(path, data)?;
        Ok(())
    }
}

fn node_config(api_address: String, peer_address: String, keys: NodeKeys) -> Result<NodeConfig> {
    info!("api address: {}", &api_address);
    info!("peer address: {}", &peer_address);

    let (consensus_public_key, consensus_secret_key) = keys.consensus_key;
    let (service_public_key, service_secret_key) = keys.service_key;

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
fn parse_v_bit_len(input: &str) -> Result<Box<Vec<u32>>> {
    let x = input
        .split(',')
        .map(|s| s.trim().parse::<u32>().map_err(anyhow::Error::msg))
        .collect::<Result<Vec<u32>>>()?;
    Ok(Box::new(x))
}

#[derive(StructOpt, Debug)]
#[structopt(name = "vchain-node")]
struct Opts {
    /// db path, should be a directory
    #[structopt(short = "-i", long, parse(from_os_str))]
    db: PathBuf,

    /// discard old database
    #[structopt(short = "-n", long)]
    create_new: bool,

    /// API Address
    #[structopt(long, default_value = "127.0.0.1:5000")]
    api_address: String,

    /// Peer Address
    #[structopt(long, default_value = "127.0.0.1:2000")]
    peer_address: String,

    /// acc type to be used
    #[structopt(long, default_value = "acc2", parse(try_from_str = parse_acc))]
    acc: acc::Type,

    /// bit len for each dimension of the v data (e.g. 16,8)
    #[structopt(long, parse(try_from_str = parse_v_bit_len))]
    #[allow(clippy::box_vec)]
    bit_len: Box<Vec<u32>>,

    /// don't build intra index
    #[structopt(short = "-f", long)]
    no_intra_index: bool,

    /// max skip list level, 0 means no skip list.
    #[structopt(long, default_value = "0")]
    skip_list_max_level: u32,
}

fn main() -> Result<()> {
    env_logger::init_from_env(
        env_logger::Env::default().filter_or("RUST_LOG", "vchain=info,vchain_exonum=info"),
    );

    let opts = Opts::from_args();

    let param = InitParam {
        v_bit_len: opts.bit_len.to_vec(),
        is_acc2: opts.acc == acc::Type::ACC2,
        intra_index: !opts.no_intra_index,
        skip_list_max_level: opts.skip_list_max_level,
    };
    info!("param: {:?}", param);

    info!("db path: {:?}", opts.db);
    if opts.create_new && opts.db.exists() {
        fs::remove_dir_all(&opts.db)?;
    }
    fs::create_dir_all(&opts.db)?;

    let key = match NodeKeys::load_from_file(&opts.db.join("keys.json")) {
        Ok(key) => {
            info!("found old key");
            key
        }
        _ => {
            warn!("create new key");
            let key = NodeKeys::new();
            key.save_to_file(&opts.db.join("keys.json"))?;
            key
        }
    };
    let db = RocksDB::open(opts.db, &DbOptions::default()).map_err(anyhow::Error::msg)?;

    let external_runtimes: Vec<RuntimeInstance> = vec![];
    let service = VChainService;
    let artifact_id = service.artifact_id();
    let services = vec![service.into()];
    let node_config = node_config(opts.api_address, opts.peer_address, key)?;
    let genesis_config = GenesisConfigBuilder::with_consensus_config(node_config.consensus.clone())
        .with_artifact(artifact_id.clone())
        .with_instance(
            artifact_id
                .into_default_instance(1, "vchain")
                .with_constructor(param),
        )
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
