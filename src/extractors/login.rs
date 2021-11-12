use axum::{
    async_trait,
    extract::{
        FromRequest,
        RequestParts,
    },
};
use http::header::AUTHORIZATION;
use http_auth_basic::Credentials;

use crate::{
    config::SharedConfiguration,
    error::HttpError,
};


#[derive(Debug)]
pub struct LoginExtractor(pub Credentials);


#[async_trait]
impl<B> FromRequest<B> for LoginExtractor
where
    B: Send,
{
    type Rejection = HttpError;

    async fn from_request(request: &mut RequestParts<B>) -> Result<Self, Self::Rejection> {
        let config: &SharedConfiguration = request
            .extensions()
            .expect("Failed to retrieve Extensions from Request")
            .get()
            .expect("Failed to get SharedConfiguration from Extensions");

        let credentials = credentials_from_request(request)?;
        let expected_credentials = Credentials::new(
            &config.tf_http_username,
            &config.tf_http_password,
        );

        if credentials == expected_credentials {
            Ok(Self(credentials))
        } else {
            Err(HttpError::unauthorized(None))
        }
    }
}


fn credentials_from_request<B>(request: &RequestParts<B>) -> Result<Credentials, HttpError> {
    let header = request
        .headers()
        .ok_or(HttpError::internal_server_error(None))?
        .get(AUTHORIZATION)
        .ok_or(HttpError::unauthorized(None))?
        .to_str()
        .map_err(|_| HttpError::unauthorized(None))?;

    Credentials::from_header(header.to_owned())
        .map_err(HttpError::from)
}
