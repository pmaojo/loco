use std::fmt::{self, Display, Formatter};
use std::str::FromStr;

use oxrdf::NamedNode;
use thiserror::Error;

/// Value object ensuring that supplied text represents a valid IRI.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Iri {
    value: String,
}

impl Iri {
    /// Validates and constructs a new [`Iri`] value object.
    ///
    /// The constructor rejects malformed identifiers in order to guarantee that
    /// every entity uses canonical identifiers.
    pub fn new(value: impl Into<String>) -> Result<Self, IriError> {
        let value = value.into();
        NamedNode::new(value.as_str()).map_err(|_| IriError::Invalid {
            value: value.clone(),
        })?;
        Ok(Self { value })
    }

    /// Returns the underlying textual representation.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.value
    }
}

impl Display for Iri {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(&self.value)
    }
}

impl FromStr for Iri {
    type Err = IriError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s.to_owned())
    }
}

impl TryFrom<String> for Iri {
    type Error = IriError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

/// Errors produced when validating an [`Iri`].
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum IriError {
    /// The provided text could not be parsed as an IRI.
    #[error("invalid IRI: {value}")]
    Invalid { value: String },
}

#[cfg(test)]
mod tests {
    use super::Iri;

    #[test]
    fn accepts_valid_iri() {
        let iri = Iri::new("https://example.org/resource").expect("valid IRI");
        assert_eq!(iri.as_str(), "https://example.org/resource");
    }

    #[test]
    fn rejects_invalid_iri() {
        let err = Iri::new("not an iri").expect_err("invalid IRI");
        assert!(matches!(err, super::IriError::Invalid { value } if value == "not an iri"));
    }
}
