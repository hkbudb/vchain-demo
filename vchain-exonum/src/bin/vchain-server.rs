use actix_cors::Cors;
use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use futures::StreamExt;
use serde::Serialize;
use std::fmt;
use structopt::StructOpt;
use vchain::acc;
use vchain::chain::*;

static mut API_ADDRESS: Option<String> = None;

fn get_api_address() -> &'static str {
    unsafe { API_ADDRESS.as_ref().unwrap() }
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

async fn web_get_param() -> impl Responder {
    HttpResponse::TemporaryRedirect()
        .header("Location", format!("{}/get/param", get_api_address()))
        .finish()
}

macro_rules! impl_get_info {
    ($name: ident, $url: expr) => {
        async fn $name(req: web::Path<(IdType,)>) -> impl Responder {
            let id = req.0;
            HttpResponse::TemporaryRedirect()
                .header(
                    "Location",
                    format!("{}/get/{}?id={}", get_api_address(), $url, id),
                )
                .finish()
        }
    };
}

impl_get_info!(web_get_blk_header, "blk_header");
impl_get_info!(web_get_blk_data, "blk_data");
impl_get_info!(web_get_intra_index_node, "intraindex");
impl_get_info!(web_get_index_node, "index");
impl_get_info!(web_get_skip_list_node, "skiplist");
impl_get_info!(web_get_object, "obj");

async fn web_query() -> impl Responder {
    HttpResponse::TemporaryRedirect()
        .header("Location", format!("{}/query", get_api_address()))
        .finish()
}

#[derive(Serialize)]
struct VerifyResponse {
    pass: bool,
    detail: VerifyResult,
    verify_time_in_ms: u64,
}

#[derive(Debug, Clone)]
struct LightChain {
    param_api: String,
    blk_header_api: String,
}

impl LightChain {
    fn new(api_address: &str) -> Self {
        Self {
            param_api: format!("{}/get/param", api_address),
            blk_header_api: format!("{}/get/blk_header", api_address),
        }
    }
}

#[async_trait::async_trait]
impl LightNodeInterface for LightChain {
    async fn lightnode_get_parameter(&self) -> anyhow::Result<Parameter> {
        reqwest::get(&self.param_api)
            .await?
            .json::<Parameter>()
            .await
            .map_err(anyhow::Error::msg)
    }

    async fn lightnode_read_block_header(&self, id: IdType) -> anyhow::Result<BlockHeader> {
        let client = reqwest::Client::new();
        client
            .get(&self.blk_header_api)
            .query(&[("id", id)])
            .send()
            .await?
            .json::<BlockHeader>()
            .await
            .map_err(anyhow::Error::msg)
    }
}

async fn web_verify(mut body: web::Payload) -> actix_web::Result<impl Responder> {
    let mut bytes = web::BytesMut::new();
    while let Some(item) = body.next().await {
        bytes.extend_from_slice(&item?);
    }

    let lightnode = LightChain::new(get_api_address());
    let param = lightnode
        .lightnode_get_parameter()
        .await
        .map_err(handle_err)?;
    let (verify_result, time) = match param.acc_type {
        acc::Type::ACC1 => {
            let res: OverallResult<acc::Acc1Proof> =
                serde_json::from_slice(&bytes).map_err(handle_err)?;
            res.verify(&lightnode).await
        }
        acc::Type::ACC2 => {
            let res: OverallResult<acc::Acc2Proof> =
                serde_json::from_slice(&bytes).map_err(handle_err)?;
            res.verify(&lightnode).await
        }
    }
    .map_err(handle_err)?;
    let response = VerifyResponse {
        pass: verify_result == VerifyResult::Ok,
        detail: verify_result,
        verify_time_in_ms: time.as_millis() as u64,
    };
    Ok(HttpResponse::Ok().json(response))
}

#[derive(StructOpt, Debug)]
#[structopt(name = "vchain-server")]
struct Opts {
    /// api address
    #[structopt(short, long, default_value = "http://127.0.0.1:5000")]
    api_address: String,

    /// server binding address
    #[structopt(short, long, default_value = "127.0.0.1:8000")]
    binding: String,
}

#[actix_rt::main]
async fn main() -> actix_web::Result<()> {
    env_logger::init_from_env(env_logger::Env::default().filter_or("RUST_LOG", "info"));
    let opts = Opts::from_args();
    let api_address = format!("{}/api/services/vchain", opts.api_address);
    unsafe {
        API_ADDRESS = Some(api_address);
    }

    HttpServer::new(|| {
        App::new()
            .wrap(
                Cors::new()
                .send_wildcard()
                .allowed_methods(vec!["GET", "POST"])
                .finish(),
            )
            .route("/get/param", web::get().to(web_get_param))
            .route("/get/blk_header/{id}", web::get().to(web_get_blk_header))
            .route("/get/blk_data/{id}", web::get().to(web_get_blk_data))
            .route(
                "/get/intraindex/{id}",
                web::get().to(web_get_intra_index_node),
            )
            .route("/get/skiplist/{id}", web::get().to(web_get_skip_list_node))
            .route("/get/index/{id}", web::get().to(web_get_index_node))
            .route("/get/obj/{id}", web::get().to(web_get_object))
            .route("/query", web::post().to(web_query))
            .route("/verify", web::post().to(web_verify))
    })
    .bind(opts.binding)?
        .run()
        .await?;

    Ok(())
}
