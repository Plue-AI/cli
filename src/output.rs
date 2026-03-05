use clap::Args;
use serde::Serialize;
use serde_json::{Map, Value};

/// Output format for CLI results.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OutputFormat {
    /// Human-readable table/text output (default)
    Table,
    /// Machine-readable JSON
    Json { fields: Option<String> },
    /// Token-Oriented Object Notation — optimized for LLM consumption (~40% fewer tokens than JSON)
    Toon { fields: Option<String> },
}

/// Global output flags available on every command.
#[derive(Debug, Args)]
pub struct OutputArgs {
    /// Output as JSON (optionally projected with comma-separated fields via --json=field1,field2)
    #[arg(
        long,
        global = true,
        require_equals = true,
        num_args = 0..=1,
        default_missing_value = ""
    )]
    pub json: Option<String>,

    /// Output in TOON format (optionally projected with comma-separated fields via --toon=field1,field2)
    #[arg(
        long,
        global = true,
        require_equals = true,
        num_args = 0..=1,
        default_missing_value = ""
    )]
    pub toon: Option<String>,

    /// Disable color output
    #[arg(long, global = true)]
    pub no_color: bool,
}

impl OutputArgs {
    pub fn format(&self) -> OutputFormat {
        if let Some(fields) = &self.json {
            OutputFormat::Json {
                fields: Some(fields.clone()),
            }
        } else if let Some(fields) = &self.toon {
            OutputFormat::Toon {
                fields: Some(fields.clone()),
            }
        } else {
            OutputFormat::Table
        }
    }
}

/// Print a value according to the output format.
pub fn print_value<T: Serialize + std::fmt::Display>(value: &T, format: OutputFormat) {
    println!("{}", render_value(value, &format));
}

/// Serialize any `Serialize` value to TOON and print it.
///
/// Convenience helper used by command handlers with an `OutputFormat::Toon { fields }` arm.
pub fn print_toon<T: Serialize>(value: &T, fields: Option<&str>) {
    println!("{}", render_toon_value(value, fields));
}

fn render_value<T: Serialize + std::fmt::Display>(value: &T, format: &OutputFormat) -> String {
    match format {
        OutputFormat::Json { fields } => {
            let json_value = serde_json::to_value(value).expect("failed to serialize to JSON");
            let projected = fields
                .as_deref()
                .filter(|f| !f.is_empty())
                .map_or(json_value.clone(), |wanted| {
                    filter_fields(&json_value, wanted)
                });
            serde_json::to_string_pretty(&projected).expect("failed to serialize to JSON")
        }
        OutputFormat::Toon { fields } => render_toon_value(value, fields.as_deref()),
        OutputFormat::Table => value.to_string(),
    }
}

fn render_toon_value<T: Serialize>(value: &T, fields: Option<&str>) -> String {
    let json_value = serde_json::to_value(value).expect("failed to serialize for TOON output");
    let field_list = fields.filter(|f| !f.is_empty()).map(parse_fields);
    to_toon(&json_value, field_list.as_deref())
}

/// Serialize a JSON value to TOON (Token-Oriented Object Notation).
///
/// TOON is designed for LLM consumption with ~40% fewer tokens than JSON:
/// - Key-value pairs separated by spaces: `key:value`
/// - Simple values (numbers, booleans, simple strings) without quotes
/// - String values with whitespace or special characters are quoted: `title:"Add auth"`
/// - Nested objects use dot notation: `author.login:alice`
/// - Null values are skipped
/// - Arrays of objects produce one TOON line per element
/// - Optional field projection filters which keys are emitted
pub fn to_toon(value: &Value, fields: Option<&[String]>) -> String {
    let value = if let Some(fields) = fields {
        if fields.is_empty() {
            value.clone()
        } else {
            filter_fields_vec(value, fields)
        }
    } else {
        value.clone()
    };

    match &value {
        Value::Array(items) => items
            .iter()
            .map(|item| toon_encode_object(item, ""))
            .collect::<Vec<_>>()
            .join("\n"),
        Value::Object(_) => toon_encode_object(&value, ""),
        _ => toon_encode_scalar(&value),
    }
}

/// Encode a single JSON object (or scalar) into a TOON line.
fn toon_encode_object(value: &Value, prefix: &str) -> String {
    match value {
        Value::Object(obj) => {
            let pairs: Vec<String> = obj
                .iter()
                .filter(|(_, v)| !v.is_null())
                .flat_map(|(key, val)| {
                    let full_key = if prefix.is_empty() {
                        key.clone()
                    } else {
                        format!("{prefix}.{key}")
                    };
                    match val {
                        Value::Object(_) => {
                            vec![toon_encode_object(val, &full_key)]
                        }
                        Value::Array(items) => {
                            let encoded = items
                                .iter()
                                .map(toon_encode_scalar)
                                .collect::<Vec<_>>()
                                .join(",");
                            vec![format!("{full_key}:[{encoded}]")]
                        }
                        _ => {
                            vec![format!("{full_key}:{}", toon_encode_scalar(val))]
                        }
                    }
                })
                .collect();
            pairs.join(" ")
        }
        _ => toon_encode_scalar(value),
    }
}

/// Encode a scalar JSON value for TOON output.
fn toon_encode_scalar(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::String(s) => {
            if s.is_empty() {
                return "\"\"".to_string();
            }
            if needs_quoting(s) {
                serde_json::to_string(s).expect("failed to escape TOON string")
            } else {
                s.clone()
            }
        }
        Value::Array(items) => {
            let parts: Vec<String> = items.iter().map(toon_encode_scalar).collect();
            format!("[{}]", parts.join(","))
        }
        Value::Object(_) => toon_encode_object(value, ""),
    }
}

/// Check if a string value needs quoting in TOON format.
fn needs_quoting(s: &str) -> bool {
    s.contains(char::is_whitespace)
        || s.contains(':')
        || s.contains('\\')
        || s.contains('"')
        || s.contains('[')
        || s.contains(']')
        || s.contains(',')
}

/// Filter fields from a Value using a Vec<String> (for TOON field projection).
fn filter_fields_vec(value: &Value, wanted: &[String]) -> Value {
    if wanted.is_empty() {
        return value.clone();
    }
    match value {
        Value::Object(obj) => Value::Object(project_object(obj, wanted)),
        Value::Array(items) => Value::Array(
            items
                .iter()
                .map(|item| match item {
                    Value::Object(obj) => Value::Object(project_object(obj, wanted)),
                    _ => item.clone(),
                })
                .collect(),
        ),
        _ => value.clone(),
    }
}

pub fn filter_fields(value: &Value, fields: &str) -> Value {
    let wanted = parse_fields(fields);
    if wanted.is_empty() {
        return value.clone();
    }

    match value {
        Value::Object(obj) => Value::Object(project_object(obj, &wanted)),
        Value::Array(items) => Value::Array(
            items
                .iter()
                .map(|item| match item {
                    Value::Object(obj) => Value::Object(project_object(obj, &wanted)),
                    _ => item.clone(),
                })
                .collect(),
        ),
        _ => value.clone(),
    }
}

pub fn list_available_fields(value: &Value) -> Vec<String> {
    match value {
        Value::Object(obj) => obj.keys().cloned().collect(),
        Value::Array(items) => items
            .iter()
            .find_map(Value::as_object)
            .map(|obj| obj.keys().cloned().collect())
            .unwrap_or_default(),
        _ => Vec::new(),
    }
}

fn parse_fields(fields: &str) -> Vec<String> {
    fields
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn project_object(obj: &Map<String, Value>, wanted: &[String]) -> Map<String, Value> {
    let mut projected = Map::new();
    for key in wanted {
        if let Some(value) = obj.get(key) {
            projected.insert(key.clone(), value.clone());
        }
    }
    projected
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::{Parser, Subcommand};
    use serde_json::json;

    #[derive(Debug, Parser)]
    #[command(subcommand_precedence_over_arg = true)]
    struct TestCli {
        #[command(flatten)]
        output: OutputArgs,
        #[command(subcommand)]
        command: TestCommand,
    }

    #[derive(Debug, Subcommand)]
    enum TestCommand {
        List,
    }

    // ===== filter_fields tests =====

    #[test]
    fn filter_fields_object() {
        let value = json!({
            "number": 7,
            "title": "demo",
            "state": "open",
            "body": "ignore",
        });

        let filtered = filter_fields(&value, "number,title,state");

        assert_eq!(filtered["number"], 7);
        assert_eq!(filtered["title"], "demo");
        assert_eq!(filtered["state"], "open");
        assert!(filtered.get("body").is_none());
    }

    #[test]
    fn filter_fields_array() {
        let value = json!([
            {
                "number": 7,
                "title": "demo",
                "state": "open",
                "body": "ignore",
            },
            {
                "number": 8,
                "title": "next",
                "state": "closed",
                "body": "skip",
            }
        ]);

        let filtered = filter_fields(&value, "number,title");

        assert_eq!(filtered[0]["number"], 7);
        assert_eq!(filtered[0]["title"], "demo");
        assert!(filtered[0].get("state").is_none());
        assert_eq!(filtered[1]["number"], 8);
        assert_eq!(filtered[1]["title"], "next");
        assert!(filtered[1].get("state").is_none());
    }

    #[test]
    fn filter_fields_missing_fields_are_ignored() {
        let value = json!({
            "number": 7,
            "title": "demo",
        });

        let filtered = filter_fields(&value, "number,missing");

        assert_eq!(filtered["number"], 7);
        assert!(filtered.get("missing").is_none());
        assert!(filtered.get("title").is_none());
    }

    #[test]
    fn filter_fields_nested_object_is_retained() {
        let value = json!({
            "number": 7,
            "author": {
                "login": "alice",
                "id": 1,
            },
            "state": "open",
        });

        let filtered = filter_fields(&value, "author");

        assert_eq!(filtered["author"]["login"], "alice");
        assert!(filtered.get("number").is_none());
        assert!(filtered.get("state").is_none());
    }

    // ===== OutputArgs tests =====

    #[test]
    fn output_args_parses_json_without_value() {
        let parsed =
            TestCli::try_parse_from(["plue", "--json", "list"]).expect("json without value");
        assert_eq!(parsed.output.json.as_deref(), Some(""));
        assert!(parsed.output.toon.is_none());
    }

    #[test]
    fn output_args_parses_json_with_field_list() {
        let parsed = TestCli::try_parse_from(["plue", "--json=number,title,state", "list"])
            .expect("json with value");
        assert_eq!(parsed.output.json.as_deref(), Some("number,title,state"));
        assert!(parsed.output.toon.is_none());
    }

    #[test]
    fn output_args_parses_toon_without_value() {
        let parsed =
            TestCli::try_parse_from(["plue", "--toon", "list"]).expect("toon without value");
        assert_eq!(parsed.output.toon.as_deref(), Some(""));
        assert!(parsed.output.json.is_none());
    }

    #[test]
    fn output_args_parses_toon_with_field_list() {
        let parsed = TestCli::try_parse_from(["plue", "--toon=number,title", "list"])
            .expect("toon with value");
        assert_eq!(parsed.output.toon.as_deref(), Some("number,title"));
        assert!(parsed.output.json.is_none());
    }

    #[test]
    fn output_args_format_prefers_json_when_both_flags_present() {
        let parsed = TestCli::try_parse_from(["plue", "--json=number", "--toon=title", "list"])
            .expect("json and toon");
        assert_eq!(
            parsed.output.format(),
            OutputFormat::Json {
                fields: Some("number".to_string())
            }
        );
    }

    #[test]
    fn output_args_format_uses_toon_when_only_toon_is_present() {
        let parsed =
            TestCli::try_parse_from(["plue", "--toon=number,title", "list"]).expect("toon format");
        assert_eq!(
            parsed.output.format(),
            OutputFormat::Toon {
                fields: Some("number,title".to_string())
            }
        );
    }

    // ===== list_available_fields tests =====

    #[test]
    fn list_available_fields_for_object() {
        let value = json!({
            "number": 7,
            "title": "demo",
            "state": "open",
        });

        let mut fields = list_available_fields(&value);
        fields.sort();

        assert_eq!(fields, vec!["number", "state", "title"]);
    }

    #[test]
    fn list_available_fields_for_array_uses_first_object() {
        let value = json!([
            {
                "number": 7,
                "title": "demo",
                "state": "open",
            },
            {
                "number": 8,
                "title": "next",
                "body": "ignored",
            }
        ]);

        let mut fields = list_available_fields(&value);
        fields.sort();

        assert_eq!(fields, vec!["number", "state", "title"]);
    }

    // ===== render_value tests =====

    #[test]
    fn render_value_table_uses_display() {
        let rendered = render_value(&"hello".to_string(), &OutputFormat::Table);
        assert_eq!(rendered, "hello");
    }

    #[test]
    fn render_value_json_applies_field_projection() {
        let value = json!({
            "number": 42,
            "title": "Add auth",
            "state": "open"
        });
        let rendered = render_value(
            &value,
            &OutputFormat::Json {
                fields: Some("number,title".to_string()),
            },
        );
        let parsed: Value = serde_json::from_str(&rendered).expect("json output");
        assert_eq!(parsed["number"], 42);
        assert_eq!(parsed["title"], "Add auth");
        assert!(parsed.get("state").is_none());
    }

    #[test]
    fn render_value_toon_applies_field_projection() {
        let value = json!({
            "number": 42,
            "title": "Add auth",
            "state": "open"
        });
        let rendered = render_value(
            &value,
            &OutputFormat::Toon {
                fields: Some("number,title".to_string()),
            },
        );
        assert!(rendered.contains("number:42"));
        assert!(rendered.contains("title:\"Add auth\""));
        assert!(!rendered.contains("state"));
    }

    // ===== to_toon tests =====

    #[test]
    fn to_toon_simple_flat_object() {
        let value = json!({
            "number": 42,
            "state": "open",
            "title": "Add auth"
        });
        let result = to_toon(&value, None);
        assert!(result.contains("number:42"));
        assert!(result.contains("state:open"));
        assert!(result.contains("title:\"Add auth\""));
    }

    #[test]
    fn to_toon_nested_object_uses_dot_notation() {
        let value = json!({
            "number": 7,
            "author": {
                "login": "alice",
                "id": 1
            }
        });
        let result = to_toon(&value, None);
        assert!(result.contains("author.login:alice"));
        assert!(result.contains("author.id:1"));
        assert!(result.contains("number:7"));
    }

    #[test]
    fn to_toon_null_values_are_skipped() {
        let value = json!({
            "number": 7,
            "milestone_id": null,
            "title": "demo"
        });
        let result = to_toon(&value, None);
        assert!(result.contains("number:7"));
        assert!(result.contains("title:demo"));
        assert!(!result.contains("milestone_id"));
    }

    #[test]
    fn to_toon_boolean_values() {
        let value = json!({
            "is_empty": false,
            "is_public": true
        });
        let result = to_toon(&value, None);
        assert!(result.contains("is_empty:false"));
        assert!(result.contains("is_public:true"));
    }

    #[test]
    fn to_toon_array_of_objects_one_line_per_element() {
        let value = json!([
            {"number": 1, "title": "First"},
            {"number": 2, "title": "Second"}
        ]);
        let result = to_toon(&value, None);
        let lines: Vec<&str> = result.lines().collect();
        assert_eq!(lines.len(), 2);
        assert!(lines[0].contains("number:1"));
        assert!(lines[0].contains("title:First"));
        assert!(lines[1].contains("number:2"));
        assert!(lines[1].contains("title:Second"));
    }

    #[test]
    fn to_toon_empty_string_is_quoted() {
        let value = json!({
            "body": "",
            "title": "demo"
        });
        let result = to_toon(&value, None);
        assert!(result.contains("body:\"\""));
        assert!(result.contains("title:demo"));
    }

    #[test]
    fn to_toon_string_with_colon_is_quoted() {
        let value = json!({
            "url": "https://example.com"
        });
        let result = to_toon(&value, None);
        assert!(result.contains("url:\"https://example.com\""));
    }

    #[test]
    fn to_toon_field_projection_filters_keys() {
        let value = json!({
            "number": 42,
            "title": "Add auth",
            "state": "open",
            "body": "details"
        });
        let fields = vec!["number".to_string(), "title".to_string()];
        let result = to_toon(&value, Some(&fields));
        assert!(result.contains("number:42"));
        assert!(result.contains("title:\"Add auth\""));
        assert!(!result.contains("state"));
        assert!(!result.contains("body"));
    }

    #[test]
    fn to_toon_field_projection_on_array() {
        let value = json!([
            {"number": 1, "title": "First", "body": "skip"},
            {"number": 2, "title": "Second", "body": "skip"}
        ]);
        let fields = vec!["number".to_string(), "title".to_string()];
        let result = to_toon(&value, Some(&fields));
        let lines: Vec<&str> = result.lines().collect();
        assert_eq!(lines.len(), 2);
        assert!(lines[0].contains("number:1"));
        assert!(lines[0].contains("title:First"));
        assert!(!lines[0].contains("body"));
    }

    #[test]
    fn to_toon_scalar_value() {
        assert_eq!(to_toon(&json!(42), None), "42");
        assert_eq!(to_toon(&json!("hello"), None), "hello");
        assert_eq!(to_toon(&json!(true), None), "true");
    }

    #[test]
    fn to_toon_array_field_in_object() {
        let value = json!({
            "name": "main",
            "bookmarks": ["main", "dev"]
        });
        let result = to_toon(&value, None);
        assert!(result.contains("name:main"));
        assert!(result.contains("bookmarks:[main,dev]"));
    }

    #[test]
    fn to_toon_deeply_nested_object() {
        let value = json!({
            "repo": {
                "owner": {
                    "login": "alice"
                },
                "name": "demo"
            }
        });
        let result = to_toon(&value, None);
        assert!(result.contains("repo.owner.login:alice"));
        assert!(result.contains("repo.name:demo"));
    }

    #[test]
    fn to_toon_string_with_quotes_is_escaped() {
        let value = json!({
            "message": "say \"hello\""
        });
        let result = to_toon(&value, None);
        assert!(result.contains("message:\"say \\\"hello\\\"\""));
    }

    #[test]
    fn to_toon_string_with_newline_is_escaped() {
        let value = json!({
            "message": "line1\nline2"
        });
        let result = to_toon(&value, None);
        assert!(result.contains("message:\"line1\\nline2\""));
    }

    #[test]
    fn to_toon_empty_object() {
        let value = json!({});
        let result = to_toon(&value, None);
        assert_eq!(result, "");
    }

    #[test]
    fn to_toon_empty_array() {
        let value = json!([]);
        let result = to_toon(&value, None);
        assert_eq!(result, "");
    }

    #[test]
    fn to_toon_landing_request_realistic() {
        let value = json!({
            "number": 42,
            "title": "Add auth",
            "state": "open",
            "author": {
                "id": 1,
                "login": "alice"
            },
            "change_ids": ["kxyz"],
            "target_bookmark": "main",
            "stack_size": 2
        });
        let result = to_toon(&value, None);
        assert!(result.contains("number:42"));
        assert!(result.contains("title:\"Add auth\""));
        assert!(result.contains("state:open"));
        assert!(result.contains("author.id:1"));
        assert!(result.contains("author.login:alice"));
        assert!(result.contains("change_ids:[kxyz]"));
        assert!(result.contains("target_bookmark:main"));
        assert!(result.contains("stack_size:2"));
    }
}
