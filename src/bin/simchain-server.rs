#[macro_use]
extern crate log;

use std::path::{PathBuf};
use structopt::StructOpt;
use vchain::acc;
use vchain::chain::*;
use actix_web::{web, App, HttpRequest, HttpServer, Responder};
use std::fmt;
use anyhow::Context;
use futures::StreamExt;
use serde::Serialize;

static mut CHAIN: Option<SimChain> = None;

fn get_chain() -> &'static SimChain {
    unsafe { CHAIN.as_ref().unwrap() }
}

#[derive(Debug)]
struct MyErr(anyhow::Error);

impl fmt::Display for MyErr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "error: {}", self.0.to_string())
    }
}

fn handle_err<E: fmt::Display + fmt::Debug + Send + Sync + 'static>(e: E) -> MyErr {
    MyErr(anyhow::Error::msg(e))
}

impl actix_web::error::ResponseError for MyErr {}

macro_rules! impl_get_info {
    ($name: ident, $func: ident) => {
        async fn $name(req: HttpRequest) -> actix_web::Result<impl Responder> {
            let id = req.match_info().get("id").context("failed to get id")
                .map_err(handle_err)?
                .parse::<IdType>()
                .map_err(handle_err)?;
            info!("call {} with {}", stringify!($func), id);
            let data = get_chain().$func(id).map_err(handle_err)?;
            Ok(serde_json::to_string(&data))
        }
    };
}

impl_get_info!(web_get_blk_header, read_block_header);
impl_get_info!(web_get_blk_data, read_block_data);
impl_get_info!(web_get_intra_index_node, read_intra_index_node);
impl_get_info!(web_get_skip_list_node, read_skip_list_node);
impl_get_info!(web_get_object, read_object);

async fn web_get_param(_req: HttpRequest) -> actix_web::Result<impl Responder> {
    let data = get_chain().get_parameter().map_err(handle_err)?;
    Ok(serde_json::to_string(&data))
}

async fn web_query(mut body : web::Payload) -> actix_web::Result<impl Responder> {
    let mut bytes = web::BytesMut::new();
    while let Some(item) = body.next().await {
        bytes.extend_from_slice(&item?);
    }
    let query_req = serde_json::from_slice(&bytes).map_err(handle_err)?;
    let query = Query::from_json(&query_req).map_err(handle_err)?;

    let param = get_chain().get_parameter().map_err(handle_err)?;
    match param.acc_type {
        acc::Type::ACC1 => {
            let res: OverallResult<acc::Acc1Proof> = historical_query(&query, get_chain()).map_err(handle_err)?;
            Ok(serde_json::to_string(&res))
        }
        acc::Type::ACC2 => {
            let res: OverallResult<acc::Acc2Proof> = historical_query(&query, get_chain()).map_err(handle_err)?;
            Ok(serde_json::to_string(&res))
        }
    }
}

#[derive(Serialize)]
struct VerifyResponse {
    pass: bool,
    detail: VerifyResult,
    verify_time_in_ms: u128,
}

async fn web_verify(mut body: web::Payload) -> actix_web::Result<impl Responder> {
    let mut bytes = web::BytesMut::new();
    while let Some(item) = body.next().await {
        bytes.extend_from_slice(&item?);
    }

    let param = get_chain().get_parameter().map_err(handle_err)?;
    let (verify_result, time) = match param.acc_type {
        acc::Type::ACC1 => {
            let res: OverallResult<acc::Acc1Proof> = serde_json::from_slice(&bytes).map_err(handle_err)?;
            res.verify(get_chain())
        }
        acc::Type::ACC2 => {
            let res: OverallResult<acc::Acc2Proof> = serde_json::from_slice(&bytes).map_err(handle_err)?;
            res.verify(get_chain())
        }
    }.map_err(handle_err)?;
    let response = VerifyResponse {
        pass: verify_result == VerifyResult::Ok,
        detail: verify_result,
        verify_time_in_ms: time.as_millis(),
    };
    Ok(serde_json::to_string(&response))
}

#[derive(StructOpt, Debug)]
#[structopt(name = "simchain-server")]
struct Opts {
    /// input db path
    #[structopt(short = "-i", long, parse(from_os_str))]
    db: PathBuf,

    /// server binding address
    #[structopt(short, long, default_value = "127.0.0.1:8000")]
    binding: String,
}

#[actix_rt::main]
async fn main() -> actix_web::Result<()> {
    env_logger::init_from_env(env_logger::Env::default().filter_or("RUST_LOG", "info"));
    let opts = Opts::from_args();
    let chain = SimChain::open(&opts.db).map_err(handle_err)?;
    unsafe { CHAIN = Some(chain); }

    HttpServer::new(|| {
        App::new()
            .route("/get/param", web::get().to(web_get_param))
            .route("/get/blk_header/{id}", web::get().to(web_get_blk_header))
            .route("/get/blk_data/{id}", web::get().to(web_get_blk_data))
            .route("/get/intraindex/{id}", web::get().to(web_get_intra_index_node))
            .route("/get/skiplist/{id}", web::get().to(web_get_skip_list_node))
            .route("/get/obj/{id}", web::get().to(web_get_object))
            .route("/query", web::post().to(web_query))
            .route("/verify", web::post().to(web_verify))
    })
    .bind(opts.binding)?
        .run()
        .await?;

    Ok(())
}
