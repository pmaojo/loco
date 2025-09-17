//! Core ontology domain primitives and contracts.
//!
//! The module defines rich value objects and aggregate roots describing ontologies
//! independently from persistence or transport concerns. It embraces a hexagonal
//! architecture approach by keeping only pure domain constructs and traits that
//! describe the required infrastructure behavior.

pub mod entities;
pub mod repositories;
pub mod service;
pub mod value_objects;

pub use entities::{
    Class, Individual, Ontology, OntologyError, Property, PropertyAssertion, PropertyKind,
};
pub use repositories::{OntologyRepository, OntologySnapshot, OntologySummary, ReasoningQuery};
pub use service::{OntologyService, OntologyServiceError};
pub use value_objects::{Iri, IriError};
