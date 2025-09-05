use axum::{
    extract::{Multipart, Query, State},
    http::{Response, StatusCode, header},
    response::Json,
};
use std::sync::Arc;

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

fn create_error_response(status: StatusCode, message: &str) -> Response<String> {
    Response::builder()
        .status(status)
        .body(message.to_string())
        .unwrap()
}
