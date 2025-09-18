use std::{
    net::SocketAddr,
    sync::{Arc, Mutex},
};

use axum::{
    http::StatusCode,
    routing::{get, post},
    Router,
};
use loco_rs::{
    app::AppContext,
    controller::{cli_console, cli_console::ListableCommand},
    introspection::cli::{
        CliAutomationService, CommandOutput, ListGeneratorsRequest, ListTasksRequest,
        RunDoctorRequest, RunGeneratorRequest, RunTaskRequest,
    },
    tests_cfg, TestServer,
};
use serde_json::json;

fn router_with_state(ctx: AppContext) -> Router {
    Router::new()
        .route("/__loco/cli/generators", get(cli_console::list_generators))
        .route(
            "/__loco/cli/generators/run",
            post(cli_console::run_generator),
        )
        .route("/__loco/cli/tasks", get(cli_console::list_tasks))
        .route("/__loco/cli/tasks/run", post(cli_console::run_task))
        .route(
            "/__loco/cli/doctor/snapshot",
            post(cli_console::doctor_snapshot),
        )
        .with_state(ctx)
}

#[derive(Default)]
struct StubCliAutomationService {
    list_generators_response: CommandOutput,
    list_tasks_response: CommandOutput,
    run_generator_response: CommandOutput,
    run_task_response: CommandOutput,
    doctor_response: CommandOutput,
    list_generators_calls: Mutex<Vec<ListGeneratorsRequest>>,
    list_tasks_calls: Mutex<Vec<ListTasksRequest>>,
    run_generator_calls: Mutex<Vec<RunGeneratorRequest>>,
    run_task_calls: Mutex<Vec<RunTaskRequest>>,
    doctor_calls: Mutex<Vec<RunDoctorRequest>>,
}

impl StubCliAutomationService {
    fn with_list_generators_stdout(stdout: &str) -> Self {
        Self {
            list_generators_response: CommandOutput::new(0, stdout, ""),
            ..Self::default()
        }
    }

    fn with_list_tasks_stdout(stdout: &str) -> Self {
        Self {
            list_tasks_response: CommandOutput::new(0, stdout, ""),
            ..Self::default()
        }
    }

    fn list_generators_calls(&self) -> Vec<ListGeneratorsRequest> {
        self.list_generators_calls
            .lock()
            .expect("list_generators lock")
            .clone()
    }

    fn list_tasks_calls(&self) -> Vec<ListTasksRequest> {
        self.list_tasks_calls
            .lock()
            .expect("list_tasks lock")
            .clone()
    }

    fn run_generator_calls(&self) -> Vec<RunGeneratorRequest> {
        self.run_generator_calls
            .lock()
            .expect("run_generator lock")
            .clone()
    }

    fn run_task_calls(&self) -> Vec<RunTaskRequest> {
        self.run_task_calls.lock().expect("run_task lock").clone()
    }

    fn doctor_calls(&self) -> Vec<RunDoctorRequest> {
        self.doctor_calls.lock().expect("doctor lock").clone()
    }
}

impl CliAutomationService for StubCliAutomationService {
    fn list_generators(&self, request: &ListGeneratorsRequest) -> loco_rs::Result<CommandOutput> {
        self.list_generators_calls
            .lock()
            .expect("list_generators lock")
            .push(request.clone());
        Ok(self.list_generators_response.clone())
    }

    fn run_generator(&self, request: &RunGeneratorRequest) -> loco_rs::Result<CommandOutput> {
        self.run_generator_calls
            .lock()
            .expect("run_generator lock")
            .push(request.clone());
        if self.run_generator_response.status == 0
            && self.run_generator_response.stdout.is_empty()
            && self.run_generator_response.stderr.is_empty()
        {
            Ok(CommandOutput::default())
        } else {
            Ok(self.run_generator_response.clone())
        }
    }

    fn list_tasks(&self, request: &ListTasksRequest) -> loco_rs::Result<CommandOutput> {
        self.list_tasks_calls
            .lock()
            .expect("list_tasks lock")
            .push(request.clone());
        Ok(self.list_tasks_response.clone())
    }

    fn list_jobs(
        &self,
        _request: &loco_rs::introspection::cli::ListJobsRequest,
    ) -> loco_rs::Result<CommandOutput> {
        unimplemented!()
    }

    fn enqueue_job(
        &self,
        _request: &loco_rs::introspection::cli::EnqueueJobRequest,
    ) -> loco_rs::Result<CommandOutput> {
        unimplemented!()
    }

    fn run_doctor(&self, request: &RunDoctorRequest) -> loco_rs::Result<CommandOutput> {
        self.doctor_calls
            .lock()
            .expect("doctor lock")
            .push(request.clone());
        Ok(self.doctor_response.clone())
    }

    fn run_task(&self, request: &RunTaskRequest) -> loco_rs::Result<CommandOutput> {
        self.run_task_calls
            .lock()
            .expect("run_task lock")
            .push(request.clone());
        Ok(self.run_task_response.clone())
    }
}

fn insert_service(ctx: &AppContext, service: Arc<StubCliAutomationService>) {
    let automation: Arc<dyn CliAutomationService> = service.clone();
    ctx.shared_store.insert(automation);
}

#[tokio::test]
async fn list_generators_parses_cli_output() {
    let ctx = tests_cfg::app::get_app_context().await;
    let service = Arc::new(StubCliAutomationService::with_list_generators_stdout(
        "Commands:\n  model    Generates a new model\n  migration    Generates a new migration\n",
    ));
    insert_service(&ctx, service.clone());

    let router = router_with_state(ctx.clone());
    let server =
        TestServer::new(router.into_make_service_with_connect_info::<SocketAddr>()).unwrap();

    let response = server
        .get("/__loco/cli/generators?environment=development")
        .await;

    assert_eq!(response.status_code(), StatusCode::OK);

    let commands: Vec<ListableCommand> = response.json();
    assert_eq!(
        commands,
        vec![
            ListableCommand {
                command: "model".into(),
                summary: "Generates a new model".into(),
            },
            ListableCommand {
                command: "migration".into(),
                summary: "Generates a new migration".into(),
            },
        ]
    );

    let calls = service.list_generators_calls();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].environment, Some("development".to_string()));
}

#[tokio::test]
async fn run_generator_forwards_payload() {
    let ctx = tests_cfg::app::get_app_context().await;
    let service = Arc::new(StubCliAutomationService {
        run_generator_response: CommandOutput::new(0, "generated", ""),
        ..StubCliAutomationService::default()
    });
    insert_service(&ctx, service.clone());

    let router = router_with_state(ctx.clone());
    let server =
        TestServer::new(router.into_make_service_with_connect_info::<SocketAddr>()).unwrap();

    let response = server
        .post("/__loco/cli/generators/run")
        .json(&json!({
            "generator": "model",
            "arguments": ["posts", "title:string"],
            "environment": "qa"
        }))
        .await;

    assert_eq!(response.status_code(), StatusCode::OK);

    let body = response.json::<serde_json::Value>();
    assert_eq!(body["status"], 0);
    assert_eq!(body["stdout"], "generated");

    let calls = service.run_generator_calls();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].generator, "model");
    assert_eq!(calls[0].arguments, vec!["posts", "title:string"]);
    assert_eq!(calls[0].environment, Some("qa".into()));
}

#[tokio::test]
async fn list_tasks_parses_cli_output() {
    let ctx = tests_cfg::app::get_app_context().await;
    let service = Arc::new(StubCliAutomationService::with_list_tasks_stdout(
        "foo                           [Run foo]\nbar                           [Run bar]\n",
    ));
    insert_service(&ctx, service.clone());

    let router = router_with_state(ctx.clone());
    let server =
        TestServer::new(router.into_make_service_with_connect_info::<SocketAddr>()).unwrap();

    let response = server.get("/__loco/cli/tasks?environment=staging").await;

    assert_eq!(response.status_code(), StatusCode::OK);

    let commands: Vec<ListableCommand> = response.json();
    assert_eq!(
        commands,
        vec![
            ListableCommand {
                command: "foo".into(),
                summary: "[Run foo]".into(),
            },
            ListableCommand {
                command: "bar".into(),
                summary: "[Run bar]".into(),
            },
        ]
    );

    let calls = service.list_tasks_calls();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].environment, Some("staging".into()));
}

#[tokio::test]
async fn run_task_merges_arguments_and_params() {
    let ctx = tests_cfg::app::get_app_context().await;
    let service = Arc::new(StubCliAutomationService {
        run_task_response: CommandOutput::new(0, "ran", ""),
        ..StubCliAutomationService::default()
    });
    insert_service(&ctx, service.clone());

    let router = router_with_state(ctx.clone());
    let server =
        TestServer::new(router.into_make_service_with_connect_info::<SocketAddr>()).unwrap();

    let response = server
        .post("/__loco/cli/tasks/run")
        .json(&json!({
            "task": "parse_args",
            "arguments": ["foo:bar"],
            "params": {"alpha": "one", "beta": "two"},
            "environment": "test"
        }))
        .await;

    assert_eq!(response.status_code(), StatusCode::OK);

    let payload = response.json::<serde_json::Value>();
    assert_eq!(payload["stdout"], "ran");

    let calls = service.run_task_calls();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].task, "parse_args");
    assert_eq!(calls[0].environment, Some("test".into()));
    assert_eq!(calls[0].arguments, vec!["foo:bar", "alpha:one", "beta:two"]);
}

#[tokio::test]
async fn doctor_snapshot_parses_json_output() {
    let ctx = tests_cfg::app::get_app_context().await;
    let service = Arc::new(StubCliAutomationService {
        doctor_response: CommandOutput::new(0, "{\"ok\":true}", ""),
        ..StubCliAutomationService::default()
    });
    insert_service(&ctx, service.clone());

    let router = router_with_state(ctx.clone());
    let server =
        TestServer::new(router.into_make_service_with_connect_info::<SocketAddr>()).unwrap();

    let response = server
        .post("/__loco/cli/doctor/snapshot")
        .json(&json!({"environment": "dev", "graph": true}))
        .await;

    assert_eq!(response.status_code(), StatusCode::OK);

    let snapshot = response.json::<serde_json::Value>();
    assert_eq!(snapshot["status"], 0);
    assert_eq!(snapshot["stdout"], json!({"ok": true}));

    let calls = service.doctor_calls();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].graph, true);
    assert_eq!(calls[0].environment, Some("dev".into()));
}

#[tokio::test]
async fn returns_not_found_when_service_missing() {
    let ctx = tests_cfg::app::get_app_context().await;

    let router = router_with_state(ctx.clone());
    let server =
        TestServer::new(router.into_make_service_with_connect_info::<SocketAddr>()).unwrap();

    let response = server.get("/__loco/cli/generators").await;

    assert_eq!(response.status_code(), StatusCode::NOT_FOUND);
}
