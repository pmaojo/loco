//! This module contains a base routes related to readiness checks and status
//! reporting. These routes are commonly used to monitor the readiness of the
//! application and its dependencies.

#[cfg(feature = "introspection_console")]
use super::cli_console;
use super::{format, routes::Routes};

#[cfg(debug_assertions)]
use crate::introspection::graph::mutation::{
    GraphMutationService, NodeCreationRequest, ScaffoldGenerator,
};

#[cfg(feature = "introspection_assistant")]
use crate::introspection::assistant::{
    self, IntrospectionAssistant, RuleBasedAssistantClient, SharedStoreConversationStore,
};
use crate::{
    app::AppContext,
    config,
    errors::Error,
    introspection::graph::service::{GraphIntrospectionSeed, GraphQueryService},
    Result,
};
use axum::{extract::State, response::Response, routing::get};
#[cfg(any(debug_assertions, feature = "introspection_assistant"))]
use axum::{routing::post, Json};
#[cfg(feature = "introspection_assistant")]
use serde::Deserialize;
use serde::Serialize;
#[cfg(debug_assertions)]
use std::sync::Arc;

/// Represents the health status of the application.
#[derive(Serialize)]
pub struct Health {
    pub ok: bool,
}

/// Check application ping endpoint
///
/// # Errors
/// This function always returns `Ok` with a JSON response indicating the
pub async fn ping() -> Result<Response> {
    format::json(Health { ok: true })
}

/// Check application ping endpoint
///
/// # Errors
/// This function always returns `Ok` with a JSON response indicating the
pub async fn health() -> Result<Response> {
    format::json(Health { ok: true })
}

/// Check the readiness of the application by sending a ping request to
/// Redis or the DB (depending on feature flags) to ensure connection liveness.
///
/// # Errors
/// All errors are logged, and the readiness status is returned as a JSON response.
pub async fn readiness(State(ctx): State<AppContext>) -> Result<Response> {
    let mut is_ok: bool = true;

    #[cfg(feature = "with-db")]
    if let Err(error) = &ctx.db.ping().await {
        tracing::error!(err.msg = %error, err.detail = ?error, "readiness_db_ping_error");
        is_ok = false;
    }

    if let Some(queue) = &ctx.queue_provider {
        if let Err(error) = queue.ping().await {
            tracing::error!(err.msg = %error, err.detail = ?error, "readiness_queue_ping_error");
            is_ok = false;
        }
    }

    #[cfg(any(feature = "cache_inmem", feature = "cache_redis"))]
    {
        match ctx.config.cache {
            #[cfg(feature = "cache_inmem")]
            config::CacheConfig::InMem(_) => {
                if let Err(error) = &ctx.cache.driver.ping().await {
                    tracing::error!(err.msg = %error, err.detail = ?error, "readiness_cache_ping_error");
                    is_ok = false;
                }
            }
            #[cfg(feature = "cache_redis")]
            config::CacheConfig::Redis(_) => {
                if let Err(error) = &ctx.cache.driver.ping().await {
                    tracing::error!(err.msg = %error, err.detail = ?error, "readiness_cache_ping_error");
                    is_ok = false;
                }
            }
            config::CacheConfig::Null => (),
        }
    }

    format::json(Health { ok: is_ok })
}

/// Returns the application graph snapshot used for introspection adapters.
pub async fn graph(State(ctx): State<AppContext>) -> Result<Response> {
    let snapshot = {
        let seed = ctx
            .shared_store
            .get_ref::<GraphIntrospectionSeed>()
            .ok_or_else(|| Error::Message("application graph metadata unavailable".to_string()))?;
        seed.into_service(&ctx).snapshot()
    };

    format::json(snapshot)
}

#[cfg(debug_assertions)]
pub async fn create_graph_node(
    State(ctx): State<AppContext>,
    Json(request): Json<NodeCreationRequest>,
) -> Result<Response> {
    let service = ctx
        .shared_store
        .get_ref::<GraphMutationService<Arc<dyn ScaffoldGenerator>>>()
        .ok_or_else(|| Error::Message("scaffold generator unavailable".to_string()))?;
    let generation = service.create_node(request)?;
    format::json(generation)
}

#[cfg(feature = "introspection_assistant")]
#[derive(Deserialize)]
pub struct AssistantRequestBody {
    #[serde(default)]
    pub doctor_findings: Vec<assistant::DoctorFinding>,
}

#[cfg(feature = "introspection_assistant")]
pub async fn assistant(
    State(ctx): State<AppContext>,
    Json(payload): Json<AssistantRequestBody>,
) -> Result<Response> {
    let conversation_store = SharedStoreConversationStore::new(ctx.shared_store.clone());
    let client = RuleBasedAssistantClient::default();
    let advice = {
        let seed = ctx
            .shared_store
            .get_ref::<GraphIntrospectionSeed>()
            .ok_or_else(|| Error::Message("application graph metadata unavailable".to_string()))?;
        let app_name = seed.app_name.clone();
        let graph_service = seed.into_service(&ctx);
        let adapter = IntrospectionAssistant::new(
            app_name.as_str(),
            &graph_service,
            &client,
            &conversation_store,
        );

        adapter
            .advise(&payload.doctor_findings)
            .await
            .map_err(|error| Error::Message(error.to_string()))?
    };

    format::json(advice)
}

/// Defines and returns the readiness-related routes.
pub fn routes() -> Routes {
    let mut routes = Routes::new()
        .add("/_readiness", get(readiness))
        .add("/_ping", get(ping))
        .add("/_health", get(health))
        .add("/__loco/graph", get(graph));

    #[cfg(feature = "introspection_console")]
    {
        routes = routes.merge(cli_console::routes());
    }

    #[cfg(debug_assertions)]
    {
        routes = routes.add("/__loco/graph/nodes", post(create_graph_node));
    }

    #[cfg(feature = "introspection_assistant")]
    {
        routes = routes.add("/__loco/assistant", post(assistant));
    }
    routes
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "cache_redis")]
    use crate::tests_cfg::redis::setup_redis_container;
    use axum::routing::get;
    use loco_rs::tests_cfg::db::fail_connection;
    use loco_rs::{bgworker, cache, config, controller::monitoring, tests_cfg};
    use serde_json::Value;
    use tower::ServiceExt;

    #[tokio::test]
    async fn ping_works() {
        let ctx = tests_cfg::app::get_app_context().await;

        // Create a router with the ping route
        let router = axum::Router::new()
            .route("/_ping", get(monitoring::ping))
            .with_state(ctx);

        // Create a request
        let req = axum::http::Request::builder()
            .uri("/_ping")
            .method("GET")
            .body(axum::body::Body::empty())
            .unwrap();

        // Test the router directly using oneshot
        let response = router.oneshot(req).await.unwrap();
        assert_eq!(response.status(), 200);

        // Get the response body
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let res_json: Value = serde_json::from_slice(&body).expect("Valid JSON response");
        assert_eq!(res_json["ok"], true);
    }

    #[tokio::test]
    async fn health_works() {
        let ctx = tests_cfg::app::get_app_context().await;

        // Create a router with the health route
        let router = axum::Router::new()
            .route("/_health", get(monitoring::health))
            .with_state(ctx);

        // Create a request
        let req = axum::http::Request::builder()
            .uri("/_health")
            .method("GET")
            .body(axum::body::Body::empty())
            .unwrap();

        // Test the router directly using oneshot
        let response = router.oneshot(req).await.unwrap();
        assert_eq!(response.status(), 200);

        // Get the response body
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let res_json: Value = serde_json::from_slice(&body).expect("Valid JSON response");
        assert_eq!(res_json["ok"], true);
    }

    #[cfg(not(feature = "with-db"))]
    #[tokio::test]
    async fn readiness_no_features() {
        let ctx = tests_cfg::app::get_app_context().await;

        // Create a router with the readiness route
        let router = axum::Router::new()
            .route("/_readiness", get(monitoring::readiness))
            .with_state(ctx);

        // Create a request
        let req = axum::http::Request::builder()
            .uri("/_readiness")
            .method("GET")
            .body(axum::body::Body::empty())
            .unwrap();

        // Test the router directly using oneshot
        let response = router.oneshot(req).await.unwrap();
        assert_eq!(response.status(), 200);

        // Get the response body
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let res_json: Value = serde_json::from_slice(&body).expect("Valid JSON response");
        assert_eq!(res_json["ok"], true);
    }

    #[cfg(feature = "with-db")]
    #[tokio::test]
    async fn readiness_with_db_success() {
        let ctx = tests_cfg::app::get_app_context().await;

        // Create a router with the readiness route
        let router = axum::Router::new()
            .route("/_readiness", get(monitoring::readiness))
            .with_state(ctx);

        // Create a request
        let req = axum::http::Request::builder()
            .uri("/_readiness")
            .method("GET")
            .body(axum::body::Body::empty())
            .unwrap();

        // Test the router directly using oneshot
        let response = router.oneshot(req).await.unwrap();
        assert_eq!(response.status(), 200);

        // Get the response body
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let res_json: Value = serde_json::from_slice(&body).expect("Valid JSON response");
        assert_eq!(res_json["ok"], true);
    }

    #[cfg(feature = "with-db")]
    #[tokio::test]
    async fn readiness_with_db_failure() {
        let mut ctx = tests_cfg::app::get_app_context().await;
        ctx.db = fail_connection().await;

        // Create a router with the readiness route
        let router = axum::Router::new()
            .route("/_readiness", get(monitoring::readiness))
            .with_state(ctx);

        // Create a request
        let req = axum::http::Request::builder()
            .uri("/_readiness")
            .method("GET")
            .body(axum::body::Body::empty())
            .unwrap();

        // Test the router directly using oneshot
        let response = router.oneshot(req).await.unwrap();
        assert_eq!(response.status(), 200);

        // Get the response body
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let res_json: Value = serde_json::from_slice(&body).expect("Valid JSON response");
        assert_eq!(res_json["ok"], false);
    }

    #[cfg(feature = "cache_inmem")]
    #[tokio::test]
    async fn readiness_with_cache_inmem() {
        let mut ctx = tests_cfg::app::get_app_context().await;

        ctx.cache = cache::drivers::inmem::new(&loco_rs::config::InMemCacheConfig {
            max_capacity: 32 * 1024 * 1024,
        })
        .into();

        // Create a router with the readiness route
        let router = axum::Router::new()
            .route("/_readiness", get(monitoring::readiness))
            .with_state(ctx);

        // Create a request
        let req = axum::http::Request::builder()
            .uri("/_readiness")
            .method("GET")
            .body(axum::body::Body::empty())
            .unwrap();

        // Test the router directly using oneshot
        let response = router.oneshot(req).await.unwrap();
        assert_eq!(response.status(), 200);

        // Get the response body
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let res_json: Value = serde_json::from_slice(&body).expect("Valid JSON response");
        assert_eq!(res_json["ok"], true);
    }

    #[cfg(feature = "cache_redis")]
    #[tokio::test]
    async fn readiness_with_cache_redis_success() {
        let (redis_url, _container) = setup_redis_container().await;
        let mut ctx = tests_cfg::app::get_app_context().await;

        // Create Redis cache driver and assign to ctx.cache
        let redis_cache = cache::drivers::redis::new(&config::RedisCacheConfig {
            uri: redis_url,
            max_size: 10,
        })
        .await
        .expect("Failed to create Redis cache");
        ctx.cache = redis_cache.into();

        // Create a router with the readiness route
        let router = axum::Router::new()
            .route("/_readiness", get(monitoring::readiness))
            .with_state(ctx);

        // Create a request
        let req = axum::http::Request::builder()
            .uri("/_readiness")
            .method("GET")
            .body(axum::body::Body::empty())
            .unwrap();

        // Test the router directly using oneshot
        let response = router.oneshot(req).await.unwrap();
        assert_eq!(response.status(), 200);

        // Get the response body
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let res_json: Value = serde_json::from_slice(&body).expect("Valid JSON response");
        assert_eq!(res_json["ok"], true);
    }

    #[cfg(feature = "cache_redis")]
    #[tokio::test]
    async fn readiness_with_cache_redis_failure() {
        let mut ctx = tests_cfg::app::get_app_context().await;
        let failour_redis_url = "redis://127.0.0.2:0";
        // Force config to Redis to ensure ping path executes, but swap driver to Null (which errors on ping)
        ctx.config.cache = config::CacheConfig::Redis(loco_rs::config::RedisCacheConfig {
            uri: failour_redis_url.to_string(),
            max_size: 10,
        });
        // Create Redis cache driver and assign to ctx.cache
        ctx.cache = cache::drivers::redis::new(&config::RedisCacheConfig {
            uri: failour_redis_url.to_string(),
            max_size: 10,
        })
        .await
        .expect("Failed to create Redis cache")
        .into();

        // Create a router with the readiness route
        let router = axum::Router::new()
            .route("/_readiness", get(monitoring::readiness))
            .with_state(ctx);

        // Create a request
        let req = axum::http::Request::builder()
            .uri("/_readiness")
            .method("GET")
            .body(axum::body::Body::empty())
            .unwrap();

        // Test the router directly using oneshot
        let response = router.oneshot(req).await.unwrap();
        assert_eq!(response.status(), 200);

        // Get the response body
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let res_json: Value = serde_json::from_slice(&body).expect("Valid JSON response");
        assert_eq!(res_json["ok"], false);
    }

    #[tokio::test]
    async fn readiness_with_queue_not_present() {
        let mut ctx = tests_cfg::app::get_app_context().await;
        // simulate background queue mode with a no-op provider
        ctx.config.workers.mode = config::WorkerMode::BackgroundQueue;
        ctx.queue_provider = Some(std::sync::Arc::new(bgworker::Queue::None));

        // Create a router with the readiness route
        let router = axum::Router::new()
            .route("/_readiness", get(monitoring::readiness))
            .with_state(ctx);

        // Create a request
        let req = axum::http::Request::builder()
            .uri("/_readiness")
            .method("GET")
            .body(axum::body::Body::empty())
            .unwrap();

        // Test the router directly using oneshot
        let response = router.oneshot(req).await.unwrap();
        assert_eq!(response.status(), 200);

        // Get the response body
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let res_json: Value = serde_json::from_slice(&body).expect("Valid JSON response");
        assert_eq!(res_json["ok"], true);
    }

    #[cfg(feature = "bg_redis")]
    #[tokio::test]
    async fn readiness_with_queue_present_failure() {
        let mut ctx = tests_cfg::app::get_app_context().await;

        // Configure Redis queue with invalid URL to trigger failure
        let failure_redis_url = "redis://127.0.0.2:0";
        ctx.config.workers.mode = config::WorkerMode::BackgroundQueue;
        ctx.config.queue = Some(config::QueueConfig::Redis(config::RedisQueueConfig {
            uri: failure_redis_url.to_string(),
            dangerously_flush: false,
            queues: None,
            num_workers: 1,
        }));

        // Create Redis queue provider directly with failing Redis connection
        ctx.queue_provider = Some(std::sync::Arc::new(
            bgworker::redis::create_provider(&config::RedisQueueConfig {
                uri: failure_redis_url.to_string(),
                dangerously_flush: false,
                queues: None,
                num_workers: 1,
            })
            .await
            .expect("Failed to create Redis queue provider"),
        ));

        // Create a router with the readiness route
        let router = axum::Router::new()
            .route("/_readiness", get(monitoring::readiness))
            .with_state(ctx);

        // Create a request
        let req = axum::http::Request::builder()
            .uri("/_readiness")
            .method("GET")
            .body(axum::body::Body::empty())
            .unwrap();

        // Test the router directly using oneshot
        let response = router.oneshot(req).await.unwrap();
        assert_eq!(response.status(), 200);

        // Get the response body
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let res_json: Value = serde_json::from_slice(&body).expect("Valid JSON response");
        assert_eq!(res_json["ok"], false);
    }
}
