use std::{collections::BTreeMap, convert::Infallible, fmt::Write as _, sync::Arc};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::{
    app::SharedStore,
    doctor::{Check, CheckStatus, Resource},
    introspection::graph::{
        domain::RouteDescriptor,
        service::{GraphDependencies, GraphQueryService, GraphSnapshot},
    },
};

const SYSTEM_PROMPT: &str = "You are an engineering assistant analysing Loco introspection data. Reference node identifiers when recommending changes.";

/// High level status for a doctor check result.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DoctorStatus {
    /// The check passed successfully.
    Passing,
    /// The check failed and needs attention.
    Failing,
    /// The check is not configured or inconclusive.
    Warning,
}

impl From<&CheckStatus> for DoctorStatus {
    fn from(status: &CheckStatus) -> Self {
        match status {
            CheckStatus::Ok => Self::Passing,
            CheckStatus::NotOk => Self::Failing,
            CheckStatus::NotConfigure => Self::Warning,
        }
    }
}

impl DoctorStatus {
    fn label(self) -> &'static str {
        match self {
            Self::Passing => "passing",
            Self::Failing => "failing",
            Self::Warning => "warning",
        }
    }
}

/// Serializable doctor finding used when invoking the assistant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DoctorFinding {
    pub resource: String,
    pub status: DoctorStatus,
    pub message: String,
    pub detail: Option<String>,
}

impl DoctorFinding {
    /// Builds a finding from a doctor [`Check`].
    #[must_use]
    pub fn from_check(resource: &Resource, check: &Check) -> Self {
        Self {
            resource: describe_resource(resource),
            status: DoctorStatus::from(&check.status),
            message: check.message.clone(),
            detail: check.description.clone(),
        }
    }
}

fn describe_resource(resource: &Resource) -> String {
    match resource {
        Resource::SeaOrmCLI => "SeaOrmCLI".to_string(),
        Resource::Database => "Database".to_string(),
        Resource::Queue => "Queue".to_string(),
        Resource::Deps => "Dependencies".to_string(),
        Resource::PublishedLocoVersion => "PublishedLocoVersion".to_string(),
        Resource::Initializer(name) => format!("Initializer:{name}"),
    }
}

/// Converts doctor check results into serialisable findings expected by the assistant.
#[must_use]
pub fn findings_from_checks(checks: &BTreeMap<Resource, Check>) -> Vec<DoctorFinding> {
    checks
        .iter()
        .map(|(resource, check)| DoctorFinding::from_check(resource, check))
        .collect()
}

/// Role of a conversation turn.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConversationRole {
    User,
    Assistant,
}

/// Represents a single entry in the assistant conversation history.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConversationTurn {
    pub role: ConversationRole,
    pub content: String,
}

impl ConversationTurn {
    #[must_use]
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: ConversationRole::User,
            content: content.into(),
        }
    }

    #[must_use]
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: ConversationRole::Assistant,
            content: content.into(),
        }
    }
}

/// Prompt sent to the assistant provider.
#[derive(Debug, Clone)]
pub struct AssistantPrompt {
    pub system: String,
    pub history: Vec<ConversationTurn>,
    pub user: String,
}

/// Request handed to an [`AssistantClient`].
#[derive(Debug, Clone)]
pub struct AssistantRequest {
    pub app_name: String,
    pub prompt: AssistantPrompt,
    pub graph: GraphSnapshot,
    pub doctor_findings: Vec<DoctorFinding>,
}

/// Suggestion returned by the assistant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AssistantSuggestion {
    pub node_id: String,
    pub summary: String,
    pub rationale: Option<String>,
}

/// Advice returned to adapters.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AssistantAdvice {
    pub response: String,
    pub suggestions: Vec<AssistantSuggestion>,
}

/// Provider response payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssistantCompletion {
    pub reply: String,
    pub suggestions: Vec<AssistantSuggestion>,
}

/// Conversation state stored across invocations.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AssistantState {
    pub history: Vec<ConversationTurn>,
}

/// Abstraction over a conversation state storage backend.
pub trait ConversationStore: Send + Sync {
    fn load(&self) -> AssistantState;
    fn save(&self, state: AssistantState);
}

/// `SharedStore` backed conversation repository.
#[derive(Clone)]
pub struct SharedStoreConversationStore {
    shared: Arc<SharedStore>,
}

impl SharedStoreConversationStore {
    #[must_use]
    pub fn new(shared: Arc<SharedStore>) -> Self {
        Self { shared }
    }
}

impl ConversationStore for SharedStoreConversationStore {
    fn load(&self) -> AssistantState {
        self.shared
            .get::<AssistantState>()
            .unwrap_or_else(AssistantState::default)
    }

    fn save(&self, state: AssistantState) {
        self.shared.insert(state);
    }
}

/// Errors produced by the assistant pipeline.
#[derive(Debug, thiserror::Error)]
pub enum AssistantError {
    #[error("assistant client error: {0}")]
    Client(String),
}

/// Client abstraction representing an AI provider.
#[async_trait]
pub trait AssistantClient: Send + Sync {
    type Error: std::error::Error + Send + Sync + 'static;

    async fn complete(&self, request: AssistantRequest)
        -> Result<AssistantCompletion, Self::Error>;
}

/// Adapter orchestrating prompt creation, conversation management and provider interaction.
pub struct IntrospectionAssistant<'a, Q, C, S> {
    app_name: &'a str,
    graph: &'a Q,
    client: &'a C,
    store: &'a S,
}

impl<'a, Q, C, S> IntrospectionAssistant<'a, Q, C, S>
where
    Q: GraphQueryService + Send + Sync,
    C: AssistantClient,
    S: ConversationStore,
{
    #[must_use]
    pub fn new(app_name: &'a str, graph: &'a Q, client: &'a C, store: &'a S) -> Self {
        Self {
            app_name,
            graph,
            client,
            store,
        }
    }

    /// Requests advice from the configured assistant provider.
    pub async fn advise(
        &self,
        doctor_findings: &[DoctorFinding],
    ) -> Result<AssistantAdvice, AssistantError> {
        let snapshot = self.graph.snapshot();
        let mut state = self.store.load();
        let prompt_text = build_prompt(self.app_name, &snapshot, doctor_findings);

        let request = AssistantRequest {
            app_name: self.app_name.to_string(),
            prompt: AssistantPrompt {
                system: SYSTEM_PROMPT.to_string(),
                history: state.history.clone(),
                user: prompt_text.clone(),
            },
            graph: snapshot.clone(),
            doctor_findings: doctor_findings.to_vec(),
        };

        let completion = self
            .client
            .complete(request)
            .await
            .map_err(|err| AssistantError::Client(err.to_string()))?;

        state.history.push(ConversationTurn::user(prompt_text));
        state
            .history
            .push(ConversationTurn::assistant(completion.reply.clone()));
        self.store.save(state);

        Ok(AssistantAdvice {
            response: completion.reply,
            suggestions: completion.suggestions,
        })
    }
}

fn build_prompt(app_name: &str, snapshot: &GraphSnapshot, findings: &[DoctorFinding]) -> String {
    let mut prompt = String::new();
    writeln!(prompt, "Application: {app_name}").unwrap();
    writeln!(
        prompt,
        "Graph health: {}",
        if snapshot.health.ok { "ok" } else { "not ok" }
    )
    .unwrap();
    prompt.push('\n');

    append_routes(&mut prompt, &snapshot.routes);
    append_background_workers(&mut prompt, &snapshot.dependencies);
    append_scheduler_jobs(&mut prompt, &snapshot.dependencies);
    append_tasks(&mut prompt, &snapshot.dependencies);
    append_findings(&mut prompt, findings);

    prompt.push_str(
        "\nProvide actionable recommendations that reference the node identifiers above.",
    );
    prompt
}

fn append_routes(buffer: &mut String, routes: &[RouteDescriptor]) {
    buffer.push_str("Routes:\n");
    if routes.is_empty() {
        buffer.push_str("- none defined\n");
        buffer.push('\n');
        return;
    }

    for route in routes {
        let mut methods = route.methods.clone();
        methods.sort();
        methods.dedup();
        let joined = if methods.is_empty() {
            "unknown".to_string()
        } else {
            methods.join(", ")
        };
        writeln!(buffer, "- route:{} (methods: {joined})", route.path).unwrap();
    }
    buffer.push('\n');
}

fn append_background_workers(buffer: &mut String, dependencies: &GraphDependencies) {
    buffer.push_str("Background workers:\n");
    if dependencies.background_workers.is_empty() {
        buffer.push_str("- none registered\n");
        buffer.push('\n');
        return;
    }

    for worker in &dependencies.background_workers {
        let queue = worker
            .queue
            .clone()
            .unwrap_or_else(|| "unspecified".to_string());
        writeln!(buffer, "- worker:{} (queue: {queue})", worker.name).unwrap();
    }
    buffer.push('\n');
}

fn append_scheduler_jobs(buffer: &mut String, dependencies: &GraphDependencies) {
    buffer.push_str("Scheduler jobs:\n");
    if dependencies.scheduler_jobs.is_empty() {
        buffer.push_str("- none configured\n");
        buffer.push('\n');
        return;
    }

    for job in &dependencies.scheduler_jobs {
        let tags = if job.tags.is_empty() {
            "none".to_string()
        } else {
            job.tags.join(", ")
        };
        writeln!(
            buffer,
            "- scheduler:{} (schedule: {}, command: {}, tags: {tags})",
            job.name, job.schedule, job.command
        )
        .unwrap();
    }
    buffer.push('\n');
}

fn append_tasks(buffer: &mut String, dependencies: &GraphDependencies) {
    buffer.push_str("Tasks:\n");
    if dependencies.tasks.is_empty() {
        buffer.push_str("- none registered\n");
        buffer.push('\n');
        return;
    }

    for task in &dependencies.tasks {
        match &task.detail {
            Some(detail) => {
                writeln!(buffer, "- task:{} (detail: {detail})", task.name).unwrap();
            }
            None => {
                writeln!(buffer, "- task:{}", task.name).unwrap();
            }
        }
    }
    buffer.push('\n');
}

fn append_findings(buffer: &mut String, findings: &[DoctorFinding]) {
    buffer.push_str("Doctor findings:\n");
    if findings.is_empty() {
        buffer.push_str("- no doctor data provided\n");
        return;
    }

    for finding in findings {
        match &finding.detail {
            Some(detail) => {
                writeln!(
                    buffer,
                    "- {} => {} ({})",
                    finding.resource,
                    finding.status.label(),
                    format!("{} - {detail}", finding.message)
                )
                .unwrap();
            }
            None => {
                writeln!(
                    buffer,
                    "- {} => {} ({})",
                    finding.resource,
                    finding.status.label(),
                    finding.message
                )
                .unwrap();
            }
        }
    }
}

/// Simple rule-based assistant used as the default implementation when no remote provider is configured.
#[derive(Debug, Default)]
pub struct RuleBasedAssistantClient;

#[async_trait]
impl AssistantClient for RuleBasedAssistantClient {
    type Error = Infallible;

    async fn complete(
        &self,
        request: AssistantRequest,
    ) -> Result<AssistantCompletion, Self::Error> {
        let mut suggestions: Vec<AssistantSuggestion> = request
            .doctor_findings
            .iter()
            .filter(|finding| finding.status != DoctorStatus::Passing)
            .map(|finding| AssistantSuggestion {
                node_id: format!("app:{}", request.app_name),
                summary: format!("Investigate {}", finding.resource),
                rationale: Some(match &finding.detail {
                    Some(detail) => format!("{} - {detail}", finding.message),
                    None => finding.message.clone(),
                }),
            })
            .collect();

        if suggestions.is_empty() {
            if let Some(route) = request.graph.routes.first() {
                suggestions.push(AssistantSuggestion {
                    node_id: format!("route:{}", route.path),
                    summary: format!("Review {} route for optimisation opportunities", route.path),
                    rationale: Some("No doctor warnings were reported; consider confirming the route behaviour.".to_string()),
                });
            } else {
                suggestions.push(AssistantSuggestion {
                    node_id: format!("app:{}", request.app_name),
                    summary: "System appears healthy. Continue monitoring.".to_string(),
                    rationale: Some("No doctor warnings were reported.".to_string()),
                });
            }
        }

        Ok(AssistantCompletion {
            reply: "Generated suggestions from local rules.".to_string(),
            suggestions,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::introspection::graph::{
        domain::{BackgroundWorkerDescriptor, SchedulerJobDescriptor, TaskDescriptor},
        service::GraphHealth,
    };
    use std::sync::Mutex;

    #[derive(Clone)]
    struct StubGraphService {
        snapshot: GraphSnapshot,
    }

    impl GraphQueryService for StubGraphService {
        fn snapshot(&self) -> GraphSnapshot {
            self.snapshot.clone()
        }
    }

    #[derive(Clone)]
    struct RecordingClient {
        last: Arc<Mutex<Option<AssistantRequest>>>,
        completion: AssistantCompletion,
    }

    impl RecordingClient {
        fn new(completion: AssistantCompletion) -> Self {
            Self {
                last: Arc::new(Mutex::new(None)),
                completion,
            }
        }

        fn captured(&self) -> AssistantRequest {
            self.last
                .lock()
                .unwrap()
                .clone()
                .expect("request was captured")
        }
    }

    #[async_trait]
    impl AssistantClient for RecordingClient {
        type Error = Infallible;

        async fn complete(
            &self,
            request: AssistantRequest,
        ) -> Result<AssistantCompletion, Self::Error> {
            *self.last.lock().unwrap() = Some(request);
            Ok(self.completion.clone())
        }
    }

    fn sample_snapshot() -> GraphSnapshot {
        GraphSnapshot {
            routes: vec![RouteDescriptor {
                path: "/health".to_string(),
                methods: vec!["GET".to_string()],
            }],
            dependencies: GraphDependencies {
                background_workers: vec![BackgroundWorkerDescriptor {
                    name: "mailer".to_string(),
                    queue: Some("redis".to_string()),
                }],
                scheduler_jobs: vec![SchedulerJobDescriptor {
                    name: "daily".to_string(),
                    schedule: "0 0 * * *".to_string(),
                    command: "task cleanup".to_string(),
                    run_on_start: false,
                    shell: true,
                    tags: vec!["maintenance".to_string()],
                }],
                tasks: vec![TaskDescriptor {
                    name: "cleanup".to_string(),
                    detail: Some("remove temp files".to_string()),
                }],
            },
            health: GraphHealth { ok: true },
        }
    }

    fn failing_finding() -> DoctorFinding {
        DoctorFinding {
            resource: "Queue".to_string(),
            status: DoctorStatus::Failing,
            message: "queue connection: failed".to_string(),
            detail: Some("redis is unreachable".to_string()),
        }
    }

    #[tokio::test]
    async fn formats_prompt_and_returns_suggestions() {
        let snapshot = sample_snapshot();
        let graph = StubGraphService {
            snapshot: snapshot.clone(),
        };
        let shared_store = Arc::new(SharedStore::default());
        let store = SharedStoreConversationStore::new(Arc::clone(&shared_store));
        let completion = AssistantCompletion {
            reply: "Mock reply".to_string(),
            suggestions: vec![AssistantSuggestion {
                node_id: "route:/health".to_string(),
                summary: "Verify health endpoint".to_string(),
                rationale: Some("Ensure monitoring matches requirements.".to_string()),
            }],
        };
        let client = RecordingClient::new(completion.clone());
        let assistant = IntrospectionAssistant::new("demo", &graph, &client, &store);

        let advice = assistant
            .advise(&[failing_finding()])
            .await
            .expect("assistant advice");

        let captured = client.captured();
        assert!(captured.prompt.user.contains("route:/health"));
        assert!(captured.prompt.user.contains("Doctor findings"));
        assert_eq!(advice.suggestions, completion.suggestions);

        let state = shared_store
            .get::<AssistantState>()
            .expect("state stored in shared store");
        assert_eq!(state.history.len(), 2);
    }

    #[tokio::test]
    async fn reuses_conversation_history_between_calls() {
        let graph = StubGraphService {
            snapshot: sample_snapshot(),
        };
        let shared_store = Arc::new(SharedStore::default());
        let store = SharedStoreConversationStore::new(Arc::clone(&shared_store));

        let first = RecordingClient::new(AssistantCompletion {
            reply: "First".to_string(),
            suggestions: vec![],
        });
        let assistant = IntrospectionAssistant::new("demo", &graph, &first, &store);
        assistant
            .advise(&[failing_finding()])
            .await
            .expect("first call succeeds");

        let second = RecordingClient::new(AssistantCompletion {
            reply: "Second".to_string(),
            suggestions: vec![],
        });
        let assistant = IntrospectionAssistant::new("demo", &graph, &second, &store);
        assistant.advise(&[]).await.expect("second call succeeds");

        let captured = second.captured();
        assert_eq!(captured.prompt.history.len(), 2);
        assert!(captured
            .prompt
            .history
            .iter()
            .any(|turn| matches!(turn.role, ConversationRole::Assistant)));
    }
}
