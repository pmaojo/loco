use std::sync::Arc;

use crate::cli::automation::CargoAutomationCommandBuilder;
use crate::introspection::cli::{
    CliAutomationService, CliCommand, CommandExecutor, CommandOutput, EnqueueJobRequest,
    JobStatusRequest, JobStatusResponse, ListGeneratorsRequest, ListJobsRequest, ListTasksRequest,
    RunDoctorRequest, RunGeneratorRequest, RunTaskRequest,
};
use crate::{Error, Result};
use serde::Deserialize;

pub struct CargoCliAutomationService<E: CommandExecutor> {
    executor: Arc<E>,
}

impl<E: CommandExecutor> CargoCliAutomationService<E> {
    #[must_use]
    pub fn new(executor: Arc<E>) -> Self {
        Self { executor }
    }

    fn execute(&self, command: CliCommand) -> Result<CommandOutput> {
        self.executor.execute(&command)
    }
}

impl<E: CommandExecutor> CliAutomationService for CargoCliAutomationService<E> {
    fn list_generators(&self, request: &ListGeneratorsRequest) -> Result<CommandOutput> {
        let command = CargoAutomationCommandBuilder::list_generators(request);
        self.execute(command)
    }

    fn run_generator(&self, request: &RunGeneratorRequest) -> Result<CommandOutput> {
        let command = CargoAutomationCommandBuilder::run_generator(request);
        self.execute(command)
    }

    fn list_tasks(&self, request: &ListTasksRequest) -> Result<CommandOutput> {
        let command = CargoAutomationCommandBuilder::list_tasks(request);
        self.execute(command)
    }

    fn run_task(&self, request: &RunTaskRequest) -> Result<CommandOutput> {
        let command = CargoAutomationCommandBuilder::run_task(request);
        self.execute(command)
    }

    fn list_jobs(&self, request: &ListJobsRequest) -> Result<CommandOutput> {
        let command = CargoAutomationCommandBuilder::list_jobs(request);
        self.execute(command)
    }

    fn enqueue_job(&self, request: &EnqueueJobRequest) -> Result<CommandOutput> {
        let command = CargoAutomationCommandBuilder::enqueue_job(request);
        self.execute(command)
    }

    fn job_status(&self, request: &JobStatusRequest) -> Result<JobStatusResponse> {
        let command = CargoAutomationCommandBuilder::job_status(request);
        let output = self.execute(command)?;
        parse_job_status(&output.stdout)
    }

    fn run_doctor(&self, request: &RunDoctorRequest) -> Result<CommandOutput> {
        let command = CargoAutomationCommandBuilder::run_doctor(request);
        self.execute(command)
    }
}

#[derive(Debug, Deserialize)]
struct JobStatusPayload {
    id: String,
    state: String,
    #[serde(default)]
    result: Option<JobResultPayload>,
    #[serde(default)]
    error: Option<String>,
    #[serde(rename = "updatedAt", default)]
    updated_at: Option<String>,
}

#[derive(Debug, Deserialize)]
struct JobResultPayload {
    status: i32,
    #[serde(default)]
    stdout: String,
    #[serde(default)]
    stderr: String,
}

fn parse_job_status(stdout: &str) -> Result<JobStatusResponse> {
    let payload: JobStatusPayload = serde_json::from_str(stdout)
        .map_err(|err| Error::Message(format!("failed to parse job status response: {err}")))?;
    Ok(JobStatusResponse {
        id: payload.id,
        state: payload.state,
        result: payload
            .result
            .map(|result| CommandOutput::new(result.status, result.stdout, result.stderr)),
        error: payload.error,
        updated_at: payload.updated_at,
    })
}

#[derive(Default)]
pub struct StdCommandExecutor;

impl CommandExecutor for StdCommandExecutor {
    fn execute(&self, command: &CliCommand) -> Result<CommandOutput> {
        use std::process::Command;

        let output = Command::new(&command.program)
            .args(&command.args)
            .output()?;
        let status_code = output.status.code().unwrap_or(-1);
        let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

        if !output.status.success() {
            return Err(Error::Message(format!(
                "command `{}` failed with status {status_code}: {}",
                command,
                stderr.trim()
            )));
        }

        Ok(CommandOutput::new(status_code, stdout, stderr))
    }
}

impl CargoCliAutomationService<StdCommandExecutor> {
    #[must_use]
    pub fn system() -> Self {
        Self::new(Arc::new(StdCommandExecutor::default()))
    }
}

impl Default for CargoCliAutomationService<StdCommandExecutor> {
    fn default() -> Self {
        Self::system()
    }
}
