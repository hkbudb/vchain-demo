#[macro_use]
extern crate log;

use std::path::{PathBuf};
use structopt::StructOpt;
use vchain::acc;
use vchain::chain::*;
use actix_web::{web, App, HttpRequest, HttpServer, Responder};
use std::fmt;
use anyhow::Context;

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
    })
    .bind(opts.binding)?
        .run()
        .await?;

    Ok(())
}
