use crate::introspection::cli::{
    CliCommand, EnqueueJobRequest, JobStatusRequest, ListGeneratorsRequest, ListJobsRequest,
    ListTasksRequest, RunDoctorRequest, RunGeneratorRequest, RunTaskRequest,
};

#[derive(Default)]
pub struct CargoAutomationCommandBuilder;

impl CargoAutomationCommandBuilder {
    const PROGRAM: &'static str = "cargo";

    fn apply_environment(args: &mut Vec<String>, environment: Option<&String>) {
        if let Some(env) = environment {
            args.push("--environment".into());
            args.push(env.clone());
        }
    }

    fn build_command(args: Vec<String>, environment: Option<&String>) -> CliCommand {
        let mut args = args;
        Self::apply_environment(&mut args, environment);
        CliCommand::new(Self::PROGRAM, args)
    }

    #[must_use]
    pub fn list_generators(request: &ListGeneratorsRequest) -> CliCommand {
        let args = vec!["loco".into(), "generate".into(), "--help".into()];
        Self::build_command(args, request.environment.as_ref())
    }

    #[must_use]
    pub fn run_generator(request: &RunGeneratorRequest) -> CliCommand {
        let mut args = vec!["loco".into(), "generate".into(), request.generator.clone()];
        args.extend(request.arguments.clone());
        Self::build_command(args, request.environment.as_ref())
    }

    #[must_use]
    pub fn list_tasks(request: &ListTasksRequest) -> CliCommand {
        let args = vec!["loco".into(), "task".into()];
        Self::build_command(args, request.environment.as_ref())
    }

    #[must_use]
    pub fn run_task(request: &RunTaskRequest) -> CliCommand {
        let mut args = vec!["loco".into(), "task".into(), request.task.clone()];
        args.extend(request.arguments.clone());
        Self::build_command(args, request.environment.as_ref())
    }

    #[must_use]
    pub fn list_jobs(request: &ListJobsRequest) -> CliCommand {
        let mut args = vec!["loco".into(), "scheduler".into(), "--list".into()];
        if let Some(config_path) = &request.config_path {
            args.push("--config".into());
            args.push(config_path.clone());
        }
        if let Some(name) = &request.name {
            args.push("--name".into());
            args.push(name.clone());
        }
        if let Some(tag) = &request.tag {
            args.push("--tag".into());
            args.push(tag.clone());
        }
        Self::build_command(args, request.environment.as_ref())
    }

    #[must_use]
    pub fn enqueue_job(request: &EnqueueJobRequest) -> CliCommand {
        let mut args = vec![
            "loco".into(),
            "jobs".into(),
            "enqueue".into(),
            request.job_name.clone(),
        ];
        if let Some(queue) = &request.queue {
            args.push("--queue".into());
            args.push(queue.clone());
        }
        if let Some(run_at) = &request.run_at {
            args.push("--run-at".into());
            args.push(run_at.clone());
        }
        for tag in &request.tags {
            args.push("--tag".into());
            args.push(tag.clone());
        }
        if let Some(payload) = &request.payload {
            args.push("--payload".into());
            args.push(payload.clone());
        }
        args.extend(request.arguments.clone());
        Self::build_command(args, request.environment.as_ref())
    }

    #[must_use]
    pub fn job_status(request: &JobStatusRequest) -> CliCommand {
        let args = vec![
            "loco".into(),
            "jobs".into(),
            "status".into(),
            request.job_id.clone(),
        ];
        Self::build_command(args, request.environment.as_ref())
    }

    #[must_use]
    pub fn run_doctor(request: &RunDoctorRequest) -> CliCommand {
        let mut args = vec!["loco".into(), "doctor".into()];
        if request.production {
            args.push("--production".into());
        }
        if request.config {
            args.push("--config".into());
        }
        if request.graph {
            args.push("--graph".into());
        }
        #[cfg(feature = "introspection_assistant")]
        if request.assistant {
            args.push("--assistant".into());
        }
        Self::build_command(args, request.environment.as_ref())
    }
}
