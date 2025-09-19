use std::sync::{Arc, Mutex};

use loco_rs::introspection::cli::adapters::cargo::CargoCliAutomationService;
use loco_rs::introspection::cli::{
    CliAutomationService, CliCommand, CommandExecutor, CommandOutput, EnqueueJobRequest,
    JobStatusRequest, JobStatusResponse, ListGeneratorsRequest, ListJobsRequest, ListTasksRequest,
    RunDoctorRequest, RunGeneratorRequest, RunTaskRequest,
};
use loco_rs::Result;

#[derive(Clone)]
struct FakeCommandExecutor {
    commands: Arc<Mutex<Vec<CliCommand>>>,
    output: CommandOutput,
}

impl FakeCommandExecutor {
    fn new(output: CommandOutput) -> Self {
        Self {
            commands: Arc::new(Mutex::new(Vec::new())),
            output,
        }
    }

    fn recorded(&self) -> Vec<CliCommand> {
        self.commands.lock().expect("lock poisoned").clone()
    }
}

impl Default for FakeCommandExecutor {
    fn default() -> Self {
        Self::new(CommandOutput::default())
    }
}

impl CommandExecutor for FakeCommandExecutor {
    fn execute(&self, command: &CliCommand) -> Result<CommandOutput> {
        self.commands
            .lock()
            .expect("lock poisoned")
            .push(command.clone());
        Ok(self.output.clone())
    }
}

fn service_with_executor(
    executor: Arc<FakeCommandExecutor>,
) -> CargoCliAutomationService<FakeCommandExecutor> {
    CargoCliAutomationService::new(executor)
}

#[test]
fn list_generators_uses_generate_help() {
    let executor = Arc::new(FakeCommandExecutor::default());
    let service = service_with_executor(Arc::clone(&executor));
    let request = ListGeneratorsRequest {
        environment: Some("development".into()),
    };

    let output = service
        .list_generators(&request)
        .expect("command to succeed");

    assert_eq!(output.status, 0);
    let commands = executor.recorded();
    assert_eq!(commands.len(), 1);
    assert_eq!(commands[0].program, "cargo");
    assert_eq!(
        commands[0].args,
        vec![
            "loco".to_string(),
            "generate".to_string(),
            "--help".to_string(),
            "--environment".to_string(),
            "development".to_string(),
        ]
    );
}

#[test]
fn run_generator_composes_arguments() {
    let executor = Arc::new(FakeCommandExecutor::default());
    let service = service_with_executor(Arc::clone(&executor));
    let request = RunGeneratorRequest {
        generator: "model".into(),
        arguments: vec!["posts".into(), "title:string".into()],
        ..RunGeneratorRequest::default()
    };

    service.run_generator(&request).expect("command to succeed");

    let commands = executor.recorded();
    assert_eq!(commands.len(), 1);
    assert_eq!(
        commands[0].args,
        vec![
            "loco".to_string(),
            "generate".to_string(),
            "model".to_string(),
            "posts".to_string(),
            "title:string".to_string(),
        ]
    );
}

#[test]
fn list_tasks_honours_environment() {
    let executor = Arc::new(FakeCommandExecutor::default());
    let service = service_with_executor(Arc::clone(&executor));
    let request = ListTasksRequest {
        environment: Some("qa".into()),
    };

    service.list_tasks(&request).expect("command to succeed");

    let commands = executor.recorded();
    assert_eq!(commands.len(), 1);
    assert_eq!(
        commands[0].args,
        vec![
            "loco".to_string(),
            "task".to_string(),
            "--environment".to_string(),
            "qa".to_string(),
        ]
    );
}

#[test]
fn run_task_composes_arguments() {
    let executor = Arc::new(FakeCommandExecutor::default());
    let service = service_with_executor(Arc::clone(&executor));
    let request = RunTaskRequest {
        environment: Some("dev".into()),
        task: "parse_args".into(),
        arguments: vec!["foo:bar".into(), "alpha:one".into()],
    };

    service.run_task(&request).expect("command to succeed");

    let commands = executor.recorded();
    assert_eq!(commands.len(), 1);
    assert_eq!(
        commands[0].args,
        vec![
            "loco".to_string(),
            "task".to_string(),
            "parse_args".to_string(),
            "foo:bar".to_string(),
            "alpha:one".to_string(),
            "--environment".to_string(),
            "dev".to_string(),
        ]
    );
}

#[test]
fn list_jobs_includes_filters() {
    let executor = Arc::new(FakeCommandExecutor::default());
    let service = service_with_executor(Arc::clone(&executor));
    let request = ListJobsRequest {
        environment: Some("staging".into()),
        config_path: Some("config/scheduler.yml".into()),
        name: Some("nightly".into()),
        tag: Some("reports".into()),
    };

    service.list_jobs(&request).expect("command to succeed");

    let commands = executor.recorded();
    assert_eq!(commands.len(), 1);
    assert_eq!(
        commands[0].args,
        vec![
            "loco".to_string(),
            "scheduler".to_string(),
            "--list".to_string(),
            "--config".to_string(),
            "config/scheduler.yml".to_string(),
            "--name".to_string(),
            "nightly".to_string(),
            "--tag".to_string(),
            "reports".to_string(),
            "--environment".to_string(),
            "staging".to_string(),
        ]
    );
}

#[test]
fn enqueue_job_appends_all_options() {
    let executor = Arc::new(FakeCommandExecutor::default());
    let service = service_with_executor(Arc::clone(&executor));
    let request = EnqueueJobRequest {
        environment: Some("test".into()),
        job_name: "CleanupJob".into(),
        queue: Some("critical".into()),
        run_at: Some("2024-01-01T00:00:00Z".into()),
        tags: vec!["fast".into(), "nightly".into()],
        payload: Some("{\"scope\":\"all\"}".into()),
        arguments: vec!["priority=high".into()],
    };

    service.enqueue_job(&request).expect("command to succeed");

    let commands = executor.recorded();
    assert_eq!(commands.len(), 1);
    assert_eq!(
        commands[0].args,
        vec![
            "loco".to_string(),
            "jobs".to_string(),
            "enqueue".to_string(),
            "CleanupJob".to_string(),
            "--queue".to_string(),
            "critical".to_string(),
            "--run-at".to_string(),
            "2024-01-01T00:00:00Z".to_string(),
            "--tag".to_string(),
            "fast".to_string(),
            "--tag".to_string(),
            "nightly".to_string(),
            "--payload".to_string(),
            "{\"scope\":\"all\"}".to_string(),
            "priority=high".to_string(),
            "--environment".to_string(),
            "test".to_string(),
        ]
    );
}

#[test]
fn run_doctor_pushes_flags() {
    let executor = Arc::new(FakeCommandExecutor::default());
    let service = service_with_executor(Arc::clone(&executor));
    let request = RunDoctorRequest {
        environment: Some("production".into()),
        production: true,
        config: true,
        graph: false,
        assistant: false,
    };

    service.run_doctor(&request).expect("command to succeed");

    let commands = executor.recorded();
    assert_eq!(commands.len(), 1);
    assert_eq!(
        commands[0].args,
        vec![
            "loco".to_string(),
            "doctor".to_string(),
            "--production".to_string(),
            "--config".to_string(),
            "--environment".to_string(),
            "production".to_string(),
        ]
    );
}

#[test]
fn job_status_builds_command_with_environment() {
    let executor = Arc::new(FakeCommandExecutor::new(CommandOutput::new(
        0,
        r#"{"id":"job-9","state":"queued"}"#,
        "",
    )));
    let service = service_with_executor(Arc::clone(&executor));
    let request = JobStatusRequest {
        environment: Some("prod".into()),
        job_id: "job-9".into(),
    };

    service
        .job_status(&request)
        .expect("job status command to succeed");

    let commands = executor.recorded();
    assert_eq!(commands.len(), 1);
    assert_eq!(
        commands[0].args,
        vec![
            "loco".to_string(),
            "jobs".to_string(),
            "status".to_string(),
            "job-9".to_string(),
            "--environment".to_string(),
            "prod".to_string(),
        ]
    );
}

#[test]
fn job_status_parses_cli_output() {
    let executor = Arc::new(FakeCommandExecutor::new(CommandOutput::new(
        0,
        r#"{"id":"job-7","state":"completed","result":{"status":0,"stdout":"done","stderr":""},"updatedAt":"2024-01-01T00:00:00Z"}"#,
        "",
    )));
    let service = service_with_executor(Arc::clone(&executor));
    let request = JobStatusRequest {
        job_id: "job-7".into(),
        ..JobStatusRequest::default()
    };

    let response = service
        .job_status(&request)
        .expect("job status command to succeed");

    assert_eq!(
        response,
        JobStatusResponse {
            id: "job-7".into(),
            state: "completed".into(),
            result: Some(CommandOutput::new(0, "done", "")),
            error: None,
            updated_at: Some("2024-01-01T00:00:00Z".into()),
        }
    );
}
