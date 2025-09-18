use std::{collections::BTreeMap, sync::Arc};

use axum::extract::{Query, State};
use axum::routing::{get, post};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::{
    app::AppContext,
    controller::{format, Json, Routes},
    errors::Error,
    introspection::cli::{
        CliAutomationService, CommandOutput, ListGeneratorsRequest, ListTasksRequest,
        RunDoctorRequest, RunGeneratorRequest, RunTaskRequest,
    },
    Result,
};

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ListableCommand {
    pub command: String,
    pub summary: String,
}

#[derive(Debug, Deserialize)]
pub struct GenerationRequest {
    pub generator: String,
    #[serde(default)]
    pub arguments: Vec<String>,
    #[serde(default)]
    pub environment: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct TaskRunRequest {
    pub task: String,
    #[serde(default)]
    pub arguments: Vec<String>,
    #[serde(default)]
    pub params: BTreeMap<String, String>,
    #[serde(default)]
    pub environment: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct DoctorSnapshotRequest {
    #[serde(default)]
    pub environment: Option<String>,
    #[serde(default)]
    pub production: bool,
    #[serde(default)]
    pub config: bool,
    #[serde(default)]
    pub graph: bool,
    #[serde(default)]
    pub assistant: bool,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct CommandExecution {
    pub status: i32,
    pub stdout: String,
    pub stderr: String,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct DoctorSnapshotResponse {
    pub status: i32,
    pub stdout: Value,
    pub stderr: String,
}

#[derive(Debug, Default, Deserialize)]
pub struct AutomationQuery {
    environment: Option<String>,
}

pub fn routes() -> Routes {
    Routes::new()
        .add("/__loco/cli/generators", get(list_generators))
        .add("/__loco/cli/generators/run", post(run_generator))
        .add("/__loco/cli/tasks", get(list_tasks))
        .add("/__loco/cli/tasks/run", post(run_task))
        .add("/__loco/cli/doctor/snapshot", post(doctor_snapshot))
}

pub async fn list_generators(
    State(ctx): State<AppContext>,
    Query(query): Query<AutomationQuery>,
) -> Result<axum::response::Response> {
    let service = resolve_service(&ctx)?;
    let request = ListGeneratorsRequest {
        environment: query.environment,
    };
    let output = service.list_generators(&request)?;
    let commands = parse_listable_commands(&output.stdout);
    format::json(commands)
}

pub async fn run_generator(
    State(ctx): State<AppContext>,
    Json(payload): Json<GenerationRequest>,
) -> Result<axum::response::Response> {
    let service = resolve_service(&ctx)?;
    let GenerationRequest {
        generator,
        arguments,
        environment,
    } = payload;
    let request = RunGeneratorRequest {
        environment,
        generator,
        arguments,
    };
    let output = service.run_generator(&request)?;
    format::json(CommandExecution::from(output))
}

pub async fn list_tasks(
    State(ctx): State<AppContext>,
    Query(query): Query<AutomationQuery>,
) -> Result<axum::response::Response> {
    let service = resolve_service(&ctx)?;
    let request = ListTasksRequest {
        environment: query.environment,
    };
    let output = service.list_tasks(&request)?;
    let commands = parse_listable_commands(&output.stdout);
    format::json(commands)
}

pub async fn run_task(
    State(ctx): State<AppContext>,
    Json(payload): Json<TaskRunRequest>,
) -> Result<axum::response::Response> {
    let service = resolve_service(&ctx)?;
    let TaskRunRequest {
        task,
        mut arguments,
        params,
        environment,
    } = payload;
    arguments.extend(
        params
            .into_iter()
            .map(|(key, value)| format!("{key}:{value}")),
    );
    let request = RunTaskRequest {
        environment,
        task,
        arguments,
    };
    let output = service.run_task(&request)?;
    format::json(CommandExecution::from(output))
}

pub async fn doctor_snapshot(
    State(ctx): State<AppContext>,
    Json(payload): Json<DoctorSnapshotRequest>,
) -> Result<axum::response::Response> {
    let service = resolve_service(&ctx)?;
    let DoctorSnapshotRequest {
        environment,
        production,
        config,
        graph,
        assistant,
    } = payload;
    let request = RunDoctorRequest {
        environment,
        production,
        config,
        graph,
        assistant,
    };
    let output = service.run_doctor(&request)?;
    format::json(DoctorSnapshotResponse::from(output))
}

fn resolve_service(ctx: &AppContext) -> Result<Arc<dyn CliAutomationService>> {
    if !ctx.config.introspection.console.enabled {
        return Err(Error::NotFound);
    }
    ctx.shared_store
        .get_ref::<Arc<dyn CliAutomationService>>()
        .map(|service| Arc::clone(&*service))
        .ok_or(Error::NotFound)
}

fn parse_listable_commands(stdout: &str) -> Vec<ListableCommand> {
    stdout.lines().filter_map(parse_listable_command).collect()
}

fn parse_listable_command(line: &str) -> Option<ListableCommand> {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.ends_with(':') {
        return None;
    }
    if trimmed.starts_with('-') {
        return None;
    }

    let split = trimmed.find(char::is_whitespace)?;
    let (command, summary) = trimmed.split_at(split);
    let command = command.trim();
    let summary = summary.trim();

    if command.is_empty() || summary.is_empty() {
        return None;
    }

    Some(ListableCommand {
        command: command.to_string(),
        summary: summary.to_string(),
    })
}

impl From<CommandOutput> for CommandExecution {
    fn from(output: CommandOutput) -> Self {
        let CommandOutput {
            status,
            stdout,
            stderr,
        } = output;
        Self {
            status,
            stdout,
            stderr,
        }
    }
}

impl From<CommandOutput> for DoctorSnapshotResponse {
    fn from(output: CommandOutput) -> Self {
        let CommandOutput {
            status,
            stdout,
            stderr,
        } = output;
        let stdout_value =
            serde_json::from_str(&stdout).unwrap_or_else(|_| json!({ "raw": stdout }));
        Self {
            status,
            stdout: stdout_value,
            stderr,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_listable_command_supports_single_space_separator() {
        let command = parse_listable_command("model Generates a new model");

        assert_eq!(
            command,
            Some(ListableCommand {
                command: "model".into(),
                summary: "Generates a new model".into(),
            })
        );
    }

    #[test]
    fn parse_listable_command_supports_tab_separator() {
        let command = parse_listable_command("migration\tGenerates a migration");

        assert_eq!(
            command,
            Some(ListableCommand {
                command: "migration".into(),
                summary: "Generates a migration".into(),
            })
        );
    }
}
