use crate::value::ValueExt;
use serde_yaml::Value;
use std::str::FromStr;

pub struct Args;

impl Args {
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
}
