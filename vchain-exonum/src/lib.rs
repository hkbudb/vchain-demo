#[macro_use]
extern crate serde_derive; // Required for Protobuf
#[macro_use]
extern crate exonum_derive;
#[macro_use]
extern crate log;

pub mod api;
pub mod proto;
pub mod schema;
pub mod service;
pub mod transactions;

pub mod errors {
    #[derive(Debug, IntoExecutionError)]
    pub enum Error {
        Unknown = 1,
    }
}

#[cfg(test)]
mod tests;
