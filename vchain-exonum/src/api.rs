use crate::schema::VChainSchema;
use exonum::runtime::rust::api::{self, ServiceApiBuilder, ServiceApiState};
use vchain::{IdType, ReadInterface};

#[derive(Debug, Clone, Copy)]
pub struct VChainApi;

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
        id: IdType,
    ) -> api::Result<vchain::Object> {
        let schema = VChainSchema::new(state.service_data());
        schema
            .read_object(id)
            .map_err(|e| api::Error::NotFound(format!("{:?}", e)))
    }

    pub fn wire(self, builder: &mut ServiceApiBuilder) {
        builder
            .public_scope()
            .endpoint(
                "get/param",
                move |state: &ServiceApiState<'_>, _query: ()| self.get_param(state),
            )
            .endpoint(
                "get/object",
                move |state: &ServiceApiState<'_>, id: IdType| self.get_object(state, id),
            );
    }
}
