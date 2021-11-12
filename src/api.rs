use axum::{
    Router,
    routing::{
        get,
        post,
    },
};
use sqlx::SqlitePool;
use tower::ServiceBuilder;
use tower_http::{
    add_extension::AddExtensionLayer,
    trace::TraceLayer,
};

use crate::{
    config::SharedConfiguration,
    routes::terraform::{
        TerraformLockRoute,
        TerraformRoute,
    },
};


pub struct Api {
    config: SharedConfiguration,
    pool: SqlitePool,
}


impl Api {
    pub fn new(config: SharedConfiguration, pool: SqlitePool) -> Self {
        Self {
            config,
            pool
        }
    }
}


impl Into<Router> for Api {
    fn into(self) -> Router {
        let layer = ServiceBuilder::new()
            .layer(AddExtensionLayer::new(self.config))
            .layer(AddExtensionLayer::new(self.pool))
            .into_inner();

        let tf_service = get(TerraformRoute::get)
            .post(TerraformRoute::post);

        let tf_lock_service= post(TerraformLockRoute::post)
            .delete(TerraformLockRoute::delete);

        Router::new()
            .route("/terraform/:id", tf_service)
            .route("/terraform/:id/lock", tf_lock_service)
            .layer(TraceLayer::new_for_http())
            .layer(layer)
    }
}
