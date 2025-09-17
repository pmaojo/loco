use std::sync::Arc;

use async_trait::async_trait;
use thiserror::Error;

use crate::{
    config::{AiSettings, KnowledgeAssistantBackend},
    ontology::{
        service::{OntologyServiceError, ReasonerHandle},
        value_objects::Iri,
    },
};

pub mod infrastructure;
pub mod task;

/// Request issued to a [`KnowledgeAssistant`] implementation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KnowledgeRequest {
    /// Prompt provided by the caller.
    pub prompt: String,
    /// Ontology identifier anchoring the reasoning context.
    pub ontology: Iri,
    /// Aggregated reasoning results to be consumed by the assistant.
    pub inferences: Vec<ReasoningOutcome>,
}

impl KnowledgeRequest {
    /// Builds a textual representation of the reasoning context.
    #[must_use]
    pub fn context_as_text(&self) -> String {
        if self.inferences.is_empty() {
            return String::from("No ontology inferences were requested.");
        }

        let mut buffer = String::new();
        for (index, inference) in self.inferences.iter().enumerate() {
            if index > 0 {
                buffer.push_str("\n\n");
            }
            buffer.push_str(&inference.describe());
        }
        buffer
    }
}

/// High level response returned by assistant adapters.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KnowledgeResponse {
    /// Natural language answer returned by the provider.
    pub message: String,
}

/// Resulting synthesis combining raw reasoning outputs with the assistant
/// response.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KnowledgeSynthesis {
    /// Assistant message.
    pub message: String,
    /// Reasoning outcomes produced while orchestrating the request.
    pub inferences: Vec<ReasoningOutcome>,
}

/// Supported reasoning commands executed before delegating to an assistant.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReasoningCommand {
    /// Fetch the transitive closure of parent classes.
    Ancestors { class: Iri },
    /// Fetch the transitive closure of descendant classes.
    Descendants { class: Iri },
    /// Retrieve individuals connected through the provided property.
    RelatedIndividuals { property: Iri, individual: Iri },
    /// Compute the shortest path between two individuals.
    ShortestPath { start: Iri, end: Iri },
}

/// Canonical representation of reasoning outcomes attached to assistant
/// invocations.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReasoningOutcome {
    /// Result of [`ReasoningCommand::Ancestors`].
    Ancestors { class: Iri, ancestors: Vec<Iri> },
    /// Result of [`ReasoningCommand::Descendants`].
    Descendants { class: Iri, descendants: Vec<Iri> },
    /// Result of [`ReasoningCommand::RelatedIndividuals`].
    RelatedIndividuals {
        property: Iri,
        individual: Iri,
        related: Vec<Iri>,
    },
    /// Result of [`ReasoningCommand::ShortestPath`].
    ShortestPath {
        start: Iri,
        end: Iri,
        path: Option<Vec<Iri>>,
    },
}

impl ReasoningOutcome {
    /// Converts the outcome into a human readable string.
    #[must_use]
    pub fn describe(&self) -> String {
        match self {
            Self::Ancestors { class, ancestors } => {
                let mut text = format!("Ancestors of class `{class}` ({} items):", ancestors.len());
                for iri in ancestors {
                    text.push_str("\n  - ");
                    text.push_str(iri.as_str());
                }
                text
            }
            Self::Descendants { class, descendants } => {
                let mut text = format!(
                    "Descendants of class `{class}` ({} items):",
                    descendants.len()
                );
                for iri in descendants {
                    text.push_str("\n  - ");
                    text.push_str(iri.as_str());
                }
                text
            }
            Self::RelatedIndividuals {
                property,
                individual,
                related,
            } => {
                let mut text = format!(
                    "Individuals related to `{individual}` via `{property}` ({} items):",
                    related.len()
                );
                for iri in related {
                    text.push_str("\n  - ");
                    text.push_str(iri.as_str());
                }
                text
            }
            Self::ShortestPath { start, end, path } => match path {
                Some(hops) => {
                    let mut text = format!(
                        "Shortest path between `{start}` and `{end}` ({} hops):",
                        hops.len()
                    );
                    for iri in hops {
                        text.push_str("\n  - ");
                        text.push_str(iri.as_str());
                    }
                    text
                }
                None => format!("No path discovered between `{start}` and `{end}`."),
            },
        }
    }
}

/// Contract implemented by AI providers capable of synthesizing ontology
/// reasoning results with natural language responses.
#[async_trait]
pub trait KnowledgeAssistant: Send + Sync {
    /// Produces a combined response from the supplied request.
    async fn respond(
        &self,
        request: KnowledgeRequest,
    ) -> Result<KnowledgeResponse, KnowledgeAssistantError>;
}

/// Factory error raised when building assistant adapters from configuration.
#[derive(Debug, Error)]
pub enum KnowledgeAssistantInitError {
    /// Configuration referenced an unknown backend.
    #[error("knowledge assistant backend is not configured")]
    MissingBackend,
    /// Provided configuration was invalid.
    #[error("invalid knowledge assistant configuration: {0}")]
    InvalidConfiguration(String),
    /// Adapter construction failed.
    #[error("failed to construct assistant adapter: {0}")]
    Adapter(String),
}

/// Errors surfaced by assistant adapters.
#[derive(Debug, Error)]
pub enum KnowledgeAssistantError {
    /// Building the provider request failed.
    #[error("failed to compose provider request: {0}")]
    Request(String),
    /// Provider returned an invalid response.
    #[error("provider returned an unexpected response")]
    EmptyResponse,
    /// Provider interaction failed.
    #[error("provider error: {0}")]
    Provider(String),
}

/// Orchestrates reasoning commands before delegating to an assistant.
pub struct KnowledgeOrchestrator {
    reasoner: Arc<ReasonerHandle>,
    assistant: Arc<dyn KnowledgeAssistant>,
}

impl KnowledgeOrchestrator {
    /// Creates a new orchestrator from its dependencies.
    pub fn new(reasoner: Arc<ReasonerHandle>, assistant: Arc<dyn KnowledgeAssistant>) -> Self {
        Self {
            reasoner,
            assistant,
        }
    }

    /// Executes the supplied reasoning plan before invoking the assistant.
    pub async fn run(
        &self,
        ontology: Iri,
        prompt: String,
        plan: Vec<ReasoningCommand>,
    ) -> Result<KnowledgeSynthesis, KnowledgeOrchestratorError> {
        let mut inferences = Vec::with_capacity(plan.len());
        for command in plan {
            match command {
                ReasoningCommand::Ancestors { class } => {
                    let ancestors = self.reasoner.ancestors_of(&ontology, &class).await?;
                    inferences.push(ReasoningOutcome::Ancestors { class, ancestors });
                }
                ReasoningCommand::Descendants { class } => {
                    let descendants = self.reasoner.descendants_of(&ontology, &class).await?;
                    inferences.push(ReasoningOutcome::Descendants { class, descendants });
                }
                ReasoningCommand::RelatedIndividuals {
                    property,
                    individual,
                } => {
                    let related = self
                        .reasoner
                        .related_individuals(&ontology, &property, &individual)
                        .await?;
                    inferences.push(ReasoningOutcome::RelatedIndividuals {
                        property,
                        individual,
                        related,
                    });
                }
                ReasoningCommand::ShortestPath { start, end } => {
                    let path = self.reasoner.shortest_path(&ontology, &start, &end).await?;
                    inferences.push(ReasoningOutcome::ShortestPath { start, end, path });
                }
            }
        }

        let request = KnowledgeRequest {
            prompt,
            ontology: ontology.clone(),
            inferences: inferences.clone(),
        };
        let response = self.assistant.respond(request).await?;
        Ok(KnowledgeSynthesis {
            message: response.message,
            inferences,
        })
    }
}

/// Errors produced while orchestrating knowledge assistant calls.
#[derive(Debug, Error)]
pub enum KnowledgeOrchestratorError {
    /// Reasoner returned an error while executing the plan.
    #[error(transparent)]
    Reasoner(#[from] OntologyServiceError),
    /// Assistant invocation failed.
    #[error(transparent)]
    Assistant(#[from] KnowledgeAssistantError),
}

/// Builds a knowledge assistant adapter from configuration.
pub fn build_assistant(
    settings: &AiSettings,
) -> Result<Option<Arc<dyn KnowledgeAssistant>>, KnowledgeAssistantInitError> {
    let Some(backend) = settings.assistant.as_ref() else {
        return Ok(None);
    };

    match backend {
        KnowledgeAssistantBackend::OpenAi(cfg) => {
            let adapter = infrastructure::openai::OpenAiKnowledgeAssistant::try_new(cfg)?;
            Ok(Some(Arc::new(adapter)))
        }
    }
}

#[cfg(test)]
pub(crate) mod test_support {
    use super::*;
    use async_trait::async_trait;
    use std::sync::Mutex;

    /// Mock reasoner storing the list of executed operations.
    #[derive(Default)]
    pub struct MockReasoner {
        pub calls: Mutex<Vec<String>>,
        pub ancestors: Vec<Iri>,
        pub descendants: Vec<Iri>,
        pub related: Vec<Iri>,
        pub path: Option<Vec<Iri>>,
    }

    #[async_trait]
    impl crate::ontology::repositories::ReasoningQuery for MockReasoner {
        type Error = OntologyServiceError;

        async fn ancestors_of(
            &self,
            _ontology: &Iri,
            class: &Iri,
        ) -> Result<Vec<Iri>, Self::Error> {
            self.calls
                .lock()
                .unwrap()
                .push(format!("ancestors:{class}"));
            Ok(self.ancestors.clone())
        }

        async fn descendants_of(
            &self,
            _ontology: &Iri,
            class: &Iri,
        ) -> Result<Vec<Iri>, Self::Error> {
            self.calls
                .lock()
                .unwrap()
                .push(format!("descendants:{class}"));
            Ok(self.descendants.clone())
        }

        async fn related_individuals(
            &self,
            _ontology: &Iri,
            property: &Iri,
            individual: &Iri,
        ) -> Result<Vec<Iri>, Self::Error> {
            self.calls
                .lock()
                .unwrap()
                .push(format!("related:{individual}:{property}"));
            Ok(self.related.clone())
        }

        async fn shortest_path(
            &self,
            _ontology: &Iri,
            start: &Iri,
            end: &Iri,
        ) -> Result<Option<Vec<Iri>>, Self::Error> {
            self.calls
                .lock()
                .unwrap()
                .push(format!("path:{start}:{end}"));
            Ok(self.path.clone())
        }
    }

    /// Mock assistant capturing the last request.
    pub struct MockAssistant {
        pub last_request: Mutex<Option<KnowledgeRequest>>,
        pub response: KnowledgeResponse,
    }

    impl Default for MockAssistant {
        fn default() -> Self {
            Self {
                last_request: Mutex::new(None),
                response: KnowledgeResponse {
                    message: String::new(),
                },
            }
        }
    }

    #[async_trait]
    impl KnowledgeAssistant for MockAssistant {
        async fn respond(
            &self,
            request: KnowledgeRequest,
        ) -> Result<KnowledgeResponse, KnowledgeAssistantError> {
            *self.last_request.lock().unwrap() = Some(request);
            Ok(self.response.clone())
        }
    }

    #[tokio::test]
    async fn orchestrator_executes_reasoning_plan() {
        let reasoner = Arc::new(MockReasoner {
            ancestors: vec![Iri::new("https://example.org/Parent").unwrap()],
            descendants: vec![],
            related: vec![],
            path: Some(vec![Iri::new("https://example.org/path").unwrap()]),
            ..MockReasoner::default()
        });
        let assistant = Arc::new(MockAssistant {
            response: KnowledgeResponse {
                message: "ack".to_string(),
            },
            ..MockAssistant::default()
        });
        let orchestrator = KnowledgeOrchestrator::new(reasoner.clone(), assistant.clone());
        let ontology = Iri::new("https://example.org/ontology").unwrap();
        let class = Iri::new("https://example.org/Class").unwrap();
        let synthesis = orchestrator
            .run(
                ontology.clone(),
                "Explain the hierarchy".to_string(),
                vec![
                    ReasoningCommand::Ancestors {
                        class: class.clone(),
                    },
                    ReasoningCommand::ShortestPath {
                        start: class.clone(),
                        end: Iri::new("https://example.org/Other").unwrap(),
                    },
                ],
            )
            .await
            .expect("orchestrator to succeed");

        assert_eq!(synthesis.message, "ack");
        assert_eq!(synthesis.inferences.len(), 2);
        let calls = reasoner.calls.lock().unwrap();
        assert_eq!(
            calls.as_slice(),
            [
                format!("ancestors:{class}"),
                format!(
                    "path:{}:{}",
                    class,
                    Iri::new("https://example.org/Other").unwrap()
                ),
            ]
        );
        let recorded = assistant
            .last_request
            .lock()
            .unwrap()
            .as_ref()
            .cloned()
            .expect("request to be recorded");
        assert_eq!(recorded.prompt, "Explain the hierarchy");
        assert_eq!(recorded.ontology, ontology);
        assert_eq!(recorded.inferences.len(), 2);
    }
}
