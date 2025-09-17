use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

use axum::{
    body::Body,
    http::{Request, StatusCode},
    routing::post,
    Router,
};
use loco_rs::{
    app::Hooks,
    controller::monitoring,
    introspection::graph::mutation::{
        GraphMutationService, NodeCreationCommand, ScaffoldGeneration, ScaffoldGenerator,
    },
    tests_cfg,
};
use tower::ServiceExt;

#[derive(Default)]
struct SpyGenerator {
    calls: AtomicUsize,
}

impl SpyGenerator {
    fn call_count(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }
}

impl ScaffoldGenerator for SpyGenerator {
    fn generate(&self, _command: NodeCreationCommand) -> loco_rs::Result<ScaffoldGeneration> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(ScaffoldGeneration::new("ok"))
    }
}

#[tokio::test]
async fn http_node_generation_invokes_generator() {
    let ctx = tests_cfg::app::get_app_context().await;
    let generator = Arc::new(SpyGenerator::default());

    let generator_clone: Arc<dyn ScaffoldGenerator> = generator.clone();
    ctx.shared_store.insert(GraphMutationService::new(
        tests_cfg::db::AppHook::app_name(),
        generator_clone,
    ));

    let router = Router::new()
        .route("/__loco/graph/nodes", post(monitoring::create_graph_node))
        .with_state(ctx);

    let payload = serde_json::json!({
        "component": "task",
        "name": "cleanup",
    });

    let response = router
        .oneshot(
            Request::builder()
                .uri("/__loco/graph/nodes")
                .method("POST")
                .header("content-type", "application/json")
                .body(Body::from(payload.to_string()))
                .expect("build request"),
        )
        .await
        .expect("http response");

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(generator.call_count(), 1);
}
