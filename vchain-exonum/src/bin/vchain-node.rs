use exonum_cli::NodeBuilder;
use vchain_exonum::contracts::VChainService;

fn main() -> Result<(), failure::Error> {
    env_logger::init_from_env(
        env_logger::Env::default().filter_or("RUST_LOG", "vchain=info,vchain_exonum=info"),
    );

    NodeBuilder::new().with_service(VChainService).run()
}
