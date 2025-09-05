use axum::{
    extract::{Multipart, Query, State},
    http::{Response, StatusCode, header},
    response::Json,
};
use std::{sync::Arc, process::Command};
use tokio::task;

use crate::{
    config::Config,
    services::{get_available_decoders, FileProcessor, ServiceError},
    types::DecoderQuery,
};

pub async fn get_versions(State(config): State<Arc<Config>>) -> Result<Json<Vec<String>>, StatusCode> {
    match get_available_decoders(&config) {
        Ok(versions) => Ok(Json(versions)),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

pub async fn decode_file(
    State(config): State<Arc<Config>>,
    Query(query): Query<DecoderQuery>,
    multipart: Multipart,
) -> Result<Response<String>, StatusCode> {
    let file_processor = FileProcessor::new((*config).clone());
    
    // Process file upload
    let filepath = match file_processor.process_upload(multipart).await {
        Ok(path) => path,
        Err(ServiceError::InvalidInput(msg)) => {
            return Ok(create_error_response(StatusCode::BAD_REQUEST, &msg));
        }
        Err(_) => {
            return Ok(create_error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to process upload",
            ));
        }
    };

    // Run decoder
    match file_processor.run_decoder(&filepath, &query.version, &query.log_level, query.include_log_level).await {
        Ok(result) => Ok(Response::builder()
            .header(header::CONTENT_TYPE, "application/json; charset=utf-8")
            .body(result)
            .unwrap()),
        Err(ServiceError::NotFound(msg)) => {
            Ok(create_error_response(StatusCode::NOT_FOUND, &msg))
        }
        Err(ServiceError::InvalidInput(msg)) => {
            Ok(create_error_response(StatusCode::BAD_REQUEST, &msg))
        }
        Err(_) => Ok(create_error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Internal server error",
        )),
    }
}

pub async fn refresh_azure_files(State(_config): State<Arc<Config>>) -> Result<Json<serde_json::Value>, StatusCode> {
    // Run the Azure blob downloader script in the background with virtual environment activated
    // Note: Not using --clear-existing to avoid directory locking issues
    let result = task::spawn_blocking(move || {
        let output = Command::new("bash")
            .arg("-c")
            .arg("cd /app && source venv_azure/bin/activate && python3 azure_blob_downloader.py")
            .output();
        
        match output {
            Ok(output) => {
                if output.status.success() {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    Ok(format!("Azure files refresh completed successfully: {}", stdout))
                } else {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    Err(format!("Azure files refresh failed: {}", stderr))
                }
            }
            Err(e) => Err(format!("Failed to execute Azure downloader script: {}", e))
        }
    }).await;
    
    match result {
        Ok(Ok(message)) => {
            Ok(Json(serde_json::json!({
                "status": "success",
                "message": message
            })))
        }
        Ok(Err(error)) => {
            Ok(Json(serde_json::json!({
                "status": "error",
                "message": error
            })))
        }
        Err(_) => {
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

fn create_error_response(status: StatusCode, message: &str) -> Response<String> {
    Response::builder()
        .status(status)
        .body(message.to_string())
        .unwrap()
}
