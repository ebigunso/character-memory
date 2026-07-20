use super::CURRENT_SCHEMA_VERSION;
use crate::errors::CustomError;

pub(crate) fn require_current_schema_version(
    schema_version: &str,
    context: &'static str,
) -> Result<(), CustomError> {
    if schema_version == CURRENT_SCHEMA_VERSION {
        return Ok(());
    }

    Err(CustomError::UnsupportedSchemaVersion {
        context,
        expected: CURRENT_SCHEMA_VERSION,
        actual: schema_version.to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_schema_version_is_accepted() {
        assert!(
            require_current_schema_version(CURRENT_SCHEMA_VERSION, "test storage boundary").is_ok()
        );
    }

    #[test]
    fn unsupported_schema_version_is_rejected_with_context() {
        let error = require_current_schema_version("future_schema", "test storage boundary")
            .expect_err("unsupported schema should fail");

        assert!(matches!(
            error,
            CustomError::UnsupportedSchemaVersion {
                context: "test storage boundary",
                expected: CURRENT_SCHEMA_VERSION,
                actual
            } if actual == "future_schema"
        ));
    }
}
