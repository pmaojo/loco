use axum::{extract::State, routing::post};
use serde::{Deserialize, Serialize};

use crate::{
    ai::{KnowledgeOrchestrator, ReasoningCommand},
    app::AppContext,
    controller::{format, Json, Routes},
    ontology::value_objects::Iri,
    Error, Result,
};

#[derive(Deserialize)]
pub struct KnowledgePrompt {
    pub ontology: String,
    pub prompt: String,
    #[serde(default)]
    pub reasoning: Vec<ReasoningStep>,
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum ReasoningStep {
    Ancestors {
        class: String,
    },
    Descendants {
        class: String,
    },
    RelatedIndividuals {
        property: String,
        individual: String,
    },
    ShortestPath {
        start: String,
        end: String,
    },
}

#[derive(Serialize)]
pub struct KnowledgeResponseBody {
    pub message: String,
    pub reasoning: Vec<ReasoningOutcomeView>,
}

#[derive(Serialize)]
pub struct ReasoningOutcomeView {
    pub kind: String,
    pub summary: String,
}

impl From<&crate::ai::ReasoningOutcome> for ReasoningOutcomeView {
    fn from(value: &crate::ai::ReasoningOutcome) -> Self {
        let kind = match value {
            crate::ai::ReasoningOutcome::Ancestors { .. } => "ancestors",
            crate::ai::ReasoningOutcome::Descendants { .. } => "descendants",
            crate::ai::ReasoningOutcome::RelatedIndividuals { .. } => "related-individuals",
            crate::ai::ReasoningOutcome::ShortestPath { .. } => "shortest-path",
        }
        .to_string();

        Self {
            kind,
            summary: value.describe(),
        }
    }
}

impl KnowledgeResponseBody {
    fn from_synthesis(synthesis: crate::ai::KnowledgeSynthesis) -> Self {
        let reasoning = synthesis
            .inferences
            .iter()
            .map(ReasoningOutcomeView::from)
            .collect();
        Self {
            message: synthesis.message,
            reasoning,
        }
    }
}

fn parse_iri(value: &str, field: &str) -> Result<Iri> {
    Iri::new(value).map_err(|err| Error::BadRequest(format!("invalid {field} IRI: {err}")))
}

fn build_plan(steps: &[ReasoningStep]) -> Result<Vec<ReasoningCommand>> {
    let mut plan = Vec::with_capacity(steps.len());
    for step in steps {
        match step {
            ReasoningStep::Ancestors { class } => {
                plan.push(ReasoningCommand::Ancestors {
                    class: parse_iri(class, "class")?,
                });
            }
            ReasoningStep::Descendants { class } => {
                plan.push(ReasoningCommand::Descendants {
                    class: parse_iri(class, "class")?,
                });
            }
            ReasoningStep::RelatedIndividuals {
                property,
                individual,
            } => {
                plan.push(ReasoningCommand::RelatedIndividuals {
                    property: parse_iri(property, "property")?,
                    individual: parse_iri(individual, "individual")?,
                });
            }
            ReasoningStep::ShortestPath { start, end } => {
                plan.push(ReasoningCommand::ShortestPath {
                    start: parse_iri(start, "start")?,
                    end: parse_iri(end, "end")?,
                });
            }
        }
    }
    Ok(plan)
}

pub async fn invoke(
    State(ctx): State<AppContext>,
    Json(payload): Json<KnowledgePrompt>,
) -> Result<axum::response::Response> {
    let assistant =
        ctx.knowledge_assistant.as_ref().cloned().ok_or_else(|| {
            Error::BadRequest("knowledge assistant is not configured".to_string())
        })?;

    let ontology = parse_iri(&payload.ontology, "ontology")?;
    let plan = build_plan(&payload.reasoning)?;
    let reasoner = ctx.ontology.reasoner();
    let orchestrator = KnowledgeOrchestrator::new(reasoner, assistant);
    let synthesis = orchestrator
        .run(ontology, payload.prompt, plan)
        .await
        .map_err(Error::wrap)?;

    format::json(KnowledgeResponseBody::from_synthesis(synthesis))
}

pub fn routes() -> Routes {
    Routes::new().add("/ai/knowledge", post(invoke))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;
    use axum::response::IntoResponse;
    use axum::{body, extract::State};
    use serde_json::json;

    use crate::ai::{
        test_support::{MockAssistant, MockReasoner},
        KnowledgeAssistant,
    };
    use crate::config::ReasonerSettings;
    use crate::ontology::service::{OntologyService, ReasonerHandle, RepositoryHandle};
    use crate::tests_cfg;

    #[derive(Default)]
    struct NullRepository;

    #[async_trait::async_trait]
    impl crate::ontology::repositories::OntologyRepository for NullRepository {
        type Error = crate::ontology::service::OntologyServiceError;

        async fn insert(
            &self,
            _ontology: crate::ontology::entities::Ontology,
        ) -> Result<(), Self::Error> {
            Ok(())
        }

        async fn update(
            &self,
            _ontology: crate::ontology::entities::Ontology,
        ) -> Result<(), Self::Error> {
            Ok(())
        }

        async fn get(
            &self,
            _iri: &Iri,
        ) -> Result<Option<crate::ontology::repositories::OntologySnapshot>, Self::Error> {
            Ok(None)
        }

        async fn delete(&self, _iri: &Iri) -> Result<(), Self::Error> {
            Ok(())
        }

        async fn list(
            &self,
        ) -> Result<Vec<crate::ontology::repositories::OntologySummary>, Self::Error> {
            Ok(vec![])
        }

        async fn attach_class(
            &self,
            _ontology: &Iri,
            _class: crate::ontology::entities::Class,
        ) -> Result<(), Self::Error> {
            Ok(())
        }

        async fn attach_property(
            &self,
            _ontology: &Iri,
            _property: crate::ontology::entities::Property,
        ) -> Result<(), Self::Error> {
            Ok(())
        }

        async fn attach_individual(
            &self,
            _ontology: &Iri,
            _individual: crate::ontology::entities::Individual,
        ) -> Result<(), Self::Error> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn controller_invokes_assistant() {
        let reasoner = std::sync::Arc::new(MockReasoner {
            ancestors: vec![Iri::new("https://example.org/parent").unwrap()],
            ..MockReasoner::default()
        });
        let repository: std::sync::Arc<RepositoryHandle> =
            std::sync::Arc::new(NullRepository::default());
        let ontology_service = std::sync::Arc::new(OntologyService::new(
            repository,
            reasoner.clone() as std::sync::Arc<ReasonerHandle>,
            ReasonerSettings::default(),
        ));

        let assistant = std::sync::Arc::new(MockAssistant {
            response: crate::ai::KnowledgeResponse {
                message: "response".to_string(),
            },
            ..MockAssistant::default()
        });
        let assistant_trait: std::sync::Arc<dyn KnowledgeAssistant> = assistant.clone();

        let mut ctx = tests_cfg::app::get_app_context().await;
        ctx.ontology = ontology_service;
        ctx.knowledge_assistant = Some(assistant_trait);

        let body = Json(KnowledgePrompt {
            ontology: "https://example.org/ontology".to_string(),
            prompt: "Explain".to_string(),
            reasoning: vec![ReasoningStep::Ancestors {
                class: "https://example.org/child".to_string(),
            }],
        });

        let response = invoke(State(ctx), body)
            .await
            .expect("controller success")
            .into_response();

        assert_eq!(response.status(), StatusCode::OK);
        let bytes = body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(value["message"], json!("response"));
        assert_eq!(value["reasoning"].as_array().unwrap().len(), 1);
        let calls = reasoner.calls.lock().unwrap();
        assert_eq!(calls.len(), 1);
        assert!(assistant.last_request.lock().unwrap().is_some());
    }
}
