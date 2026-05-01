// Raw-reference resolution boundary. Current graph/vector storage keeps
// raw references as pointers; production raw storage can implement this later.
#![allow(dead_code)]

use async_trait::async_trait;

use crate::errors::CustomError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RawReference {
    pub(crate) reference: String,
    pub(crate) text: String,
}

impl RawReference {
    pub(crate) fn new(reference: impl Into<String>, text: impl Into<String>) -> Self {
        Self {
            reference: reference.into(),
            text: text.into(),
        }
    }
}

#[async_trait]
pub(crate) trait RawReferenceResolver: Send + Sync {
    async fn resolve(&self, reference: &str) -> Result<Option<RawReference>, CustomError>;
}

#[async_trait]
impl<T: RawReferenceResolver + ?Sized> RawReferenceResolver for Box<T> {
    async fn resolve(&self, reference: &str) -> Result<Option<RawReference>, CustomError> {
        (**self).resolve(reference).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug)]
    struct StaticRawReferenceResolver {
        result: StaticRawReferenceResult,
    }

    #[derive(Debug)]
    enum StaticRawReferenceResult {
        Resolved(Option<RawReference>),
        DatabaseError(String),
        MemoryValidation(String),
    }

    #[async_trait]
    impl RawReferenceResolver for StaticRawReferenceResolver {
        async fn resolve(&self, _reference: &str) -> Result<Option<RawReference>, CustomError> {
            match &self.result {
                StaticRawReferenceResult::Resolved(raw_reference) => Ok(raw_reference.clone()),
                StaticRawReferenceResult::DatabaseError(message) => {
                    Err(CustomError::DatabaseError(message.clone()))
                }
                StaticRawReferenceResult::MemoryValidation(message) => {
                    Err(CustomError::MemoryValidation(message.clone()))
                }
            }
        }
    }

    #[test]
    fn raw_reference_is_resolved_content_not_storage_configuration() {
        let raw = RawReference::new("chat://conversation/1#turn=2", "hello");

        assert_eq!(raw.reference, "chat://conversation/1#turn=2");
        assert_eq!(raw.text, "hello");
    }

    #[tokio::test]
    async fn raw_reference_resolver_can_resolve_internal_source_content() {
        let resolver = StaticRawReferenceResolver {
            result: StaticRawReferenceResult::Resolved(Some(RawReference::new(
                "raw://conversation/1#turn=2",
                "resolved fixture text",
            ))),
        };

        let resolved = resolver
            .resolve("raw://conversation/1#turn=2")
            .await
            .unwrap()
            .unwrap();

        assert_eq!(resolved.reference, "raw://conversation/1#turn=2");
        assert_eq!(resolved.text, "resolved fixture text");
    }

    #[tokio::test]
    async fn raw_reference_resolver_reports_unavailable_reference_as_none() {
        let resolver = StaticRawReferenceResolver {
            result: StaticRawReferenceResult::Resolved(None),
        };

        let resolved = resolver
            .resolve("raw://conversation/missing#turn=9")
            .await
            .unwrap();

        assert_eq!(resolved, None);
    }

    #[tokio::test]
    async fn raw_reference_resolver_propagates_resolver_failures_as_errors() {
        let resolver = StaticRawReferenceResolver {
            result: StaticRawReferenceResult::DatabaseError("resolver unavailable".to_owned()),
        };

        let error = resolver
            .resolve("raw://conversation/1#turn=2")
            .await
            .unwrap_err();

        assert!(matches!(
            error,
            CustomError::DatabaseError(message) if message == "resolver unavailable"
        ));
    }

    #[tokio::test]
    async fn raw_reference_resolver_fixture_can_return_non_database_errors() {
        let resolver = StaticRawReferenceResolver {
            result: StaticRawReferenceResult::MemoryValidation("invalid raw ref".to_owned()),
        };

        let error = resolver
            .resolve("raw://conversation/1#turn=2")
            .await
            .unwrap_err();

        assert!(matches!(
            error,
            CustomError::MemoryValidation(message) if message == "invalid raw ref"
        ));
    }
}
