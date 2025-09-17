use async_trait::async_trait;
use tracing::info;

use crate::{
    ai::{KnowledgeOrchestrator, ReasoningCommand},
    app::AppContext,
    ontology::value_objects::Iri,
    task::{Task, TaskInfo, Vars},
    Error, Result,
};

/// Task wiring the ontology reasoner with the configured knowledge assistant.
#[derive(Default)]
pub struct KnowledgeTask;

impl KnowledgeTask {
    fn parse_iri(value: &str, field: &str) -> Result<Iri> {
        Iri::new(value).map_err(|err| Error::Message(format!("invalid {field} IRI: {err}")))
    }

    fn build_plan(vars: &Vars) -> Result<Vec<ReasoningCommand>> {
        let mut plan = Vec::new();

        if let Some(class) = vars.cli.get("class") {
            let iri = Self::parse_iri(class, "class")?;
            plan.push(ReasoningCommand::Ancestors { class: iri.clone() });
            plan.push(ReasoningCommand::Descendants { class: iri });
        }

        if let (Some(property), Some(individual)) =
            (vars.cli.get("property"), vars.cli.get("individual"))
        {
            plan.push(ReasoningCommand::RelatedIndividuals {
                property: Self::parse_iri(property, "property")?,
                individual: Self::parse_iri(individual, "individual")?,
            });
        }

        if let (Some(start), Some(end)) = (vars.cli.get("path_start"), vars.cli.get("path_end")) {
            plan.push(ReasoningCommand::ShortestPath {
                start: Self::parse_iri(start, "path_start")?,
                end: Self::parse_iri(end, "path_end")?,
            });
        }

        Ok(plan)
    }
}

#[async_trait]
impl Task for KnowledgeTask {
    fn task(&self) -> TaskInfo {
        TaskInfo {
            name: "ontology:assist".to_string(),
            detail: "Run a reasoning plan and dispatch it to the configured knowledge assistant"
                .to_string(),
        }
    }

    async fn run(&self, app_context: &AppContext, vars: &Vars) -> Result<()> {
        let prompt = vars.cli_arg("prompt")?.to_string();
        let ontology_id = vars.cli_arg("ontology")?.to_string();
        let ontology = Self::parse_iri(&ontology_id, "ontology")?;

        let assistant = app_context
            .knowledge_assistant
            .as_ref()
            .cloned()
            .ok_or_else(|| {
                Error::Message(
                    "knowledge assistant is not configured. Provide `ai.assistant` in the configuration"
                        .to_string(),
                )
            })?;

        let plan = Self::build_plan(vars)?;
        let reasoner = app_context.ontology.reasoner();
        let orchestrator = KnowledgeOrchestrator::new(reasoner, assistant);
        let synthesis = orchestrator
            .run(ontology, prompt, plan)
            .await
            .map_err(Error::wrap)?;

        info!(message = %synthesis.message, "knowledge_assistant_response");
        for outcome in &synthesis.inferences {
            info!(context = %outcome.describe(), "knowledge_assistant_reasoning");
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::{
        test_support::{MockAssistant, MockReasoner},
        KnowledgeAssistant, KnowledgeResponse,
    };
    use crate::config::ReasonerSettings;
    use crate::ontology::service::OntologyService;
    use crate::ontology::service::{ReasonerHandle, RepositoryHandle};
    use crate::tests_cfg;
    use std::sync::Arc;

    #[derive(Default)]
    struct NullRepository;

    #[async_trait]
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
    async fn task_invokes_orchestrator_with_reasoning_plan() {
        let reasoner = Arc::new(MockReasoner {
            ancestors: vec![Iri::new("https://example.org/root").unwrap()],
            ..MockReasoner::default()
        });
        let repository: Arc<RepositoryHandle> = Arc::new(NullRepository::default());
        let ontology_service = Arc::new(OntologyService::new(
            repository,
            reasoner.clone() as Arc<ReasonerHandle>,
            ReasonerSettings::default(),
        ));

        let assistant = Arc::new(MockAssistant {
            response: KnowledgeResponse {
                message: "ok".to_string(),
            },
            ..MockAssistant::default()
        });
        let assistant_trait: Arc<dyn KnowledgeAssistant> = assistant.clone();

        let mut ctx = tests_cfg::app::get_app_context().await;
        ctx.ontology = ontology_service;
        ctx.knowledge_assistant = Some(assistant_trait);

        let task = KnowledgeTask::default();
        let vars = Vars::from_cli_args(vec![
            ("prompt".into(), "Summarize".into()),
            ("ontology".into(), "https://example.org/ontology".into()),
            ("class".into(), "https://example.org/root".into()),
        ]);

        task.run(&ctx, &vars).await.expect("task to succeed");
        let calls = reasoner.calls.lock().unwrap();
        assert!(calls.iter().any(|call| call.starts_with("ancestors:")));
        let request = assistant
            .last_request
            .lock()
            .unwrap()
            .clone()
            .expect("assistant request");
        assert_eq!(request.inferences.len(), 2); // ancestors + descendants
    }
}
