//! Weasis web backend: serves the frontend and acts as the gateway to DICOM servers.
//!
//! Configuration (environment variables):
//! - `WEASIS_ADDR`: listening address, default `127.0.0.1:8080`.
//! - `WEASIS_WEB_DIST`: folder with the built frontend (`trunk build`), default
//!   `crates/weasis-web/dist` relative to the current directory.
//! - `RUST_LOG`: log filter, default `info`.
//!
//! Java reference: `weasis-launcher` (application start-up and configuration).

use std::net::SocketAddr;
use std::path::PathBuf;

use axum::Router;
use tower_http::services::{ServeDir, ServeFile};
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;

const DEFAULT_ADDR: &str = "127.0.0.1:8080";
const DEFAULT_WEB_DIST: &str = "crates/weasis-web/dist";

/// Server settings read from the environment.
struct Config {
    addr: SocketAddr,
    web_dist: PathBuf,
}

impl Config {
    fn from_env() -> Result<Self, String> {
        let addr_text = std::env::var("WEASIS_ADDR").unwrap_or_else(|_| DEFAULT_ADDR.to_owned());
        let addr = addr_text
            .parse()
            .map_err(|error| format!("invalid WEASIS_ADDR '{addr_text}': {error}"))?;
        let web_dist = std::env::var("WEASIS_WEB_DIST")
            .unwrap_or_else(|_| DEFAULT_WEB_DIST.to_owned())
            .into();
        Ok(Self { addr, web_dist })
    }
}

/// Serves the frontend files; unknown paths fall back to `index.html` so the
/// browser-side router can handle them.
fn router(web_dist: PathBuf) -> Router {
    let index = ServeFile::new(web_dist.join("index.html"));
    let frontend = ServeDir::new(web_dist).fallback(index);
    Router::new()
        .fallback_service(frontend)
        .layer(TraceLayer::new_for_http())
}

#[tokio::main]
async fn main() -> Result<(), String> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();

    let config = Config::from_env()?;
    if !config.web_dist.join("index.html").is_file() {
        tracing::warn!(
            "no frontend build in {}; run `trunk build` in crates/weasis-web",
            config.web_dist.display()
        );
    }

    let listener = tokio::net::TcpListener::bind(config.addr)
        .await
        .map_err(|error| format!("cannot listen on {}: {error}", config.addr))?;
    tracing::info!("Weasis server listening on http://{}", config.addr);

    axum::serve(listener, router(config.web_dist))
        .with_graceful_shutdown(shutdown_signal())
        .await
        .map_err(|error| format!("server error: {error}"))
}

/// Resolves when the process receives Ctrl+C, so in-flight requests can finish.
async fn shutdown_signal() {
    if let Err(error) = tokio::signal::ctrl_c().await {
        tracing::error!("cannot listen for Ctrl+C: {error}");
    }
}
