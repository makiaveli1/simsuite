use serde::de::DeserializeOwned;

use crate::error::{AppError, AppResult};

pub(crate) fn parse_required_json_field<T>(
    json: &str,
    column_name: &str,
    record_label: &str,
    record_id: i64,
) -> AppResult<T>
where
    T: DeserializeOwned,
{
    serde_json::from_str(json).map_err(|error| {
        AppError::Message(format!(
            "Invalid {column_name} for {record_label} {record_id}: {error}"
        ))
    })
}

pub(crate) fn parse_optional_json_field<T>(
    json: Option<&str>,
    column_name: &str,
    record_label: &str,
    record_id: i64,
) -> AppResult<Option<T>>
where
    T: DeserializeOwned,
{
    json.map(|value| parse_required_json_field(value, column_name, record_label, record_id))
        .transpose()
}

#[cfg(test)]
mod tests {
    use super::{parse_optional_json_field, parse_required_json_field};

    #[test]
    fn parser_reports_column_and_record_context() {
        let parsed =
            parse_required_json_field::<Vec<String>>("[\"safe\"]", "caveats_json", "ApplyPlan", 7)
                .expect("parse required JSON field");
        assert_eq!(parsed, vec!["safe"]);

        let missing = parse_optional_json_field::<serde_json::Value>(
            None,
            "source_scope_json",
            "ApplyPlan",
            7,
        )
        .expect("parse absent optional JSON field");
        assert!(missing.is_none());

        let error = parse_required_json_field::<serde_json::Value>(
            "{",
            "plan_provenance_json",
            "ApplyPlan",
            7,
        )
        .expect_err("malformed persisted JSON should fail closed");
        assert!(error
            .to_string()
            .contains("Invalid plan_provenance_json for ApplyPlan 7"));
    }
}
