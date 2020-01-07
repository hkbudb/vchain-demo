use crate::{
    api::VChainApi,
    errors::Error,
    schema::VChainSchema,
    transactions::{InitParam, TxAddObjs},
};
use exonum::{
    crypto::Hash,
    runtime::{
        rust::{api::ServiceApiBuilder, CallContext, Service},
        BlockchainData, DispatcherError, ExecutionError,
    },
};
use exonum_merkledb::{BinaryValue, Snapshot};
use vchain::{Digest, Digestable, IdType, ReadInterface, WriteInterface};

#[exonum_interface]
pub trait VChainInterface {
    fn add_objs(&self, ctx: CallContext<'_>, arg: TxAddObjs) -> Result<(), Error>;
}

#[derive(Debug, ServiceFactory, ServiceDispatcher)]
#[service_dispatcher(implements("VChainInterface"))]
#[service_factory(proto_sources = "crate::proto")]
pub struct VChainService;

impl VChainInterface for VChainService {
    fn add_objs(&self, ctx: CallContext<'_>, arg: TxAddObjs) -> Result<(), Error> {
        let core = ctx.data().for_core();
        let block_id = core.height().0;
        warn!(
            "receive tx at blk #{} with {} objects",
            block_id,
            arg.objs.len()
        );
        let mut schema = VChainSchema::new(ctx.service_data());
        schema.objs_in_this_round.extend(arg.objs.iter().cloned());
        Ok(())
    }
}

impl Service for VChainService {
    fn initialize(&self, ctx: CallContext<'_>, params: Vec<u8>) -> Result<(), ExecutionError> {
        let param = InitParam::from_bytes(params.into())
            .map_err(DispatcherError::malformed_arguments)?
            .into_vchain_type();
        let mut schema = VChainSchema::new(ctx.service_data());
        schema.set_parameter(param).expect("failed to set param");
        Ok(())
    }

    fn state_hash(&self, data: BlockchainData<&dyn Snapshot>) -> Vec<Hash> {
        VChainSchema::new(data.for_executing_service()).state_hash()
    }

    fn wire_api(&self, builder: &mut ServiceApiBuilder) {
        VChainApi.wire(builder);
    }

    fn before_commit(&self, ctx: CallContext<'_>) {
        let core = ctx.data().for_core();
        let block_id = core.height().0 as IdType;
        let mut schema = VChainSchema::new(ctx.service_data());
        let objs: Vec<_> = schema
            .objs_in_this_round
            .into_iter()
            .map(|o| o.into_vchain_type(block_id))
            .collect();
        schema.objs_in_this_round.clear();
        let prev_block_id = block_id - 1;
        info!("commit blk #{} with {} objects", block_id, objs.len());
        let prev_hash = match schema.read_block_header(prev_block_id) {
            Ok(header) => header.to_digest(),
            _ => Digest::default(),
        };
        if let Err(e) = vchain::build_block(block_id, prev_hash, objs.iter(), &mut schema) {
            panic!("err when building new block: {:?}", e);
        }
    }
}
