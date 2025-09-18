use async_trait::async_trait;

use super::entities::{Class, Individual, Ontology, Property};
use super::value_objects::Iri;

/// Lightweight snapshot returned by repository lookups.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OntologySnapshot {
    /// Full ontology aggregate.
    pub ontology: Ontology,
}

impl From<Ontology> for OntologySnapshot {
    fn from(ontology: Ontology) -> Self {
        Self { ontology }
    }
}

/// Summary DTO for listing ontologies without loading the full aggregate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OntologySummary {
    /// Identifier of the ontology.
    pub iri: Iri,
    /// Optional label for display purposes.
    pub label: Option<String>,
    /// Number of class declarations.
    pub class_count: usize,
    /// Number of property declarations.
    pub property_count: usize,
    /// Number of individuals.
    pub individual_count: usize,
}

impl From<&Ontology> for OntologySummary {
    fn from(ontology: &Ontology) -> Self {
        Self {
            iri: ontology.id().clone(),
            label: ontology.label().map(|label| label.to_string()),
            class_count: ontology.classes().len(),
            property_count: ontology.properties().len(),
            individual_count: ontology.individuals().len(),
        }
    }
}

/// Contract describing persistence responsibilities for ontology aggregates.
#[async_trait]
pub trait OntologyRepository {
    /// Associated error type allowing infrastructure specific failures.
    type Error;

    /// Persists a brand new ontology.
    ///
    /// Implementors are expected to reject duplicate IRIs and to persist the
    /// ontology atomically.
    async fn insert(&self, ontology: Ontology) -> Result<(), Self::Error>;

    /// Updates an existing ontology aggregate.
    ///
    /// Implementors should replace the stored aggregate while ensuring
    /// concurrent updates are properly serialized.
    async fn update(&self, ontology: Ontology) -> Result<(), Self::Error>;

    /// Retrieves a stored ontology by identifier.
    ///
    /// Implementors must return `Ok(None)` when the ontology is missing.
    async fn get(&self, iri: &Iri) -> Result<Option<OntologySnapshot>, Self::Error>;

    /// Deletes an ontology and all nested resources.
    async fn delete(&self, iri: &Iri) -> Result<(), Self::Error>;

    /// Lists all ontologies without loading the entire aggregate.
    async fn list(&self) -> Result<Vec<OntologySummary>, Self::Error>;

    /// Appends a class declaration to an existing ontology.
    ///
    /// The provided [`Class`] should be validated using the domain aggregate to
    /// guarantee referential integrity.
    async fn attach_class(&self, ontology: &Iri, class: Class) -> Result<(), Self::Error>;

    /// Appends a property declaration to an existing ontology.
    async fn attach_property(&self, ontology: &Iri, property: Property) -> Result<(), Self::Error>;

    /// Appends an individual declaration to an existing ontology.
    async fn attach_individual(
        &self,
        ontology: &Iri,
        individual: Individual,
    ) -> Result<(), Self::Error>;
}

/// Abstraction describing reasoning and traversal operations on ontology graphs.
#[async_trait]
pub trait ReasoningQuery {
    /// Associated error type allowing infrastructure specific failures.
    type Error;

    /// Returns the transitive closure of all parent classes for a given class.
    async fn ancestors_of(&self, ontology: &Iri, class: &Iri) -> Result<Vec<Iri>, Self::Error>;

    /// Returns the transitive closure of all child classes for a given class.
    async fn descendants_of(&self, ontology: &Iri, class: &Iri) -> Result<Vec<Iri>, Self::Error>;

    /// Returns individuals connected to the supplied source via the provided property.
    async fn related_individuals(
        &self,
        ontology: &Iri,
        via_property: &Iri,
        individual: &Iri,
    ) -> Result<Vec<Iri>, Self::Error>;

    /// Returns the shortest property path between two individuals, if one exists.
    async fn shortest_path(
        &self,
        ontology: &Iri,
        start: &Iri,
        end: &Iri,
    ) -> Result<Option<Vec<Iri>>, Self::Error>;
}

#[cfg(test)]
mod tests {
    use super::{OntologyRepository, OntologySnapshot, OntologySummary, ReasoningQuery};
    use crate::ontology::entities::{
        Class, Individual, Ontology, Property, PropertyAssertion, PropertyKind,
    };
    use crate::ontology::value_objects::Iri;
    use async_trait::async_trait;
    use std::collections::{BTreeMap, BTreeSet, VecDeque};
    use std::sync::Mutex;

    fn iri(text: &str) -> Iri {
        Iri::new(text).expect("valid iri")
    }

    #[derive(Default)]
    struct InMemoryOntologyRepository {
        store: Mutex<BTreeMap<Iri, Ontology>>,
    }

    #[derive(Debug, thiserror::Error)]
    enum TestError {
        #[error("ontology already exists")]
        Duplicate,
        #[error("ontology missing")]
        Missing,
        #[error("domain error: {0}")]
        Domain(String),
    }

    #[async_trait]
    impl OntologyRepository for InMemoryOntologyRepository {
        type Error = TestError;

        async fn insert(&self, ontology: Ontology) -> Result<(), Self::Error> {
            let mut guard = self.store.lock().unwrap();
            let id = ontology.id().clone();
            if guard.contains_key(&id) {
                return Err(TestError::Duplicate);
            }
            guard.insert(id, ontology);
            Ok(())
        }

        async fn update(&self, ontology: Ontology) -> Result<(), Self::Error> {
            let mut guard = self.store.lock().unwrap();
            let id = ontology.id().clone();
            if !guard.contains_key(&id) {
                return Err(TestError::Missing);
            }
            guard.insert(id, ontology);
            Ok(())
        }

        async fn get(&self, iri: &Iri) -> Result<Option<OntologySnapshot>, Self::Error> {
            let guard = self.store.lock().unwrap();
            Ok(guard
                .get(iri)
                .cloned()
                .map(|ontology| OntologySnapshot { ontology }))
        }

        async fn delete(&self, iri: &Iri) -> Result<(), Self::Error> {
            let mut guard = self.store.lock().unwrap();
            guard
                .remove(iri)
                .map_or(Err(TestError::Missing), |_| Ok(()))
        }

        async fn list(&self) -> Result<Vec<OntologySummary>, Self::Error> {
            let guard = self.store.lock().unwrap();
            Ok(guard.values().map(OntologySummary::from).collect())
        }

        async fn attach_class(&self, ontology: &Iri, class: Class) -> Result<(), Self::Error> {
            let mut guard = self.store.lock().unwrap();
            let Some(existing) = guard.get_mut(ontology) else {
                return Err(TestError::Missing);
            };
            existing
                .add_class(class)
                .map_err(|err| TestError::Domain(err.to_string()))
        }

        async fn attach_property(
            &self,
            ontology: &Iri,
            property: Property,
        ) -> Result<(), Self::Error> {
            let mut guard = self.store.lock().unwrap();
            let Some(existing) = guard.get_mut(ontology) else {
                return Err(TestError::Missing);
            };
            existing
                .add_property(property)
                .map_err(|err| TestError::Domain(err.to_string()))
        }

        async fn attach_individual(
            &self,
            ontology: &Iri,
            individual: Individual,
        ) -> Result<(), Self::Error> {
            let mut guard = self.store.lock().unwrap();
            let Some(existing) = guard.get_mut(ontology) else {
                return Err(TestError::Missing);
            };
            existing
                .add_individual(individual)
                .map_err(|err| TestError::Domain(err.to_string()))
        }
    }

    #[async_trait]
    impl ReasoningQuery for InMemoryOntologyRepository {
        type Error = TestError;

        async fn ancestors_of(&self, ontology: &Iri, class: &Iri) -> Result<Vec<Iri>, Self::Error> {
            let guard = self.store.lock().unwrap();
            let ontology = guard.get(ontology).ok_or(TestError::Missing)?;
            let start = ontology
                .class(class)
                .ok_or(TestError::Domain(format!("class {class} missing")))?;
            let mut visited = BTreeSet::new();
            let mut queue: VecDeque<Iri> = start.parents().iter().cloned().collect();
            while let Some(current) = queue.pop_front() {
                if visited.insert(current.clone()) {
                    if let Some(parent) = ontology.class(&current) {
                        queue.extend(parent.parents().iter().cloned());
                    }
                }
            }
            Ok(visited.into_iter().collect())
        }

        async fn descendants_of(
            &self,
            ontology: &Iri,
            class: &Iri,
        ) -> Result<Vec<Iri>, Self::Error> {
            let guard = self.store.lock().unwrap();
            let ontology = guard.get(ontology).ok_or(TestError::Missing)?;
            if ontology.class(class).is_none() {
                return Err(TestError::Domain(format!("class {class} missing")));
            }
            let mut visited = BTreeSet::new();
            let mut queue: VecDeque<Iri> = VecDeque::from([class.clone()]);
            while let Some(current) = queue.pop_front() {
                for (id, candidate) in ontology.classes() {
                    if candidate.parents().contains(&current) && visited.insert(id.clone()) {
                        queue.push_back(id.clone());
                    }
                }
            }
            Ok(visited.into_iter().collect())
        }

        async fn related_individuals(
            &self,
            ontology: &Iri,
            via_property: &Iri,
            individual: &Iri,
        ) -> Result<Vec<Iri>, Self::Error> {
            let guard = self.store.lock().unwrap();
            let ontology = guard.get(ontology).ok_or(TestError::Missing)?;
            let source = ontology
                .individual(individual)
                .ok_or_else(|| TestError::Domain(format!("individual {individual} missing")))?;
            let Some(property) = ontology.property(via_property) else {
                return Err(TestError::Domain(format!(
                    "property {via_property} missing"
                )));
            };
            if property.kind() != PropertyKind::Object {
                return Err(TestError::Domain(format!(
                    "property {via_property} is not an object property"
                )));
            }
            let mut results = BTreeSet::new();
            if let Some(assertions) = source.properties().get(via_property) {
                for assertion in assertions {
                    if let PropertyAssertion::Individual(target) = assertion {
                        results.insert(target.clone());
                    }
                }
            }
            Ok(results.into_iter().collect())
        }

        async fn shortest_path(
            &self,
            ontology: &Iri,
            start: &Iri,
            end: &Iri,
        ) -> Result<Option<Vec<Iri>>, Self::Error> {
            let guard = self.store.lock().unwrap();
            let ontology = guard.get(ontology).ok_or(TestError::Missing)?;
            let Some(_) = ontology.individual(start) else {
                return Err(TestError::Domain(format!("individual {start} missing")));
            };
            let Some(_) = ontology.individual(end) else {
                return Err(TestError::Domain(format!("individual {end} missing")));
            };

            let mut queue = VecDeque::from([(start.clone(), vec![start.clone()])]);
            let mut visited = BTreeSet::from([start.clone()]);
            while let Some((current, path)) = queue.pop_front() {
                if current == *end {
                    return Ok(Some(path));
                }
                if let Some(individual) = ontology.individual(&current) {
                    for (property_id, assertions) in individual.properties() {
                        let Some(property) = ontology.property(property_id) else {
                            continue;
                        };
                        if property.kind() != PropertyKind::Object {
                            continue;
                        }
                        for assertion in assertions {
                            if let PropertyAssertion::Individual(target) = assertion {
                                if visited.insert(target.clone()) {
                                    let mut next_path = path.clone();
                                    next_path.push(target.clone());
                                    queue.push_back((target.clone(), next_path));
                                }
                            }
                        }
                    }
                }
            }
            Ok(None)
        }
    }

    #[tokio::test]
    async fn repository_crud_roundtrip() {
        let repo = InMemoryOntologyRepository::default();
        let mut ontology = Ontology::new(iri("https://example.org/onto"));
        repo.insert(ontology.clone()).await.expect("insert");

        ontology = ontology.with_label("Example");
        repo.update(ontology.clone()).await.expect("update");

        let fetched = repo
            .get(ontology.id())
            .await
            .expect("get")
            .expect("ontology exists");
        assert_eq!(fetched.ontology.label(), Some("Example"));

        let summaries = repo.list().await.expect("list");
        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].label.as_deref(), Some("Example"));

        repo.delete(ontology.id()).await.expect("delete");
        assert!(repo.get(ontology.id()).await.expect("get").is_none());
    }

    #[tokio::test]
    async fn repository_maintains_domain_invariants() {
        let repo = InMemoryOntologyRepository::default();
        let ontology = Ontology::new(iri("https://example.org/onto"));
        repo.insert(ontology.clone()).await.expect("insert");

        let class = Class::new(iri("https://example.org/Class"));
        repo.attach_class(ontology.id(), class)
            .await
            .expect("class");

        let mut property = Property::new(iri("https://example.org/prop"), PropertyKind::Object);
        property.add_domain(iri("https://example.org/Class"));
        property.add_range(iri("https://example.org/Class"));
        repo.attach_property(ontology.id(), property)
            .await
            .expect("property");

        let mut individual = Individual::new(iri("https://example.org/alice"));
        individual.assert_type(iri("https://example.org/Class"));
        individual.add_property_assertion(
            iri("https://example.org/prop"),
            PropertyAssertion::Individual(iri("https://example.org/bob")),
        );
        repo.attach_individual(ontology.id(), individual)
            .await
            .expect("individual");

        let updated = repo.get(ontology.id()).await.expect("get").expect("exists");
        assert_eq!(updated.ontology.individuals().len(), 1);
    }

    #[tokio::test]
    async fn reasoning_operations_traverse_graphs() {
        let repo = InMemoryOntologyRepository::default();
        let mut ontology = Ontology::new(iri("https://example.org/onto"));
        let base = Class::new(iri("https://example.org/Base"));
        let mut derived = Class::new(iri("https://example.org/Derived"));
        derived.add_parent(base.id().clone());
        let mut specialized = Class::new(iri("https://example.org/Specialized"));
        specialized.add_parent(derived.id().clone());
        ontology.add_class(base.clone()).expect("base");
        ontology.add_class(derived.clone()).expect("derived");
        ontology
            .add_class(specialized.clone())
            .expect("specialized");

        let mut link = Property::new(iri("https://example.org/link"), PropertyKind::Object);
        link.add_domain(base.id().clone());
        link.add_range(base.id().clone());
        ontology.add_property(link).expect("link");

        let mut alice = Individual::new(iri("https://example.org/alice"));
        alice.assert_type(base.id().clone());
        alice.add_property_assertion(
            iri("https://example.org/link"),
            PropertyAssertion::Individual(iri("https://example.org/bob")),
        );
        ontology.add_individual(alice).expect("alice");

        let mut bob = Individual::new(iri("https://example.org/bob"));
        bob.assert_type(base.id().clone());
        ontology.add_individual(bob).expect("bob");

        repo.insert(ontology).await.expect("insert");

        let invalid_ontology = iri("https://example.org/invalid");
        let ancestors = repo.ancestors_of(&invalid_ontology, derived.id()).await;
        assert!(ancestors.is_err(), "invalid ontology iri should error");

        let ancestors = repo
            .ancestors_of(&iri("https://example.org/onto"), derived.id())
            .await
            .expect("ancestors");
        assert_eq!(ancestors, vec![base.id().clone()]);

        let ancestors = repo
            .ancestors_of(&iri("https://example.org/onto"), specialized.id())
            .await
            .expect("ancestors");
        assert_eq!(ancestors, vec![base.id().clone(), derived.id().clone()]);

        let descendants = repo
            .descendants_of(&iri("https://example.org/onto"), base.id())
            .await
            .expect("descendants");
        assert_eq!(
            descendants,
            vec![derived.id().clone(), specialized.id().clone()]
        );

        let related = repo
            .related_individuals(
                &iri("https://example.org/onto"),
                &iri("https://example.org/link"),
                &iri("https://example.org/alice"),
            )
            .await
            .expect("related");
        assert_eq!(related, vec![iri("https://example.org/bob")]);

        let path = repo
            .shortest_path(
                &iri("https://example.org/onto"),
                &iri("https://example.org/alice"),
                &iri("https://example.org/bob"),
            )
            .await
            .expect("path");
        assert_eq!(
            path,
            Some(vec![
                iri("https://example.org/alice"),
                iri("https://example.org/bob"),
            ])
        );
    }
}
