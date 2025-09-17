use std::net::SocketAddr;

use axum::{http::StatusCode, Router};
use insta::assert_snapshot;
use loco_rs::{
    app::Hooks,
    introspection::graph::service::{
        ApplicationGraphService, GraphIntrospectionSeed, GraphQueryService,
    },
    TestServer,
};
use serde_json::Value;

use loco_rs::tests_cfg;

#[tokio::test]
async fn graph_endpoint_matches_cli_snapshot() {
    let ctx = tests_cfg::app::get_app_context().await;
    let app_routes = tests_cfg::db::AppHook::routes(&ctx);
    let collected_routes = app_routes.collect();
    let route_descriptors = ApplicationGraphService::collect_route_descriptors(&collected_routes);
    ctx.shared_store.insert(GraphIntrospectionSeed::new(
        tests_cfg::db::AppHook::app_name(),
        route_descriptors,
    ));

    let router = app_routes
        .to_router::<tests_cfg::db::AppHook>(ctx.clone(), Router::new())
        .expect("build monitoring router");

    let server = TestServer::new(router.into_make_service_with_connect_info::<SocketAddr>())
        .expect("start test server");

    let response = server.get("/__loco/graph").await;

    assert_eq!(response.status_code(), StatusCode::OK);

    let http_json: Value = response.json::<Value>();
    assert!(http_json.get("routes").is_some());
    assert!(http_json.get("dependencies").is_some());
    assert_eq!(http_json["health"]["ok"], Value::Bool(true));

    let cli_value = {
        let seed = ctx
            .shared_store
            .get_ref::<GraphIntrospectionSeed>()
            .expect("graph metadata should be seeded");
        serde_json::to_value(seed.into_service(&ctx).snapshot()).unwrap()
    };

    assert_eq!(http_json, cli_value);
    assert_snapshot!(
        "graph_cli_snapshot",
        serde_json::to_string_pretty(&cli_value).expect("serialize graph snapshot")
    );
}
