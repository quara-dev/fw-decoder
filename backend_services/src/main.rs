mod config;
mod handlers;
mod services;
mod types;
mod parser;

use axum::{
    routing::{get, post},
    Router,
};
use std::sync::Arc;
use tokio::net::TcpListener;
use tower_http::cors::CorsLayer;

use config::Config;
use handlers::{decode_file, get_versions, refresh_azure_files};

#[tokio::main]
async fn main() {
    let config = Arc::new(Config::from_env());
    
    let app = Router::new()
        .route("/api/versions", get(get_versions))
        .route("/api/decode", post(decode_file))
        .route("/api/refresh", post(refresh_azure_files))
        .layer(CorsLayer::permissive())
        .with_state(config.clone());

    let listener = TcpListener::bind(&config.bind_address)
        .await
        .expect("Failed to bind to address");
    
    println!("Server running on http://{}", config.bind_address);
    axum::serve(listener, app)
        .await
        .expect("Failed to start server");
}
