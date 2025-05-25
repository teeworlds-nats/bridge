use crate::model::CowString;
use lazy_static::lazy_static;
use regex::{Captures, Regex};
use serde_yaml::Value;
use std::borrow::Cow;

lazy_static! {
    static ref RE: Regex = Regex::new(r"\{\{([^}]+)}}").unwrap();
}

pub fn get(args: &Value, index: &str, default: &str) -> String {
    args.get(index)
        .and_then(Value::as_str)
        .unwrap_or(default)
        .to_string()
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
        _ => original.clone(),
    }
}

pub fn get_and_format(string: &str, args: &Value, list_values: &[String]) -> CowString<'static> {
    if !string.contains("{{") {
        return Cow::Owned(string.to_string());
    }

    let mut new_args = args.clone();

    let path_server_name = get(args, "path_server_name", "server_name");
    let path_thread_id = get(args, "path_thread_id", "message_thread_id");

    new_args["server_name"] = Value::String(get(args, &path_server_name, ""));
    new_args["message_thread_id"] = Value::String(get(args, &path_thread_id, "-1"));

    Cow::Owned(
        RE.replace_all(string, |caps: &Captures| {
            let key = &caps[1];

            if let Ok(index) = key.parse::<usize>() {
                list_values
                    .get(index)
                    .unwrap_or(&"".to_string())
                    .to_string()
            } else if let Some(value) = new_args.get(Value::String(key.to_string())) {
                value.as_str().unwrap_or("").to_string()
            } else {
                "".to_string()
            }
        })
        .to_string(),
    )
}

pub fn get_and_format_caps<'h>(
    string: &'h str,
    args: &Value,
    caps: Option<&Captures>,
) -> CowString<'h> {
    let list_values: Vec<String> = caps
        .as_ref()
        .map(|c| {
            c.iter()
                .filter_map(|cap| cap.map(|m| m.as_str().to_string()))
                .collect()
        })
        .unwrap_or_default();

    get_and_format(string, args, &list_values)
}
