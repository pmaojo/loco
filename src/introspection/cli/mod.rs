use std::fmt;

use crate::Result;

pub mod adapters;

/// Represents a command invocation for `cargo loco`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CliCommand {
    pub program: String,
    pub args: Vec<String>,
}

impl CliCommand {
    #[must_use]
    pub fn new(program: impl Into<String>, args: impl Into<Vec<String>>) -> Self {
        Self {
            program: program.into(),
            args: args.into(),
        }
    }
}

impl fmt::Display for CliCommand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.program)?;
        for arg in &self.args {
            write!(f, " {}", arg)?;
        }
        Ok(())
    }
}

/// Output produced after running a CLI command.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CommandOutput {
    pub status: i32,
    pub stdout: String,
    pub stderr: String,
}

impl CommandOutput {
    #[must_use]
    pub fn new(status: i32, stdout: impl Into<String>, stderr: impl Into<String>) -> Self {
        Self {
            status,
            stdout: stdout.into(),
            stderr: stderr.into(),
        }
    }
}

/// Executes operating system commands.
pub trait CommandExecutor: Send + Sync {
    fn execute(&self, command: &CliCommand) -> Result<CommandOutput>;
}

/// Defines the safe subset of `cargo loco` commands exposed for automation.
pub trait CliAutomationService: Send + Sync {
    fn list_generators(&self, request: &ListGeneratorsRequest) -> Result<CommandOutput>;
    fn run_generator(&self, request: &RunGeneratorRequest) -> Result<CommandOutput>;
    fn list_tasks(&self, request: &ListTasksRequest) -> Result<CommandOutput>;
    fn list_jobs(&self, request: &ListJobsRequest) -> Result<CommandOutput>;
    fn enqueue_job(&self, request: &EnqueueJobRequest) -> Result<CommandOutput>;
    fn run_doctor(&self, request: &RunDoctorRequest) -> Result<CommandOutput>;
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ListGeneratorsRequest {
    pub environment: Option<String>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RunGeneratorRequest {
    pub environment: Option<String>,
    pub generator: String,
    pub arguments: Vec<String>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ListTasksRequest {
    pub environment: Option<String>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ListJobsRequest {
    pub environment: Option<String>,
    pub config_path: Option<String>,
    pub name: Option<String>,
    pub tag: Option<String>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct EnqueueJobRequest {
    pub environment: Option<String>,
    pub job_name: String,
    pub queue: Option<String>,
    pub run_at: Option<String>,
    pub tags: Vec<String>,
    pub payload: Option<String>,
    pub arguments: Vec<String>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RunDoctorRequest {
    pub environment: Option<String>,
    pub production: bool,
    pub config: bool,
    pub graph: bool,
    pub assistant: bool,
}
