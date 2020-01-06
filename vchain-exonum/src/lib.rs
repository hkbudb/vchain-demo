#[macro_use]
extern crate serde_derive; // Required for Protobuf
#[macro_use]
extern crate exonum_derive;
#[macro_use]
extern crate log;

pub mod contracts;
pub mod proto;
pub mod schema;
pub mod transactions;
pub mod api;

pub mod errors {
    #[derive(Debug, IntoExecutionError)]
    pub enum Error {
        Unknown = 1,
    }
}
