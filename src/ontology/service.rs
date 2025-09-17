use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use async_trait::async_trait;

use crate::{
    config::{OntologyBackend, OntologySettings, ReasonerBackend, ReasonerSettings},
    ontology::{
        entities::{Class, Individual, Ontology, OntologyError, Property, PropertyAssertion},
        repositories::{OntologyRepository, OntologySnapshot, OntologySummary, ReasoningQuery},
        value_objects::Iri,
    },
};

/// Type alias simplifying repository trait object usage inside the service.
pub type RepositoryHandle =
    dyn OntologyRepository<Error = OntologyServiceError> + Send + Sync + 'static;
/// Type alias simplifying reasoner trait object usage inside the service.
pub type ReasonerHandle = dyn ReasoningQuery<Error = OntologyServiceError> + Send + Sync + 'static;

/// High level ontology service wiring repository and reasoner adapters together.
#[derive(Clone)]
pub struct OntologyService {
    repository: Arc<RepositoryHandle>,
    reasoner: Arc<ReasonerHandle>,
    reasoner_settings: ReasonerSettings,
}

impl OntologyService {
    /// Creates a new [`OntologyService`] from trait object handles.
    pub fn new(
        repository: Arc<RepositoryHandle>,
        reasoner: Arc<ReasonerHandle>,
        reasoner_settings: ReasonerSettings,
    ) -> Self {
        Self {
            repository,
            reasoner,
            reasoner_settings,
        }
    }

    /// Builds a service instance from configuration settings.
    pub fn from_config(
        ontology: &OntologySettings,
        reasoner: &ReasonerSettings,
    ) -> Result<Self, OntologyServiceError> {
        let store = match ontology.backend {
            OntologyBackend::InMemory => Arc::new(InMemoryStore::default()),
        };
        let repository = Arc::new(InMemoryOntologyRepository::new(store.clone()));
        repository.preload(&ontology.seeds)?;

        let reasoner_adapter: Arc<ReasonerHandle> = match reasoner.backend {
            ReasonerBackend::Native => Arc::new(InMemoryReasoner::new(store, reasoner.clone())),
        };

        Ok(Self::new(repository, reasoner_adapter, reasoner.clone()))
    }

    /// Returns a clone of the repository handle.
    pub fn repository(&self) -> Arc<RepositoryHandle> {
        Arc::clone(&self.repository)
    }

    /// Returns a clone of the reasoner handle.
    pub fn reasoner(&self) -> Arc<ReasonerHandle> {
        Arc::clone(&self.reasoner)
    }

    /// Returns the active reasoner settings.
    pub fn reasoner_settings(&self) -> &ReasonerSettings {
        &self.reasoner_settings
    }
}

/// Errors raised by ontology infrastructure components.
#[derive(Debug, thiserror::Error)]
pub enum OntologyServiceError {
    /// Attempted to create an ontology that already exists.
    #[error("ontology `{ontology}` already exists")]
    Duplicate { ontology: Iri },
    /// Referenced ontology was not found.
    #[error("ontology `{ontology}` missing")]
    Missing { ontology: Iri },
    /// Referenced class was not found in the ontology.
    #[error("class `{class}` missing in ontology `{ontology}`")]
    MissingClass { ontology: Iri, class: Iri },
    /// Referenced property was not found in the ontology.
    #[error("property `{property}` missing in ontology `{ontology}`")]
    MissingProperty { ontology: Iri, property: Iri },
    /// Referenced individual was not found in the ontology.
    #[error("individual `{individual}` missing in ontology `{ontology}`")]
    MissingIndividual { ontology: Iri, individual: Iri },
    /// Domain validation failed when mutating the ontology aggregate.
    #[error("domain error: {0}")]
    Domain(#[from] OntologyError),
    /// Accessing a configured ontology seed path failed.
    #[error("failed to access ontology seed `{path}`: {source}")]
    SeedIo {
        path: PathBuf,
        source: std::io::Error,
    },
}

impl OntologyServiceError {
    fn duplicate(ontology: &Iri) -> Self {
        Self::Duplicate {
            ontology: ontology.clone(),
        }
    }

    fn missing(ontology: &Iri) -> Self {
        Self::Missing {
            ontology: ontology.clone(),
        }
    }

    fn missing_class(ontology: &Iri, class: &Iri) -> Self {
        Self::MissingClass {
            ontology: ontology.clone(),
            class: class.clone(),
        }
    }

    fn missing_property(ontology: &Iri, property: &Iri) -> Self {
        Self::MissingProperty {
            ontology: ontology.clone(),
            property: property.clone(),
        }
    }

    fn missing_individual(ontology: &Iri, individual: &Iri) -> Self {
        Self::MissingIndividual {
            ontology: ontology.clone(),
            individual: individual.clone(),
        }
    }
}

#[derive(Default)]
struct InMemoryStore {
    ontologies: Mutex<BTreeMap<Iri, Ontology>>,
}

impl InMemoryStore {
    fn guard(&self) -> std::sync::MutexGuard<'_, BTreeMap<Iri, Ontology>> {
        self.ontologies
            .lock()
            .expect("in-memory ontology store poisoned")
    }
}

#[derive(Clone)]
struct InMemoryOntologyRepository {
    store: Arc<InMemoryStore>,
}

impl InMemoryOntologyRepository {
    fn new(store: Arc<InMemoryStore>) -> Self {
        Self { store }
    }

    fn preload(&self, seeds: &[PathBuf]) -> Result<(), OntologyServiceError> {
        for path in seeds {
            validate_seed_path(path)?;
        }
        Ok(())
    }
}

fn validate_seed_path(path: &Path) -> Result<(), OntologyServiceError> {
    if path.exists() {
        if path.is_file() || path.is_dir() {
            Ok(())
        } else {
            Err(OntologyServiceError::SeedIo {
                path: path.to_path_buf(),
                source: std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "unsupported seed path type",
                ),
            })
        }
    } else {
        fs::metadata(path)
            .map(|_| ())
            .map_err(|source| OntologyServiceError::SeedIo {
                path: path.to_path_buf(),
                source,
            })
    }
}

#[async_trait]
impl OntologyRepository for InMemoryOntologyRepository {
    type Error = OntologyServiceError;

    async fn insert(&self, ontology: Ontology) -> Result<(), Self::Error> {
        let mut guard = self.store.guard();
        let id = ontology.id().clone();
        if guard.contains_key(&id) {
            return Err(OntologyServiceError::duplicate(&id));
        }
        guard.insert(id, ontology);
        Ok(())
    }

    async fn update(&self, ontology: Ontology) -> Result<(), Self::Error> {
        let mut guard = self.store.guard();
        let id = ontology.id().clone();
        if !guard.contains_key(&id) {
            return Err(OntologyServiceError::missing(&id));
        }
        guard.insert(id, ontology);
        Ok(())
    }

    async fn get(&self, iri: &Iri) -> Result<Option<OntologySnapshot>, Self::Error> {
        let guard = self.store.guard();
        Ok(guard
            .get(iri)
            .cloned()
            .map(|ontology| OntologySnapshot { ontology }))
    }

    async fn delete(&self, iri: &Iri) -> Result<(), Self::Error> {
        let mut guard = self.store.guard();
        guard
            .remove(iri)
            .map(|_| ())
            .ok_or_else(|| OntologyServiceError::missing(iri))
    }

    async fn list(&self) -> Result<Vec<OntologySummary>, Self::Error> {
        let guard = self.store.guard();
        Ok(guard.values().map(OntologySummary::from).collect())
    }

    async fn attach_class(&self, ontology: &Iri, class: Class) -> Result<(), Self::Error> {
        let mut guard = self.store.guard();
        let Some(existing) = guard.get_mut(ontology) else {
            return Err(OntologyServiceError::missing(ontology));
        };
        existing.add_class(class)?;
        Ok(())
    }

    async fn attach_property(&self, ontology: &Iri, property: Property) -> Result<(), Self::Error> {
        let mut guard = self.store.guard();
        let Some(existing) = guard.get_mut(ontology) else {
            return Err(OntologyServiceError::missing(ontology));
        };
        existing.add_property(property)?;
        Ok(())
    }

    async fn attach_individual(
        &self,
        ontology: &Iri,
        individual: Individual,
    ) -> Result<(), Self::Error> {
        let mut guard = self.store.guard();
        let Some(existing) = guard.get_mut(ontology) else {
            return Err(OntologyServiceError::missing(ontology));
        };
        existing.add_individual(individual)?;
        Ok(())
    }
}

#[derive(Clone)]
struct InMemoryReasoner {
    store: Arc<InMemoryStore>,
    settings: ReasonerSettings,
}

impl InMemoryReasoner {
    fn new(store: Arc<InMemoryStore>, settings: ReasonerSettings) -> Self {
        Self { store, settings }
    }
}

#[async_trait]
impl ReasoningQuery for InMemoryReasoner {
    type Error = OntologyServiceError;

    async fn ancestors_of(&self, ontology: &Iri, class: &Iri) -> Result<Vec<Iri>, Self::Error> {
        if !self.settings.inference.class_hierarchy {
            return Ok(vec![]);
        }
        let guard = self.store.guard();
        let Some(ontology) = guard.get(ontology) else {
            return Err(OntologyServiceError::missing(ontology));
        };
        let Some(start) = ontology.class(class) else {
            return Err(OntologyServiceError::missing_class(ontology.id(), class));
        };

        let mut visited = BTreeSet::new();
        let mut to_visit: VecDeque<Iri> = start.parents().iter().cloned().collect();
        let mut result = Vec::new();

        while let Some(current) = to_visit.pop_front() {
            if visited.insert(current.clone()) {
                result.push(current.clone());
                if let Some(parent) = ontology.class(&current) {
                    to_visit.extend(parent.parents().iter().cloned());
                }
            }
        }

        Ok(result)
    }

    async fn descendants_of(&self, ontology: &Iri, class: &Iri) -> Result<Vec<Iri>, Self::Error> {
        if !self.settings.inference.class_hierarchy {
            return Ok(vec![]);
        }
        let guard = self.store.guard();
        let Some(ontology) = guard.get(ontology) else {
            return Err(OntologyServiceError::missing(ontology));
        };
        if ontology.class(class).is_none() {
            return Err(OntologyServiceError::missing_class(ontology.id(), class));
        }

        let mut result = Vec::new();
        for (id, candidate) in ontology.classes() {
            if candidate.parents().contains(class) {
                result.push(id.clone());
            }
        }

        Ok(result)
    }

    async fn related_individuals(
        &self,
        ontology: &Iri,
        via_property: &Iri,
        individual: &Iri,
    ) -> Result<Vec<Iri>, Self::Error> {
        if !self.settings.inference.property_assertions {
            return Ok(vec![]);
        }
        let guard = self.store.guard();
        let Some(ontology) = guard.get(ontology) else {
            return Err(OntologyServiceError::missing(ontology));
        };
        if ontology.property(via_property).is_none() {
            return Err(OntologyServiceError::missing_property(
                ontology.id(),
                via_property,
            ));
        }
        let Some(individual) = ontology.individual(individual) else {
            return Err(OntologyServiceError::missing_individual(
                ontology.id(),
                individual,
            ));
        };

        let related = individual
            .properties()
            .get(via_property)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter_map(|assertion| match assertion {
                PropertyAssertion::Individual(target) => Some(target),
                PropertyAssertion::Literal(_) => None,
            })
            .collect();

        Ok(related)
    }

    async fn shortest_path(
        &self,
        ontology: &Iri,
        start: &Iri,
        end: &Iri,
    ) -> Result<Option<Vec<Iri>>, Self::Error> {
        if !self.settings.inference.property_paths {
            return Ok(None);
        }
        let guard = self.store.guard();
        let Some(ontology) = guard.get(ontology) else {
            return Err(OntologyServiceError::missing(ontology));
        };
        let Some(source) = ontology.individual(start) else {
            return Err(OntologyServiceError::missing_individual(
                ontology.id(),
                start,
            ));
        };
        if ontology.individual(end).is_none() {
            return Err(OntologyServiceError::missing_individual(ontology.id(), end));
        }

        let mut visited = BTreeSet::from([source.id().clone()]);
        let mut queue: VecDeque<(Iri, Vec<Iri>)> =
            VecDeque::from([(source.id().clone(), vec![source.id().clone()])]);

        while let Some((current, path)) = queue.pop_front() {
            if current == *end {
                return Ok(Some(path));
            }

            if let Some(individual) = ontology.individual(&current) {
                for (property_id, assertions) in individual.properties() {
                    if !self.settings.inference.property_assertions {
                        continue;
                    }
                    if let Some(property) = ontology.property(property_id) {
                        if !matches!(property.kind(), super::entities::PropertyKind::Object) {
                            continue;
                        }
                    }

                    for assertion in assertions {
                        if let PropertyAssertion::Individual(next) = assertion {
                            if visited.insert(next.clone()) {
                                let mut next_path = path.clone();
                                next_path.push(next.clone());
                                queue.push_back((next.clone(), next_path));
                            }
                        }
                    }
                }
            }
        }

        Ok(None)
    }
}
