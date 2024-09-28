mod banner;
mod cli;
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

async fn count(
    Path(key): Path<String>,
    Query(params): Query<CountGetParams>,
    State(app_state): State<SharedState>,
) -> impl IntoResponse {
    // log request

    let app_state = app_state.try_read();
    if app_state.is_err() {
        return Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(Body::from(""))
            .unwrap();
    }

    let app_state = app_state.unwrap();
    let config = app_state.config.clone();

    let request_theme = params.theme.unwrap_or(config.default_theme);
    let request_format = params.format.unwrap_or(config.default_format);
    println!(
        "[GET] /{} with theme: {}, format: {}",
        key, request_theme, request_format
    );

    let theme = app_state.theme_manager.get(&request_theme);
    if theme.is_err() {
        return Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(Body::from("Theme Not Found"))
            .unwrap();
    }

    let theme = theme.unwrap();

    let number = 114514;

    let response = match request_format.as_str() {
        "webp" => {
            let image = theme.gen_webp(number, config.digit_count);
            if image.is_err() {
                return Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body(Body::from("failed to gen webp image"))
                    .unwrap();
            }
            let image = image.unwrap();

            let image_data = image.encode();
            if image_data.is_err() {
                return Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body(Body::from("failed to get webp image data"))
                    .unwrap();
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
                return Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body(Body::from("failed to gen svg image"))
                    .unwrap();
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
    config: cli::Config,
    theme_manager: ThemeManager,
}

impl AppState {
    fn new(config: cli::Config, theme_manager: ThemeManager) -> Self {
        AppState {
            config,
            theme_manager,
        }
    }
}

type SharedState = Arc<RwLock<AppState>>;

#[tokio::main]
async fn main() {
    // cli args parase
    let args = cli::CliArgs::parse();
    let cfg = read_config(&args.config_path);

    // init
    let theme_manager = ThemeManager::new(&cfg.themes_dir).expect("failed to load themes");

    let shared_state = SharedState::new(RwLock::new(AppState::new(cfg.clone(), theme_manager)));

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
