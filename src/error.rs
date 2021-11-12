use anyhow::{

    Context,
    Error as AnyhowError,
};
use axum::{
    body::Full,
    Json,
    response::IntoResponse,
};
use http::StatusCode;
use http_auth_basic::AuthBasicError;
use serde::Serialize;
use serde_json::json;
use sqlx::Error as SqlxError;


#[derive(Debug, Clone, Serialize)]
pub enum HttpError {
    BadGateway(String),
    BadRequest(String),
    InternalServerError(String),
    NotFound(String),
    Unauthorized(String),
}


pub trait Loggable<T, E>: Context<T, E> {
    fn log_error<C>(self, context: C) -> Result<T, AnyhowError>
    where
        C: std::fmt::Display + Send + Sync + 'static;

}


impl<T, E> Loggable<T, E> for Result<T, E>
where
    E: std::error::Error + Send + Sync + 'static {

    fn log_error<C: std::fmt::Display + Send + Sync + 'static>(self, context: C) -> Result<T, anyhow::Error> {
        self
            .context(context)
            .map_err(|e| {
                tracing::error!("{:#?}", e);

                e
            })
    }
}


impl HttpError {
    pub fn internal_server_error(message: Option<String>) -> Self {
        let message: String = message
            .unwrap_or("Internal Server Error".to_string());

        Self::InternalServerError(message)
    }

    pub fn bad_request(message: Option<String>) -> Self {
        let message: String = message
            .unwrap_or("Bad Request".to_string());

        Self::BadRequest(message)
    }

    pub fn unauthorized(message: Option<String>) -> Self {
        let message: String = message
            .unwrap_or("Unauthorized".to_string());

        Self::Unauthorized(message)
    }

    pub fn bad_gateway(message: Option<String>) -> Self {
        let message: String = message
            .unwrap_or("Bad Gateway".to_string());

        Self::BadGateway(message)
    }

    pub fn not_found(message: Option<String>) -> Self {
        let message: String = message
            .unwrap_or("Not Found".to_string());

        Self::NotFound(message)
    }
}


impl IntoResponse for HttpError {
    type Body = Full<axum::body::Bytes>;
    type BodyError = <Self::Body as axum::body::HttpBody>::Error;

    fn into_response(self) -> http::Response<Self::Body> {
        let (status_code, message) = match self {
            HttpError::InternalServerError(s) => (StatusCode::INTERNAL_SERVER_ERROR, s),
            HttpError::BadRequest(s) => (StatusCode::BAD_REQUEST, s),
            HttpError::Unauthorized(s) => (StatusCode::UNAUTHORIZED, s),
            HttpError::BadGateway(s) => (StatusCode::BAD_GATEWAY, s),
            HttpError::NotFound(s) => (StatusCode::NOT_FOUND, s),
        };

        (status_code, Json(json!({"message": message}))).into_response()
    }
}


impl From<AuthBasicError> for HttpError {
    fn from(err: AuthBasicError) -> Self {
        Self::bad_request(Some(err.to_string()))
    }
}


impl From<SqlxError> for HttpError {
    fn from(_: SqlxError) -> Self {
        Self::bad_gateway(None)
    }
}


impl From<anyhow::Error> for HttpError {
    fn from(_: anyhow::Error) -> Self {
        Self::internal_server_error(None)
    }
}
