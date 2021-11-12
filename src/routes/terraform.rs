use axum::{
    extract::{
        Extension,
        Path,
        Query,
    },
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use axum_debug::debug_handler;
use serde::{
    Deserialize,
};
use serde_json::Value;
use sqlx::SqlitePool;

use crate::{db::terraform::{
        MaybeConflictError,
        TerraformQuery,
    }, error::{HttpError, Loggable}, extractors::LoginExtractor};


#[derive(Deserialize)]
pub struct LockQuery {
    #[serde(alias = "ID")]
    id: String,
}


pub struct TerraformRoute;
pub struct TerraformLockRoute;


impl TerraformRoute {
    #[debug_handler]
    pub async fn get(
        Path(id): Path<String>,
        LoginExtractor(_creds): LoginExtractor,
        Extension(db): Extension<SqlitePool>
    ) -> Result<impl IntoResponse, HttpError> {
        let query = TerraformQuery::new(db)
            .get(&id)
            .await
            .log_error("Database exception when retrieving resource from database")?
            .ok_or(HttpError::not_found(None))?;

        let body: Value = serde_json::from_str(&query.state)
            .map_err(|_| HttpError::internal_server_error(None))?;

        Ok(Json(body))
    }

    #[debug_handler]
    pub async fn post(
        Path(id): Path<String>,
        LoginExtractor(_creds): LoginExtractor,
        Extension(db): Extension<SqlitePool>,
        Json(body): Json<Value>,
        Query(lock_query): Query<LockQuery>,
    ) -> Result<impl IntoResponse, HttpError> {
        let query = TerraformQuery::new(db);
        let serialized = body.to_string();
        let lock = query.get_lock_by_terraform_id(&id)
            .await
            .log_error("Database exception when retrieving lock from database")?
            .ok_or(HttpError::BadRequest("Resource is not locked".to_owned()))?;

        // if the lock id does not match the resource lock id
        if lock.id != lock_query.id {
            let body: Value = serde_json::from_str(&lock.state)
                .log_error("Exception deserializing lock body from database")?;

            return Ok((StatusCode::CONFLICT, Json(body)).into_response());
        }

        query.create_or_replace(&id, &serialized)
            .await?;

        Ok(Json(body).into_response())
    }
}


impl TerraformLockRoute {
    #[debug_handler]
    pub async fn post(
        Path(id): Path<String>,
        LoginExtractor(_creds): LoginExtractor,
        Extension(db): Extension<SqlitePool>,
        Json(body): Json<Value>,
    ) -> Result<impl IntoResponse, HttpError> {
        let query = TerraformQuery::new(db);
        let state = body.to_string();
        let lock_id = body.get("ID")
            .and_then(|v| v.as_str())
            .ok_or(HttpError::BadRequest("Malformed Payload: Missing or malformed ID".to_owned()))?;

        let lock = query.lock(id, lock_id.to_string(), state)
            .await;

        match lock {
            Ok(_) => {
                Ok(Json(body).into_response())
            },
            Err(err) => {
                match err {
                    MaybeConflictError::Conflict(row) => {
                        let body: Value = serde_json::from_str(&row.state)
                            .map_err(|_| HttpError::internal_server_error(None))?;

                        Ok((StatusCode::CONFLICT, Json(body)).into_response())
                    },
                    MaybeConflictError::Error(e) => {
                        Err(e.into())
                    }
                }
            }
        }
    }

    #[debug_handler]
    pub async fn delete(
        Path(id): Path<String>,
        LoginExtractor(_creds): LoginExtractor,
        Extension(db): Extension<SqlitePool>,
        Json(body): Json<Value>,
    ) -> Result<impl IntoResponse, HttpError> {
        let query = TerraformQuery::new(db);
        let state = body.to_string();
        let lock_id = body.get("ID")
            .and_then(|v| v.as_str())
            .ok_or(HttpError::BadRequest("Malformed Payload: Missing or malformed ID".to_owned()))?;

        query.unlock(id.as_str(), lock_id)
            .await
            .log_error("Database exception when deleting lock")?;

        Ok(Json(state))
    }
}



#[cfg(test)]
mod tests {
    use std::{
        collections::HashMap,
        sync::Arc,
    };

    use axum::{
        body::Body,
        http::{
            self,
            Request,
            StatusCode,
        },
    };
    use envconfig::Envconfig;
    use hyper;
    use serde_json::{
        json,
        Value,
    };
    use sqlx::SqlitePool;
    use tokio;
    use tower::ServiceExt;

    use crate::{
        api::Api,
        config::Configuration,
        database,
        db::terraform::TerraformQuery,
    };

    fn default_config() -> Configuration {
        let mut hashmap = HashMap::new();

        hashmap.insert("DATABASE_URI".to_string(), "sqlite::memory:".to_string());
        hashmap.insert("TF_HTTP_USERNAME".to_string(), "asdf".to_string());
        hashmap.insert("TF_HTTP_PASSWORD".to_string(), "asdf".to_string());

        Configuration::init_from_hashmap(&hashmap)
            .unwrap()
    }


    async fn get_migrated_pool(config: &Configuration) -> SqlitePool {
        let pool = database::get_db_pool(&config)
            .await
            .unwrap();

        database::MIGRATE.run(&pool)
            .await
            .unwrap();

        pool
    }


    fn authentication<S: AsRef<str>>(username: S, password: S) -> String {
        format!(
            "Basic {}",
            base64::encode(
                format!(
                    "{}:{}",
                    username.as_ref(),
                    password.as_ref(),
                )
            )
        )
    }


    #[tokio::test]
    async fn test_post_terraform() {
        let config = Arc::new(default_config());
        let pool = get_migrated_pool(&config)
            .await;
        let query = TerraformQuery::new(pool.clone());
        let api = Api::new(config.clone(), pool.clone());
        let id = "105";
        let lock_id = "abcd";
        let lock_body = json!({"abcd": "efgh"});
        let state_body = json!({"state": "something"});
        let uri = format!("/terraform/{}?ID={}", id, lock_id);

        query.lock(
            id,
            lock_id,
            &lock_body.to_string(),
        ).await
        .expect("Failed to create lock row");

        let request = Request::builder()
            .uri(&uri)
            .method(http::Method::POST)
            .header("AUTHORIZATION", authentication(&config.tf_http_username, &config.tf_http_password))
            .header(http::header::CONTENT_TYPE, "application/json")
            .body(Body::from(state_body.to_string()))
            .expect("Failed to build request");

        let router: axum::Router = api.into();
        let response = router
            .oneshot(request)
            .await
            .expect("Failed to call API");

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_lock() {
        let config = Arc::new(default_config());
        let pool = get_migrated_pool(&config)
            .await;
        let query = TerraformQuery::new(pool.clone());
        let api = Api::new(config.clone(), pool.clone());
        let id = "105";
        let lock_id = "abcd";
        let state = json!({"state": "something"});
        let lock_state = json!({"ID": lock_id});
        let uri = format!("/terraform/{}/lock", id);

        query.create_or_replace(id, &state.to_string())
            .await
            .expect("Failed to create terraform resource");

        let request = Request::builder()
            .uri(&uri)
            .method(http::Method::POST)
            .header("AUTHORIZATION", authentication(&config.tf_http_username, &config.tf_http_password))
            .header(http::header::CONTENT_TYPE, "application/json")
            .body(Body::from(lock_state.to_string()))
            .expect("Failed to build request");

        let router: axum::Router = api.into();
        let response = router
            .oneshot(request)
            .await
            .expect("Failed to call API");

        assert_eq!(response.status(), StatusCode::OK);
    }


    #[tokio::test]
    async fn test_lock_locked() {
        let config = Arc::new(default_config());
        let pool = get_migrated_pool(&config)
            .await;
        let query = TerraformQuery::new(pool.clone());
        let api = Api::new(config.clone(), pool.clone());
        let id = "105";
        let lock_id = "abcd";
        let state = json!({"state": "something"});
        let lock_state = json!({"ID": lock_id});
        let alt_lock_state = json!({"ID": "alt_id"});
        let uri = format!("/terraform/{}/lock", id);

        query.create_or_replace(id, &state.to_string())
            .await
            .expect("Failed to create terraform resource");

        query.lock(id, lock_id, &lock_state.to_string())
            .await
            .expect("Failed to lock resource");

        let request = Request::builder()
            .uri(&uri)
            .method(http::Method::POST)
            .header("AUTHORIZATION", authentication(&config.tf_http_username, &config.tf_http_password))
            .header(http::header::CONTENT_TYPE, "application/json")
            .body(Body::from(alt_lock_state.to_string()))
            .expect("Failed to build request");

        let router: axum::Router = api.into();
        let response = router
            .oneshot(request)
            .await
            .expect("Failed to call API");

        assert_eq!(response.status(), StatusCode::CONFLICT);

        let body = hyper::body::to_bytes(response.into_body())
            .await
            .unwrap();

        let body: Value = serde_json::from_slice(&body)
            .unwrap();

        assert_eq!(body, lock_state);
    }

    #[tokio::test]
    async fn test_lock_unlock() {
        let config = Arc::new(default_config());
        let pool = get_migrated_pool(&config)
            .await;
        let query = TerraformQuery::new(pool.clone());
        let api = Api::new(config.clone(), pool.clone());
        let id = "105";
        let lock_id = "defg";
        let state = json!({"state": "something"});
        let lock_state = json!({"ID": lock_id});
        let uri = format!("/terraform/{}/lock", id);

        query.create_or_replace(id, &state.to_string())
            .await
            .expect("Failed to create terraform resource");

        query.lock(id, lock_id, &lock_state.to_string())
            .await
            .expect("Failed to lock resource");

        let request = Request::builder()
            .uri(&uri)
            .method(http::Method::DELETE)
            .header("AUTHORIZATION", authentication(&config.tf_http_username, &config.tf_http_password))
            .header(http::header::CONTENT_TYPE, "application/json")
            .body(Body::from(lock_state.to_string()))
            .expect("Failed to build request");

        let router: axum::Router = api.into();
        let response = router
            .oneshot(request)
            .await
            .expect("Failed to call API");

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]

    async fn test_post_locked() {
        let config = Arc::new(default_config());
        let pool = get_migrated_pool(&config)
            .await;
        let query = TerraformQuery::new(pool.clone());
        let api = Api::new(config.clone(), pool.clone());
        let id = "105";
        let lock_id = "abcd";
        let lock_body = json!({"ID": lock_id, "abcd": "efgh"});
        let state_body = json!({"state": "something"});
        let uri = format!("/terraform/{}?ID={}", id, "wrong_lock_id");

        query.lock(
            id,
            lock_id,
            &lock_body.to_string(),
        ).await
        .expect("Failed to create lock row");

        let request = Request::builder()
            .uri(&uri)
            .method(http::Method::POST)
            .header("AUTHORIZATION", authentication(&config.tf_http_username, &config.tf_http_password))
            .header(http::header::CONTENT_TYPE, "application/json")
            .body(Body::from(state_body.to_string()))
            .expect("Failed to build request");

        let router: axum::Router = api.into();
        let response = router
            .oneshot(request)
            .await
            .expect("Failed to call API");

        assert_eq!(response.status(), StatusCode::CONFLICT);

        let body = hyper::body::to_bytes(response.into_body())
            .await
            .unwrap();

        let body: Value = serde_json::from_slice(&body)
            .unwrap();

        assert_eq!(body.get("ID"), lock_body.get("ID"));
    }

    #[tokio::test]
    async fn test_lock_malformed() {
        let config = Arc::new(default_config());
        let pool = get_migrated_pool(&config)
            .await;
        let query = TerraformQuery::new(pool.clone());
        let api = Api::new(config.clone(), pool.clone());
        let id = "105";
        let lock_id = "abcd";
        let state = json!({"state": "something"});
        let lock_state = json!({"IDa": lock_id});
        let uri = format!("/terraform/{}/lock", id);

        query.create_or_replace(id, &state.to_string())
            .await
            .expect("Failed to create terraform resource");

        let request = Request::builder()
            .uri(&uri)
            .method(http::Method::POST)
            .header("AUTHORIZATION", authentication(&config.tf_http_username, &config.tf_http_password))
            .header(http::header::CONTENT_TYPE, "application/json")
            .body(Body::from(lock_state.to_string()))
            .expect("Failed to build request");

        let router: axum::Router = api.into();
        let response = router
            .oneshot(request)
            .await
            .expect("Failed to call API");

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_unlock_malformed() {
        let config = Arc::new(default_config());
        let pool = get_migrated_pool(&config)
            .await;
        let query = TerraformQuery::new(pool.clone());
        let api = Api::new(config.clone(), pool.clone());
        let id = "105";
        let lock_id = "abcd";
        let state = json!({"state": "something"});
        let lock_state = json!({"IDa": lock_id});
        let uri = format!("/terraform/{}/lock", id);

        query.create_or_replace(id, &state.to_string())
            .await
            .expect("Failed to create terraform resource");

        let request = Request::builder()
            .uri(&uri)
            .method(http::Method::DELETE)
            .header("AUTHORIZATION", authentication(&config.tf_http_username, &config.tf_http_password))
            .header(http::header::CONTENT_TYPE, "application/json")
            .body(Body::from(lock_state.to_string()))
            .expect("Failed to build request");

        let router: axum::Router = api.into();
        let response = router
            .oneshot(request)
            .await
            .expect("Failed to call API");

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
}

