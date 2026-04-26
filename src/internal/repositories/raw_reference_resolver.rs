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

    #[test]
    fn raw_reference_is_resolved_content_not_storage_configuration() {
        let raw = RawReference::new("chat://conversation/1#turn=2", "hello");

        assert_eq!(raw.reference, "chat://conversation/1#turn=2");
        assert_eq!(raw.text, "hello");
    }
}
