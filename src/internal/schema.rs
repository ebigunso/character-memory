use crate::api::types::CURRENT_SCHEMA_VERSION;
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

// This seam is intentionally unused by production paths while only the current
// schema exists; boundary code should reject unsupported versions until a
// second schema introduces a real migration.
#[allow(dead_code)]
pub(crate) fn migrate_current_schema<T>(
    value: T,
    schema_version: &str,
    context: &'static str,
) -> Result<T, CustomError> {
    require_current_schema_version(schema_version, context)?;
    Ok(value)
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
    fn current_schema_migration_is_a_noop() {
        let value = String::from("unchanged");

        let migrated = migrate_current_schema(
            value.clone(),
            CURRENT_SCHEMA_VERSION,
            "test mapping boundary",
        )
        .expect("current schema should migrate without changes");

        assert_eq!(migrated, value);
    }

    #[test]
    fn forward_migrations_are_absent_until_a_second_schema_exists() {
        let value = String::from("unchanged");

        let error = migrate_current_schema(value, "future_schema", "test migration boundary")
            .expect_err("unsupported schema should not be migrated without a real migration");

        assert!(matches!(
            error,
            CustomError::UnsupportedSchemaVersion {
                context: "test migration boundary",
                expected: CURRENT_SCHEMA_VERSION,
                actual
            } if actual == "future_schema"
        ));
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
