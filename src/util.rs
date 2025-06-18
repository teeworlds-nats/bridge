use crate::model::CowString;
use crate::value::ValueExt;
use lazy_static::lazy_static;
use log::warn;
use regex::{Captures, Regex};
use serde_yaml::Value;
use std::borrow::Cow;
use std::str::FromStr;

lazy_static! {
    static ref RE: Regex = Regex::new(r"\{\{([^\}]+)\}\}").unwrap();
}

pub fn format<'c, I>(
    input: I,
    args: &Value,
    list_values: &[String],
    default: Vec<CowString<'c>>,
) -> Vec<CowString<'static>>
where
    I: Into<Option<Vec<CowString<'c>>>>,
{
    input
        .into()
        .unwrap_or(default)
        .into_iter()
        .map(|x| get_and_format(&x, args, list_values))
        .collect()
}

pub fn format_caps<'c, I>(
    input: I,
    args: &Value,
    caps: Option<&Captures>,
    default: Vec<CowString<'c>>,
) -> Vec<CowString<'static>>
where
    I: Into<Option<Vec<CowString<'c>>>>,
{
    let list_values: Vec<String> = caps
        .as_ref()
        .map(|c| {
            c.iter()
                .filter_map(|cap| cap.map(|m| m.as_str().to_string()))
                .collect()
        })
        .unwrap_or_default();

    format(input, args, &list_values, default)
}

pub fn format_single<'c>(
    input: Option<CowString<'c>>,
    args: &Value,
    list_values: &[String],
    default: CowString<'c>,
) -> CowString<'static> {
    get_and_format(&input.unwrap_or(default), args, list_values)
}

pub fn get<T: FromStr, V: ValueExt>(args: &V, index: &str, default: T) -> T
where
    T::Err: std::fmt::Debug,
{
    args.get(index)
        .and_then(|value| {
            value
                .as_str()
                .and_then(|s| s.parse().ok())
                .or_else(|| value.as_i64().and_then(|n| n.to_string().parse().ok()))
                .or_else(|| value.as_bool().and_then(|b| b.to_string().parse().ok()))
                .or_else(|| value.as_f64().and_then(|f| f.to_string().parse().ok()))
        })
        .unwrap_or(default)
}

pub fn merge_yaml_values(original: &Value, new: &Value) -> Value {
    match (original, new) {
        (Value::Mapping(original_map), Value::Mapping(new_map)) => {
            let mut merged_map = original_map.clone();

            for (key, new_value) in new_map {
                merged_map.insert(key.clone(), new_value.clone());
            }

            Value::Mapping(merged_map)
        }
        (Value::Null, Value::Mapping(_)) => new.clone(),
        _ => original.clone(),
    }
}

pub fn get_and_format(string: &str, args: &Value, list_values: &[String]) -> CowString<'static> {
    if !string.contains("{{") {
        return Cow::Owned(string.to_string());
    }

    let new_args = {
        let mut new_args = args.clone();

        let path_server_name = get(args, "path_server_name", "server_name".to_string());
        let path_thread_id = get(args, "path_thread_id", "message_thread_id".to_string());

        new_args["server_name"] = Value::String(get(args, &path_server_name, "".to_string()));
        new_args["message_thread_id"] = Value::from(get::<i64, Value>(args, &path_thread_id, -1));

        new_args
    };

    Cow::Owned(
        RE.replace_all(string, |caps: &Captures| {
            let key = &caps[1];

            if let Ok(index) = key.parse::<usize>() {
                return list_values
                    .get(index)
                    .unwrap_or(&"".to_string())
                    .to_string();
            }

            let mut current_value = &new_args;
            for part in key.split('.') {
                if let Value::Mapping(map) = current_value {
                    if let Some(value) = map.get(Value::String(part.to_string())) {
                        current_value = value;
                    } else {
                        return String::new();
                    }
                } else {
                    return String::new();
                }
            }

            if let Some(s) = current_value.as_str() {
                s.to_string()
            } else if let Some(n) = current_value.as_i64() {
                n.to_string()
            } else if let Some(b) = current_value.as_bool() {
                b.to_string()
            } else if let Some(f) = current_value.as_f64() {
                f.to_string()
            } else {
                String::new()
            }
        })
        .into_owned(),
    )
}

pub fn convert<T>(payload: &[u8]) -> Option<T>
where
    T: serde::de::DeserializeOwned,
{
    match std::str::from_utf8(payload) {
        Ok(json_string) => match serde_json::from_str::<T>(json_string) {
            Ok(value) => Some(value),
            Err(err) => {
                warn!("Error deserializing JSON: {err}");
                None
            }
        },
        Err(err) => {
            warn!("Error converting bytes to string: {err}");
            None
        }
    }
}

pub fn escape_string(cow: CowString) -> CowString {
    if cow.contains(['"', '\'', '\\']) {
        let escaped = cow
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\'', "\\'");
        Cow::Owned(escaped)
    } else {
        cow
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_yaml::Value;

    // escape_sting
    #[test]
    fn test_no_escaping_needed() {
        let input = CowString::Owned("normal string".to_string());
        let result = escape_string(input);
        assert_eq!(result, CowString::Owned("normal string".to_string()));
    }

    #[test]
    fn test_escape_single_quotes() {
        let input = CowString::Owned("text with 'quotes'".to_string());
        let result = escape_string(input);
        assert_eq!(result, CowString::Owned(r#"text with \'quotes\'"#.into()));
    }

    #[test]
    fn test_escape_all_special_chars() {
        let input = CowString::Owned(r#"mixed \ '" all"#.to_string());
        let result = escape_string(input);
        assert_eq!(result, CowString::Owned(r#"mixed \\ \'\" all"#.to_string()));
    }

    #[test]
    fn test_empty_string() {
        let input = CowString::Owned("".to_string());
        let result = escape_string(input);
        assert_eq!(result, CowString::Owned("".to_string()));
    }

    #[test]
    fn test_owned_string_no_escape() {
        let input = CowString::Owned("hello".into());
        let result = escape_string(input);
        assert!(matches!(result, Cow::Owned(_)));
        assert_eq!(result.to_string(), "hello");
    }

    #[test]
    fn test_owned_string_with_escape() {
        let input = CowString::Owned(r#"hello"world"#.into());
        let result = escape_string(input);
        assert!(matches!(result, Cow::Owned(_)));
        assert_eq!(result.to_string(), r#"hello\"world"#);
    }

    // get_and_format
    #[test]
    fn test_no_placeholders() {
        let input = "Hello world";
        let args = Value::Null;
        let list_values = vec![];

        let result = get_and_format(input, &args, &list_values);
        assert_eq!(result, CowString::Owned("Hello world".to_string()));
    }

    #[test]
    fn test_simple_placeholder() {
        let input = "Hello {{name}}";
        let args = Value::Mapping({
            let mut map = serde_yaml::Mapping::new();
            map.insert(
                Value::String("name".to_string()),
                Value::String("Alice".to_string()),
            );
            map
        });
        let list_values = vec![];

        let result = get_and_format(input, &args, &list_values);
        assert_eq!(result, CowString::Owned("Hello Alice".to_string()));
    }

    #[test]
    fn test_multiple_placeholders() {
        let input = "{{greeting}}, {{name}}!";
        let args = Value::Mapping({
            let mut map = serde_yaml::Mapping::new();
            map.insert(
                Value::String("greeting".to_string()),
                Value::String("Hello".to_string()),
            );
            map.insert(
                Value::String("name".to_string()),
                Value::String("Bob".to_string()),
            );
            map
        });
        let list_values = vec![];

        let result = get_and_format(input, &args, &list_values);
        assert_eq!(result, CowString::Owned("Hello, Bob!".to_string()));
    }

    #[test]
    fn test_missing_placeholder() {
        let input = "Hello {{name}}";
        let args = Value::Null;
        let list_values = vec![];

        let result = get_and_format(input, &args, &list_values);
        assert_eq!(result, CowString::Owned("Hello ".to_string()));
    }

    #[test]
    fn test_list_index_placeholder() {
        let input = "Item 0: {{0}}, Item 1: {{1}}";
        let args = Value::Null;
        let list_values = vec!["Apple".to_string(), "Banana".to_string()];

        let result = get_and_format(input, &args, &list_values);
        assert_eq!(
            result,
            CowString::Owned("Item 0: Apple, Item 1: Banana".to_string())
        );
    }

    #[test]
    fn test_nested_placeholders() {
        let input = "{{user.name}} ({{user.id}})";
        let args = Value::Mapping({
            let mut user_map = serde_yaml::Mapping::new();
            user_map.insert(
                Value::String("name".to_string()),
                Value::String("Bob".to_string()),
            );
            user_map.insert(
                Value::String("id".to_string()),
                Value::String("789".to_string()),
            );

            let mut map = serde_yaml::Mapping::new();
            map.insert(Value::String("user".to_string()), Value::Mapping(user_map));
            map
        });
        let list_values = vec![];

        let result = get_and_format(input, &args, &list_values);
        assert_eq!(result, CowString::Owned("Bob (789)".to_string()));
    }
}
