use axum::{
    extract::{Multipart, Query},
    http::{Response, StatusCode, header},
    response::Json,
    routing::{get, post},
    Router,
};
use serde::Deserialize;
use std::{
    fs::{self, File},
    io::Write,
    path::PathBuf,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::net::TcpListener;
use tower_http::cors::CorsLayer;

#[derive(Deserialize)]
struct DecoderQuery {
    version: String,
    log_level: String,
}

async fn versions() -> Result<Json<Vec<String>>, StatusCode> {
    let mut result = Vec::new();
    let decoders_path = std::env::var("DECODERS_PATH").unwrap_or_else(|_| "/app/decoders".to_string());
    if let Ok(entries) = fs::read_dir(&decoders_path) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    // Only include files that start with 'decoder_'
                    if name.starts_with("decoder_") {
                        result.push(name.to_string());
                    }
                }
            }
        }
    }
    Ok(Json(result))
}

async fn decode(Query(query): Query<DecoderQuery>, mut multipart: Multipart) -> Result<Response<String>, StatusCode> {
    let mut filepath = None;
    let decoders_path = std::env::var("DECODERS_PATH").unwrap_or_else(|_| "/app/decoders".to_string());
    let decoders_dir = PathBuf::from(&decoders_path);
    let temp_dir = PathBuf::from("/tmp");
    
    // Remove previous temp files (.bin and .log only) from temp directory
    if let Ok(entries) = std::fs::read_dir(&temp_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if name.ends_with(".log") || name.ends_with(".bin") {
                        let _ = std::fs::remove_file(&path);
                    }
                }
            }
        }
    }

    while let Some(field) = multipart.next_field().await.map_err(|_| StatusCode::BAD_REQUEST)? {
        if let Some(filename) = field.file_name() {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis();
            let temp_filename = format!("{}_{}", now, filename);
            let filepath_buf = temp_dir.join(&temp_filename);
            
            let data = field.bytes().await.map_err(|_| StatusCode::BAD_REQUEST)?;
            let mut f = File::create(&filepath_buf).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            f.write_all(&data).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            
            filepath = Some(filepath_buf);
            break;
        }
    }

    let filepath = filepath.ok_or(StatusCode::BAD_REQUEST)?;
    let decoder_path = decoders_dir.join(&query.version);
    let log_path = filepath.with_extension("log");
    
    // Check if decoder exists
    if !decoder_path.exists() {
        return Ok(Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(format!("Decoder '{}' not found", query.version))
            .unwrap());
    }
    
    let output = Command::new(&decoder_path)
        .arg(filepath.to_str().unwrap())
        .arg("-l")
        .arg(&query.log_level)
        .stdout(File::create(&log_path).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?)
        .output();

    match output {
        Ok(result) => {
            if !result.status.success() {
                let stderr = String::from_utf8_lossy(&result.stderr);
                return Ok(Response::builder()
                    .status(StatusCode::BAD_REQUEST)
                    .body(format!("Decoder error: {}", stderr))
                    .unwrap());
            }
            
            match std::fs::read(&log_path) {
                Ok(bytes) => {
                    // Try to convert to UTF-8, fallback to hex
                    match String::from_utf8(bytes.clone()) {
                        Ok(text) => Ok(Response::builder()
                            .header(header::CONTENT_TYPE, "text/plain; charset=utf-8")
                            .body(text)
                            .unwrap()),
                        Err(_) => {
                            let hex = bytes
                                .iter()
                                .map(|b| format!("{:02x}", b))
                                .collect::<Vec<_>>()
                                .join(" ");
                            Ok(Response::builder()
                                .header(header::CONTENT_TYPE, "text/plain; charset=utf-8")
                                .body(format!("[binary output]\n{}", hex))
                                .unwrap())
                        }
                    }
                }
                Err(e) => Ok(Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body(format!("Failed to read log file: {}", e))
                    .unwrap()),
            }
        }
        Err(e) => Ok(Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(format!("Failed to run decoder: {}", e))
            .unwrap()),
    }
}

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/api/versions", get(versions))
        .route("/api/decode", post(decode))
        .layer(CorsLayer::permissive());

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
    
    println!("Server running on http://127.0.0.1:3000");
    axum::serve(listener, app).await.unwrap();
}
