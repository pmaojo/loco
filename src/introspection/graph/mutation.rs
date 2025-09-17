#![cfg(debug_assertions)]

use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::Error;

/// Field metadata describing model and scaffold attributes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FieldDefinition {
    pub name: String,
    pub data_type: String,
}

impl FieldDefinition {
    #[must_use]
    pub fn new(name: impl Into<String>, data_type: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            data_type: data_type.into(),
        }
    }

    #[must_use]
    pub fn into_pair(self) -> (String, String) {
        (self.name, self.data_type)
    }

    #[must_use]
    pub fn as_pair(&self) -> (String, String) {
        (self.name.clone(), self.data_type.clone())
    }
}

/// Presentation style for generated scaffold components.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodePresentation {
    Htmx,
    Html,
    Api,
}

/// User-facing request describing the desired node to create.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "component", rename_all = "snake_case")]
pub enum NodeCreationRequest {
    #[cfg(feature = "with-db")]
    Model {
        name: String,
        #[serde(default)]
        with_timestamps: Option<bool>,
        #[serde(default)]
        fields: Vec<FieldDefinition>,
    },
    #[cfg(feature = "with-db")]
    Migration {
        name: String,
        #[serde(default)]
        with_timestamps: Option<bool>,
        #[serde(default)]
        fields: Vec<FieldDefinition>,
    },
    #[cfg(feature = "with-db")]
    Scaffold {
        name: String,
        #[serde(default)]
        with_timestamps: Option<bool>,
        #[serde(default)]
        fields: Vec<FieldDefinition>,
        interface: NodePresentation,
    },
    Controller {
        name: String,
        #[serde(default)]
        actions: Vec<String>,
        interface: NodePresentation,
    },
    Task {
        name: String,
    },
    Scheduler {},
    Worker {
        name: String,
    },
    Mailer {
        name: String,
    },
    Data {
        name: String,
    },
}

impl NodeCreationRequest {
    fn into_component(self) -> Result<NodeComponent, Error> {
        match self {
            #[cfg(feature = "with-db")]
            Self::Model {
                name,
                with_timestamps,
                fields,
            } => Ok(NodeComponent::Model {
                name,
                with_timestamps: with_timestamps.unwrap_or(true),
                fields,
            }),
            #[cfg(feature = "with-db")]
            Self::Migration {
                name,
                with_timestamps,
                fields,
            } => Ok(NodeComponent::Migration {
                name,
                with_timestamps: with_timestamps.unwrap_or(true),
                fields,
            }),
            #[cfg(feature = "with-db")]
            Self::Scaffold {
                name,
                with_timestamps,
                fields,
                interface,
            } => Ok(NodeComponent::Scaffold {
                name,
                with_timestamps: with_timestamps.unwrap_or(true),
                fields,
                interface,
            }),
            Self::Controller {
                name,
                actions,
                interface,
            } => Ok(NodeComponent::Controller {
                name,
                actions,
                interface,
            }),
            Self::Task { name } => Ok(NodeComponent::Task { name }),
            Self::Scheduler {} => Ok(NodeComponent::Scheduler),
            Self::Worker { name } => Ok(NodeComponent::Worker { name }),
            Self::Mailer { name } => Ok(NodeComponent::Mailer { name }),
            Self::Data { name } => Ok(NodeComponent::Data { name }),
        }
    }
}

/// Normalised command passed to code generators.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeCreationCommand {
    pub app_name: String,
    pub component: NodeComponent,
}

/// Normalised component variants independent of adapters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeComponent {
    #[cfg(feature = "with-db")]
    Model {
        name: String,
        with_timestamps: bool,
        fields: Vec<FieldDefinition>,
    },
    #[cfg(feature = "with-db")]
    Migration {
        name: String,
        with_timestamps: bool,
        fields: Vec<FieldDefinition>,
    },
    #[cfg(feature = "with-db")]
    Scaffold {
        name: String,
        with_timestamps: bool,
        fields: Vec<FieldDefinition>,
        interface: NodePresentation,
    },
    Controller {
        name: String,
        actions: Vec<String>,
        interface: NodePresentation,
    },
    Task {
        name: String,
    },
    Scheduler,
    Worker {
        name: String,
    },
    Mailer {
        name: String,
    },
    Data {
        name: String,
    },
}

/// Result returned by scaffold generators.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ScaffoldGeneration {
    pub message: String,
}

impl ScaffoldGeneration {
    #[must_use]
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

/// Port abstracting scaffold generation.
pub trait ScaffoldGenerator: Send + Sync {
    fn generate(&self, command: NodeCreationCommand) -> crate::Result<ScaffoldGeneration>;
}

impl<T> ScaffoldGenerator for Arc<T>
where
    T: ScaffoldGenerator + ?Sized,
{
    fn generate(&self, command: NodeCreationCommand) -> crate::Result<ScaffoldGeneration> {
        (**self).generate(command)
    }
}

/// Application service translating requests into generator commands.
pub struct GraphMutationService<G>
where
    G: ScaffoldGenerator,
{
    app_name: String,
    generator: G,
}

impl<G> GraphMutationService<G>
where
    G: ScaffoldGenerator,
{
    #[must_use]
    pub fn new(app_name: impl Into<String>, generator: G) -> Self {
        Self {
            app_name: app_name.into(),
            generator,
        }
    }

    pub fn create_node(&self, request: NodeCreationRequest) -> crate::Result<ScaffoldGeneration> {
        let component = request.into_component()?;
        let command = NodeCreationCommand {
            app_name: self.app_name.clone(),
            component,
        };
        self.generator.generate(command)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    #[derive(Default)]
    struct RecordingGenerator {
        commands: Mutex<Vec<NodeCreationCommand>>,
    }

    impl RecordingGenerator {
        fn take(&self) -> Vec<NodeCreationCommand> {
            self.commands.lock().unwrap().clone()
        }
    }

    impl ScaffoldGenerator for RecordingGenerator {
        fn generate(&self, command: NodeCreationCommand) -> crate::Result<ScaffoldGeneration> {
            self.commands.lock().unwrap().push(command);
            Ok(ScaffoldGeneration::new("ok"))
        }
    }

    #[cfg(feature = "with-db")]
    #[test]
    fn defaults_model_timestamps_to_true() {
        let generator = Arc::new(RecordingGenerator::default());
        let service = GraphMutationService::new("demo", Arc::clone(&generator));
        let request = NodeCreationRequest::Model {
            name: "post".into(),
            with_timestamps: None,
            fields: vec![FieldDefinition::new("title", "string")],
        };

        service
            .create_node(request)
            .expect("model generation succeeds");

        let recorded = generator.take();
        assert_eq!(recorded.len(), 1);
        assert!(matches!(
            &recorded[0].component,
            NodeComponent::Model {
                name,
                with_timestamps,
                fields,
            } if name == "post" && *with_timestamps && fields == &vec![FieldDefinition::new("title", "string")]
        ));
    }

    #[test]
    fn propagates_app_name_to_command() {
        let generator = Arc::new(RecordingGenerator::default());
        let service = GraphMutationService::new("my-app", Arc::clone(&generator));
        let request = NodeCreationRequest::Task {
            name: "cleanup".into(),
        };

        service
            .create_node(request)
            .expect("task generation succeeds");

        let recorded = generator.take();
        assert_eq!(recorded.len(), 1);
        assert_eq!(recorded[0].app_name, "my-app");
        assert!(matches!(recorded[0].component, NodeComponent::Task { .. }));
    }
}
