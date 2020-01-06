use crate::schema::VChainSchema;
use exonum::runtime::rust::api::{self, ServiceApiBuilder, ServiceApiState};
use vchain::{IdType, ReadInterface};

#[derive(Debug, Clone, Copy)]
pub struct VChainApi;

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub struct QueryInput {
    pub id: IdType,
}

impl VChainApi {
    pub fn get_param(self, state: &ServiceApiState<'_>) -> api::Result<vchain::Parameter> {
        let schema = VChainSchema::new(state.service_data());
        schema
            .get_parameter()
            .map_err(|e| api::Error::NotFound(format!("{:?}", e)))
    }

    pub fn get_object(
        self,
        state: &ServiceApiState<'_>,
        query: QueryInput,
    ) -> api::Result<vchain::Object> {
        let schema = VChainSchema::new(state.service_data());
        schema
            .read_object(query.id)
            .map_err(|e| api::Error::NotFound(format!("{:?}", e)))
    }

    pub fn get_block_header(
        self,
        state: &ServiceApiState<'_>,
        query: QueryInput,
    ) -> api::Result<vchain::BlockHeader> {
        let schema = VChainSchema::new(state.service_data());
        schema
            .read_block_header(query.id)
            .map_err(|e| api::Error::NotFound(format!("{:?}", e)))
    }

    pub fn get_block_data(
        self,
        state: &ServiceApiState<'_>,
        query: QueryInput,
    ) -> api::Result<vchain::BlockData> {
        let schema = VChainSchema::new(state.service_data());
        schema
            .read_block_data(query.id)
            .map_err(|e| api::Error::NotFound(format!("{:?}", e)))
    }

    pub fn get_intra_index_node(
        self,
        state: &ServiceApiState<'_>,
        query: QueryInput,
    ) -> api::Result<vchain::IntraIndexNode> {
        let schema = VChainSchema::new(state.service_data());
        schema
            .read_intra_index_node(query.id)
            .map_err(|e| api::Error::NotFound(format!("{:?}", e)))
    }

    pub fn get_skip_list_node(
        self,
        state: &ServiceApiState<'_>,
        query: QueryInput,
    ) -> api::Result<vchain::SkipListNode> {
        let schema = VChainSchema::new(state.service_data());
        schema
            .read_skip_list_node(query.id)
            .map_err(|e| api::Error::NotFound(format!("{:?}", e)))
    }

    pub fn get_index_node(
        self,
        state: &ServiceApiState<'_>,
        query: QueryInput,
    ) -> api::Result<serde_json::Value> {
        match self.get_intra_index_node(state, query) {
            Ok(data) => serde_json::to_value(data)
                .map_err(|e| api::Error::InternalError(failure::format_err!("{:?}", e))),
            _ => {
                let data = self.get_skip_list_node(state, query).map_err(|_| {
                    api::Error::NotFound(format!("no index node for id: {}", query.id))
                })?;
                serde_json::to_value(data)
                    .map_err(|e| api::Error::InternalError(failure::format_err!("{:?}", e)))
            }
        }
    }

    pub fn wire(self, builder: &mut ServiceApiBuilder) {
        builder
            .public_scope()
            .endpoint(
                "get/param",
                move |state: &ServiceApiState<'_>, _query: ()| self.get_param(state),
            )
            .endpoint(
                "get/obj",
                move |state: &ServiceApiState<'_>, query: QueryInput| self.get_object(state, query),
            )
            .endpoint(
                "get/blk_header",
                move |state: &ServiceApiState<'_>, query: QueryInput| {
                    self.get_block_header(state, query)
                },
            )
            .endpoint(
                "get/blk_data",
                move |state: &ServiceApiState<'_>, query: QueryInput| {
                    self.get_block_data(state, query)
                },
            )
            .endpoint(
                "get/intraindex",
                move |state: &ServiceApiState<'_>, query: QueryInput| {
                    self.get_intra_index_node(state, query)
                },
            )
            .endpoint(
                "get/skiplist",
                move |state: &ServiceApiState<'_>, query: QueryInput| {
                    self.get_skip_list_node(state, query)
                },
            )
            .endpoint(
                "get/index",
                move |state: &ServiceApiState<'_>, query: QueryInput| {
                    self.get_index_node(state, query)
                },
            );
    }
}
