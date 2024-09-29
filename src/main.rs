mod banner;
mod cli;
mod db_adpater;
mod utils;

use std::sync::{Arc, RwLock};

use axum::{
    body::Body,
    extract::{Path, Query, State},
    http::{Response, StatusCode},
    response::IntoResponse,
    routing::get,
    Router,
};
use banner::ThemeManager;
use clap::Parser;
use cli::read_config;
use serde::{Deserialize, Serialize};

async fn status() -> String {
    "everything is ok".to_string()
}

#[derive(Serialize, Deserialize)]
struct CountGetParams {
    theme: Option<String>,
    format: Option<String>,
}

fn request_err(msg: &str) -> Response<Body> {
    Response::builder()
        .status(StatusCode::BAD_REQUEST)
        .body(Body::from(msg.to_string()))
        .unwrap()
}

fn internal_err(msg: &str) -> Response<Body> {
    Response::builder()
        .status(StatusCode::INTERNAL_SERVER_ERROR)
        .body(Body::from(msg.to_string()))
        .unwrap()
}

async fn count(
    Path(key): Path<String>,
    Query(params): Query<CountGetParams>,
    State(app_state): State<SharedState>,
) -> impl IntoResponse {
    let config = app_state.config.read();

    if config.is_err() {
        return internal_err("");
    }
    let config = config.unwrap();

    let request_theme = params.theme.unwrap_or(config.default_theme.clone());
    let request_format = params.format.unwrap_or(config.default_format.clone());

    println!(
        "[GET] /{} with theme: {}, format: {}",
        key, request_theme, request_format
    );

    let theme_manager = app_state.theme_manager.read();
    if theme_manager.is_err() {
        return internal_err("failed to get themes");
    }
    let theme_manager = theme_manager.unwrap();

    let theme = theme_manager
        .get(&request_theme)
        .unwrap_or(theme_manager.get(&config.default_theme).unwrap());

    let number = 114514;

    let response = match request_format.as_str() {
        "webp" => {
            let image = theme.gen_webp(number, config.digit_count);
            if image.is_err() {
                return internal_err("failed to gen webp image");
            }
            let image = image.unwrap();

            let image_data = image.encode();
            if image_data.is_err() {
                return internal_err("failed to get webp image data");
            }

            let image_data = image_data.unwrap();
            Response::builder()
                .status(StatusCode::OK)
                .header("Content-Type", image.format().to_mime_type())
                .body(Body::from(image_data))
                .unwrap()
        }
        _ => {
            let image = theme.gen_svg(number, config.digit_count, config.pixelated);
            if image.is_err() {
                return internal_err("failed to gen svg image");
            }
            let image = image.unwrap();
            Response::builder()
                .status(StatusCode::OK)
                .header("Content-Type", "image/svg+xml")
                .body(Body::from(image.data().to_string()))
                .unwrap()
        }
    };

    response
}

struct AppState {
    config: RwLock<cli::Config>,
    theme_manager: RwLock<ThemeManager>,
}

impl AppState {
    fn new(config: cli::Config, theme_manager: ThemeManager) -> Self {
        AppState {
            config: RwLock::new(config),
            theme_manager: RwLock::new(theme_manager),
        }
    }
}

type SharedState = Arc<AppState>;

#[tokio::main]
async fn main() {
    // cli args parase
    let args = cli::CliArgs::parse();
    let cfg = read_config(&args.config_path);

    // init
    let theme_manager = ThemeManager::new(&cfg.themes_dir).expect("failed to load themes");

    let shared_state = SharedState::new(AppState::new(cfg.clone(), theme_manager));

    // initialize tracing
    tracing_subscriber::fmt::init();
    let app = Router::new()
        .route("/status", get(status))
        .route("/:key", get(count))
        .with_state(shared_state);
    let listener = tokio::net::TcpListener::bind(format!("{}:{}", cfg.listen, cfg.port))
        .await
        .unwrap();

    println!("listen on: http://{}:{}", cfg.listen, cfg.port);
    axum::serve(listener, app).await.unwrap();
}
