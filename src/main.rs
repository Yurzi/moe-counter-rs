mod banner;
mod cli;
mod db_adpater;
mod utils;

use std::{
    sync::{atomic::AtomicBool, Arc},
    time::Duration,
};

use axum::{
    body::Body,
    extract::{Path, Query, State},
    http::{Response, StatusCode},
    response::{Html, IntoResponse},
    routing::get,
    Router,
};
use banner::ThemeManager;
use clap::Parser;
use cli::read_config;
use db_adpater::DBManager;
use serde::{Deserialize, Serialize};
use tokio::{signal, time};

async fn status() -> String {
    "everything is ok".to_string()
}

#[derive(Serialize, Deserialize)]
struct CountGetParams {
    theme: Option<String>,
    format: Option<String>,
    length: Option<u32>,
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
    let config = app_state.config.clone();

    let request_theme = params.theme.unwrap_or(config.default_theme.clone());
    let request_format = params.format.unwrap_or(config.default_format.clone());
    let request_len = params.length.unwrap_or(0);
    let digit_count = config.digit_count.max(request_len);

    let theme_manager = &app_state.theme_manager;

    let theme = theme_manager.get(&request_theme).unwrap_or(
        theme_manager
            .get(&config.default_theme)
            .unwrap_or(theme_manager.get("moebooru").unwrap()),
    );

    let db_manager = &app_state.db_manager;
    let number = db_manager.count(&key).await.unwrap_or(0);

    println!(
        "[GET] /{} | theme: {}, format: {}, length: {}, count: {}",
        key, request_theme, request_format, digit_count, number
    );

    let response = match request_format.as_str() {
        "webp" => {
            let image = theme.gen_webp(number, digit_count);
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
            let image = theme.gen_svg(number, digit_count, config.pixelated);
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

async fn demo(
    Query(params): Query<CountGetParams>,
    State(app_state): State<SharedState>,
) -> impl IntoResponse {
    let config = app_state.config.clone();

    let request_theme = params.theme.unwrap_or(config.default_theme.clone());
    let request_format = params.format.unwrap_or(config.default_format.clone());

    let digit_count = 10;
    let number = 0123456789;

    let theme_manager = &app_state.theme_manager;
    let theme = theme_manager.get(&request_theme).unwrap_or(
        theme_manager
            .get(&config.default_theme)
            .unwrap_or(theme_manager.get("moebooru").unwrap()),
    );
    println!(
        "[GET] /{} | theme: {}, format: {}, length: {}, count: {}",
        "demo", request_theme, request_format, digit_count, number
    );

    let response = match request_format.as_str() {
        "webp" => {
            let image = theme.gen_webp(number, digit_count);
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
            let image = theme.gen_svg(number, digit_count, config.pixelated);
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

async fn favicon() -> impl IntoResponse {
    let favicon = Vec::from(include_bytes!("../assets/favicon.png"));
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", image::ImageFormat::Png.to_mime_type())
        .body(Body::from(favicon))
        .unwrap()
}

struct AppState {
    config: cli::Config,
    theme_manager: ThemeManager,
    db_manager: DBManager,
    should_exit: AtomicBool,
}

impl AppState {
    fn new(config: cli::Config, theme_manager: ThemeManager, db_manager: DBManager) -> Self {
        AppState {
            config,
            theme_manager,
            db_manager,
            should_exit: AtomicBool::new(false),
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

    let mut db_manager = DBManager::new(db_adpater::SqliteClient::new(
        &cfg.sqlite.path,
        &cfg.sqlite.table_name,
    ));

    db_manager.init().await.expect("failed to init database");

    let shared_state = SharedState::new(AppState::new(cfg.clone(), theme_manager, db_manager));

    // initialize tracing
    tracing_subscriber::fmt::init();
    let app = Router::new()
        .route(
            "/",
            get(|| async { Html::from("Ciallo～(∠・ω< )⌒★, this is a Moe Counter. Please see <a href=\"https://github.com/Yurzi/moe-counter-rs\">Moe-Counter-Rs</a> for details") }),
        )
        .route("/status", get(status))
        .route("/favicon.ico", get(favicon))
        .route("/demo", get(demo))
        .route("/:key", get(count))
        .with_state(shared_state.clone());
    let listener = tokio::net::TcpListener::bind(format!("{}:{}", cfg.listen, cfg.port))
        .await
        .unwrap();

    // register a loop task for sync backend
    let local_state = shared_state.clone();
    let sync_to_backend_handle = tokio::spawn(async move {
        let mut interval = time::interval(Duration::from_secs(15 * 60));

        while !local_state
            .should_exit
            .load(std::sync::atomic::Ordering::Relaxed)
        {
            interval.tick().await;
            let ret = local_state.db_manager.sync_to_backend().await;
            println!("[Info] sync with backend");
            if ret.is_err() {
                println!("[Warn] unable to sync with backend");
            }
        }
    });

    println!("listen on: http://{}:{}", cfg.listen, cfg.port);
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal(shared_state.clone()))
        .await
        .unwrap();

    // cancel timer job
    sync_to_backend_handle.abort();

    println!("[Shutdown]")
}

#[cfg(target_os = "windows")]
async fn shutdown_signal(app_state: SharedState) {
    signal::ctrl_c().await.expect("Failed to listen to ctrl-c");
    app_state
        .should_exit
        .store(true, std::sync::atomic::Ordering::Relaxed);
    let _ = app_state.db_manager.sync_to_backend().await;
}
#[cfg(target_os = "linux")]
async fn shutdown_signal(app_state: SharedState) {
    let mut stream = signal::unix::signal(signal::unix::SignalKind::terminate()).unwrap();
    tokio::select! {
        _ = stream.recv() => {}
        _ = tokio::signal::ctrl_c() => {}
    }
    app_state
        .should_exit
        .store(true, std::sync::atomic::Ordering::Relaxed);
    let _ = app_state.db_manager.sync_to_backend().await;
}
