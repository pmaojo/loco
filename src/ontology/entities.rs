use std::collections::{BTreeMap, BTreeSet};

use thiserror::Error;

use super::value_objects::Iri;

/// Ontology class definition capturing parent relationships and metadata.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Class {
    id: Iri,
    label: Option<String>,
    comment: Option<String>,
    super_classes: BTreeSet<Iri>,
}

impl Class {
    /// Creates a new [`Class`] with the supplied identifier.
    #[must_use]
    pub fn new(id: Iri) -> Self {
        Self {
            id,
            label: None,
            comment: None,
            super_classes: BTreeSet::new(),
        }
    }

    /// Sets a human friendly label for the class.
    #[must_use]
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Sets a textual description for the class.
    #[must_use]
    pub fn with_comment(mut self, comment: impl Into<String>) -> Self {
        self.comment = Some(comment.into());
        self
    }

    /// Adds a new parent class relation.
    pub fn add_parent(&mut self, parent: Iri) -> bool {
        self.super_classes.insert(parent)
    }

    /// Removes a parent class relation.
    pub fn remove_parent(&mut self, parent: &Iri) -> bool {
        self.super_classes.remove(parent)
    }

    /// Returns the unique identifier of the class.
    #[must_use]
    pub fn id(&self) -> &Iri {
        &self.id
    }

    /// Returns the optional label.
    #[must_use]
    pub fn label(&self) -> Option<&str> {
        self.label.as_deref()
    }

    /// Returns the optional comment.
    #[must_use]
    pub fn comment(&self) -> Option<&str> {
        self.comment.as_deref()
    }

    /// Returns the parent classes in lexical order.
    #[must_use]
    pub fn parents(&self) -> &BTreeSet<Iri> {
        &self.super_classes
    }
}

/// Ontology property definition supporting object and data properties.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Property {
    id: Iri,
    label: Option<String>,
    kind: PropertyKind,
    domains: BTreeSet<Iri>,
    ranges: BTreeSet<Iri>,
}

impl Property {
    /// Creates a new property with the provided identifier and kind.
    #[must_use]
    pub fn new(id: Iri, kind: PropertyKind) -> Self {
        Self {
            id,
            label: None,
            kind,
            domains: BTreeSet::new(),
            ranges: BTreeSet::new(),
        }
    }

    /// Sets a human readable label for the property.
    #[must_use]
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Declares that the property applies to the supplied domain class.
    pub fn add_domain(&mut self, class: Iri) -> bool {
        self.domains.insert(class)
    }

    /// Declares that the property produces values from the supplied range class.
    pub fn add_range(&mut self, class: Iri) -> bool {
        self.ranges.insert(class)
    }

    /// Returns the property identifier.
    #[must_use]
    pub fn id(&self) -> &Iri {
        &self.id
    }

    /// Returns the optional label.
    #[must_use]
    pub fn label(&self) -> Option<&str> {
        self.label.as_deref()
    }

    /// Returns the property kind.
    #[must_use]
    pub fn kind(&self) -> PropertyKind {
        self.kind
    }

    /// Returns the registered domain classes.
    #[must_use]
    pub fn domains(&self) -> &BTreeSet<Iri> {
        &self.domains
    }

    /// Returns the registered range classes.
    #[must_use]
    pub fn ranges(&self) -> &BTreeSet<Iri> {
        &self.ranges
    }
}

/// Classifies the type of values a property can hold.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum PropertyKind {
    /// Object properties link individuals.
    Object,
    /// Data properties capture literal values.
    Data,
}

/// Property assertions attached to individuals.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PropertyAssertion {
    /// Object properties target another individual.
    Individual(Iri),
    /// Data properties store literal values.
    Literal(String),
}

/// An ontology individual containing class memberships and property assertions.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Individual {
    id: Iri,
    types: BTreeSet<Iri>,
    properties: BTreeMap<Iri, Vec<PropertyAssertion>>,
}

impl Individual {
    /// Creates a new individual with the supplied identifier.
    #[must_use]
    pub fn new(id: Iri) -> Self {
        Self {
            id,
            types: BTreeSet::new(),
            properties: BTreeMap::new(),
        }
    }

    /// Declares that the individual is an instance of the given class.
    pub fn assert_type(&mut self, class: Iri) -> bool {
        self.types.insert(class)
    }

    /// Associates the individual with a property assertion.
    pub fn add_property_assertion(&mut self, property: Iri, assertion: PropertyAssertion) {
        self.properties
            .entry(property)
            .or_insert_with(Vec::new)
            .push(assertion);
    }

    /// Returns the identifier of the individual.
    #[must_use]
    pub fn id(&self) -> &Iri {
        &self.id
    }

    /// Returns the declared types.
    #[must_use]
    pub fn types(&self) -> &BTreeSet<Iri> {
        &self.types
    }

    /// Returns the property assertions.
    #[must_use]
    pub fn properties(&self) -> &BTreeMap<Iri, Vec<PropertyAssertion>> {
        &self.properties
    }
}

/// Aggregates ontology classes, properties and individuals.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ontology {
    id: Iri,
    label: Option<String>,
    classes: BTreeMap<Iri, Class>,
    properties: BTreeMap<Iri, Property>,
    individuals: BTreeMap<Iri, Individual>,
}

impl Ontology {
    /// Creates a new ontology aggregate with the supplied identifier.
    #[must_use]
    pub fn new(id: Iri) -> Self {
        Self {
            id,
            label: None,
            classes: BTreeMap::new(),
            properties: BTreeMap::new(),
            individuals: BTreeMap::new(),
        }
    }

    /// Sets a human readable label for the ontology.
    #[must_use]
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Adds a class to the ontology, enforcing unique identifiers.
    pub fn add_class(&mut self, class: Class) -> Result<(), OntologyError> {
        let id = class.id().clone();
        if self.classes.contains_key(&id) {
            return Err(OntologyError::DuplicateClass(id));
        }
        self.classes.insert(id, class);
        Ok(())
    }

    /// Adds a property to the ontology, validating references to known classes.
    pub fn add_property(&mut self, property: Property) -> Result<(), OntologyError> {
        let id = property.id().clone();
        if self.properties.contains_key(&id) {
            return Err(OntologyError::DuplicateProperty(id));
        }

        for class in property.domains() {
            if !self.classes.contains_key(class) {
                return Err(OntologyError::MissingClass {
                    ontology: self.id.clone(),
                    class: class.clone(),
                });
            }
        }
        for class in property.ranges() {
            if !self.classes.contains_key(class) {
                return Err(OntologyError::MissingClass {
                    ontology: self.id.clone(),
                    class: class.clone(),
                });
            }
        }

        self.properties.insert(id, property);
        Ok(())
    }

    /// Adds an individual ensuring it references known classes and properties.
    pub fn add_individual(&mut self, individual: Individual) -> Result<(), OntologyError> {
        let id = individual.id().clone();
        if self.individuals.contains_key(&id) {
            return Err(OntologyError::DuplicateIndividual(id));
        }

        for class in individual.types() {
            if !self.classes.contains_key(class) {
                return Err(OntologyError::MissingClass {
                    ontology: self.id.clone(),
                    class: class.clone(),
                });
            }
        }

        for (property_id, assertions) in individual.properties() {
            let Some(property) = self.properties.get(property_id) else {
                return Err(OntologyError::MissingProperty {
                    ontology: self.id.clone(),
                    property: property_id.clone(),
                });
            };

            for assertion in assertions {
                match (property.kind(), assertion) {
                    (PropertyKind::Object, PropertyAssertion::Individual(_)) => {}
                    (PropertyKind::Data, PropertyAssertion::Literal(_)) => {}
                    _ => {
                        return Err(OntologyError::InvalidPropertyAssertion {
                            ontology: self.id.clone(),
                            property: property_id.clone(),
                        });
                    }
                }
            }
        }

        self.individuals.insert(id, individual);
        Ok(())
    }

    /// Returns the ontology identifier.
    #[must_use]
    pub fn id(&self) -> &Iri {
        &self.id
    }

    /// Returns the optional label.
    #[must_use]
    pub fn label(&self) -> Option<&str> {
        self.label.as_deref()
    }

    /// Retrieves a class by identifier.
    #[must_use]
    pub fn class(&self, id: &Iri) -> Option<&Class> {
        self.classes.get(id)
    }

    /// Retrieves a property by identifier.
    #[must_use]
    pub fn property(&self, id: &Iri) -> Option<&Property> {
        self.properties.get(id)
    }

    /// Retrieves an individual by identifier.
    #[must_use]
    pub fn individual(&self, id: &Iri) -> Option<&Individual> {
        self.individuals.get(id)
    }

    /// Returns all classes ordered by identifier.
    #[must_use]
    pub fn classes(&self) -> &BTreeMap<Iri, Class> {
        &self.classes
    }

    /// Returns all properties ordered by identifier.
    #[must_use]
    pub fn properties(&self) -> &BTreeMap<Iri, Property> {
        &self.properties
    }

    /// Returns all individuals ordered by identifier.
    #[must_use]
    pub fn individuals(&self) -> &BTreeMap<Iri, Individual> {
        &self.individuals
    }
}

/// Errors raised when manipulating an ontology aggregate.
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum OntologyError {
    /// Attempted to add a class with an existing identifier.
    #[error("class `{0}` already exists")]
    DuplicateClass(Iri),
    /// Attempted to add a property with an existing identifier.
    #[error("property `{0}` already exists")]
    DuplicateProperty(Iri),
    /// Attempted to add an individual with an existing identifier.
    #[error("individual `{0}` already exists")]
    DuplicateIndividual(Iri),
    /// Referenced class was not part of the ontology.
    #[error("class `{class}` does not exist in ontology `{ontology}`")]
    MissingClass { ontology: Iri, class: Iri },
    /// Referenced property was not part of the ontology.
    #[error("property `{property}` does not exist in ontology `{ontology}`")]
    MissingProperty { ontology: Iri, property: Iri },
    /// Property assertion type did not match the property definition.
    #[error("property assertion does not match property `{property}` in ontology `{ontology}`")]
    InvalidPropertyAssertion { ontology: Iri, property: Iri },
}

#[cfg(test)]
mod tests {
    use super::{Class, Individual, Ontology, Property, PropertyAssertion, PropertyKind};
    use crate::ontology::value_objects::Iri;

    fn iri(text: &str) -> Iri {
        Iri::new(text).expect("valid iri")
    }

    #[test]
    fn class_parents_are_tracked() {
        let mut class = Class::new(iri("https://example.org/Class"))
            .with_label("Example")
            .with_comment("Demo");
        assert_eq!(class.label(), Some("Example"));
        assert_eq!(class.comment(), Some("Demo"));
        assert!(class.add_parent(iri("https://example.org/Base")));
        assert!(class.parents().contains(&iri("https://example.org/Base")));
        assert!(class.remove_parent(&iri("https://example.org/Base")));
        assert!(class.parents().is_empty());
    }

    #[test]
    fn property_definitions_require_known_classes() {
        let mut ontology = Ontology::new(iri("https://example.org/onto"));
        let class = Class::new(iri("https://example.org/Class"));
        ontology.add_class(class).expect("class inserted");

        let mut property = Property::new(iri("https://example.org/prop"), PropertyKind::Object);
        property.add_domain(iri("https://example.org/Class"));
        property.add_range(iri("https://example.org/Class"));
        ontology
            .add_property(property.clone())
            .expect("property inserted");
        assert_eq!(ontology.property(property.id()), Some(&property));
    }

    #[test]
    fn property_insertion_rejects_unknown_classes() {
        let mut ontology = Ontology::new(iri("https://example.org/onto"));
        let mut property = Property::new(iri("https://example.org/prop"), PropertyKind::Object);
        property.add_domain(iri("https://example.org/Class"));
        let err = ontology.add_property(property).expect_err("missing class");
        assert!(matches!(err, super::OntologyError::MissingClass { .. }));
    }

    #[test]
    fn individual_insertion_checks_references() {
        let mut ontology = Ontology::new(iri("https://example.org/onto"));
        let class = Class::new(iri("https://example.org/Class"));
        ontology.add_class(class).expect("class inserted");
        let mut property = Property::new(iri("https://example.org/prop"), PropertyKind::Object);
        property.add_domain(iri("https://example.org/Class"));
        property.add_range(iri("https://example.org/Class"));
        ontology.add_property(property).expect("property inserted");

        let mut individual = Individual::new(iri("https://example.org/alice"));
        individual.assert_type(iri("https://example.org/Class"));
        individual.add_property_assertion(
            iri("https://example.org/prop"),
            PropertyAssertion::Individual(iri("https://example.org/bob")),
        );

        ontology
            .add_individual(individual)
            .expect("individual inserted");
    }

    #[test]
    fn individual_insertion_rejects_mismatched_property_kind() {
        let mut ontology = Ontology::new(iri("https://example.org/onto"));
        let class = Class::new(iri("https://example.org/Class"));
        ontology.add_class(class).expect("class inserted");
        let mut property = Property::new(iri("https://example.org/prop"), PropertyKind::Data);
        property.add_domain(iri("https://example.org/Class"));
        ontology.add_property(property).expect("property inserted");

        let mut individual = Individual::new(iri("https://example.org/alice"));
        individual.assert_type(iri("https://example.org/Class"));
        individual.add_property_assertion(
            iri("https://example.org/prop"),
            PropertyAssertion::Individual(iri("https://example.org/bob")),
        );

        let err = ontology
            .add_individual(individual)
            .expect_err("mismatched property kind");
        assert!(matches!(
            err,
            super::OntologyError::InvalidPropertyAssertion { .. }
        ));
    }
}
