// Source-reference boundary. Current graph/vector storage keeps source
// references as opaque pointers; caller-owned source storage stays outside
// Character Memory core.
use async_trait::async_trait;

use crate::errors::CustomError;

#[derive(Debug, Clone, PartialEq, Eq)]
// Source resolution is a future boundary; remove if caller-owned source storage is dropped.
#[allow(dead_code)]
pub(crate) struct ResolvedSourceReference {
    pub(crate) reference: String,
    pub(crate) resolved_text: String,
}

impl ResolvedSourceReference {
    // Source resolver fixtures use this constructor; remove if source resolution is dropped.
    #[allow(dead_code)]
    pub(crate) fn new(reference: impl Into<String>, resolved_text: impl Into<String>) -> Self {
        Self {
            reference: reference.into(),
            resolved_text: resolved_text.into(),
        }
    }
}

#[async_trait]
// Source resolution is a future boundary; remove if caller-owned source storage is dropped.
#[allow(dead_code)]
pub(crate) trait SourceReferenceResolver: Send + Sync {
    async fn resolve(
        &self,
        reference: &str,
    ) -> Result<Option<ResolvedSourceReference>, CustomError>;
}

#[async_trait]
impl<T: SourceReferenceResolver + ?Sized> SourceReferenceResolver for Box<T> {
    async fn resolve(
        &self,
        reference: &str,
    ) -> Result<Option<ResolvedSourceReference>, CustomError> {
        (**self).resolve(reference).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug)]
    struct StaticSourceReferenceResolver {
        result: StaticSourceReferenceResult,
    }

    #[derive(Debug)]
    enum StaticSourceReferenceResult {
        Resolved(Option<ResolvedSourceReference>),
        DatabaseError(String),
        MemoryValidation(String),
    }

    #[async_trait]
    impl SourceReferenceResolver for StaticSourceReferenceResolver {
        async fn resolve(
            &self,
            _reference: &str,
        ) -> Result<Option<ResolvedSourceReference>, CustomError> {
            match &self.result {
                StaticSourceReferenceResult::Resolved(source_reference) => {
                    Ok(source_reference.clone())
                }
                StaticSourceReferenceResult::DatabaseError(message) => {
                    Err(CustomError::DatabaseError(message.clone()))
                }
                StaticSourceReferenceResult::MemoryValidation(message) => {
                    Err(CustomError::MemoryValidation(message.clone()))
                }
            }
        }
    }

    #[test]
    fn source_reference_is_resolved_content_not_storage_configuration() {
        let source = ResolvedSourceReference::new("chat://conversation/1#turn=2", "hello");

        assert_eq!(source.reference, "chat://conversation/1#turn=2");
        assert_eq!(source.resolved_text, "hello");
    }

    #[tokio::test]
    async fn source_reference_resolver_can_resolve_internal_source_content() {
        let resolver = StaticSourceReferenceResolver {
            result: StaticSourceReferenceResult::Resolved(Some(ResolvedSourceReference::new(
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
        assert_eq!(resolved.resolved_text, "resolved fixture text");
    }

    #[tokio::test]
    async fn source_reference_resolver_reports_unavailable_reference_as_none() {
        let resolver = StaticSourceReferenceResolver {
            result: StaticSourceReferenceResult::Resolved(None),
        };

        let resolved = resolver
            .resolve("raw://conversation/missing#turn=9")
            .await
            .unwrap();

        assert_eq!(resolved, None);
    }

    #[tokio::test]
    async fn source_reference_resolver_propagates_resolver_failures_as_errors() {
        let resolver = StaticSourceReferenceResolver {
            result: StaticSourceReferenceResult::DatabaseError("resolver unavailable".to_owned()),
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
    async fn source_reference_resolver_fixture_can_return_non_database_errors() {
        let resolver = StaticSourceReferenceResolver {
            result: StaticSourceReferenceResult::MemoryValidation("invalid raw ref".to_owned()),
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
