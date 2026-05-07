use regex::Regex;
use std::sync::LazyLock;

static RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\{\{([^\}]+)\}\}").unwrap_or_else(|e| {
        panic!("Hardcoded regex failed to compile: {e}");
    })
});

pub mod formatting {
    use crate::args::Args;
    use crate::format::RE;
    use crate::model::CowStr;
    use regex::Captures;
    use serde_yaml::Value;

    /// Formats values with flexible input options
    ///
    /// This macro provides a unified interface for formatting values with different combinations
    /// of parameters, supporting both vectorized and single-value operations.
    ///
    /// # Usage Examples
    ///
    /// ## Basic vector formatting (without default)
    /// ```
    /// format_values!(input, args, list_values);
    /// ```
    /// Equivalent to `format(input, args, list_values, Vec::new())`
    ///
    /// ## Vector formatting with default values
    /// ```
    /// format_values!(input, args, list_values, default_vec);
    /// ```
    /// Equivalent to `format(input, args, list_values, default_vec)`
    ///
    /// ## Single value formatting (without default)
    /// ```
    /// format_values!(input, args, list_values; single);
    /// ```
    /// Equivalent to `get_and_format(input, args, list_values)`
    ///
    /// ## Single value formatting with default
    /// ```
    /// format_values!(input, args, list_values, default_val; single);
    /// ```
    /// Equivalent to `get_and_format(input.unwrap_or(default_val), args, list_values)`
    ///
    /// # Parameters
    /// - `input`: Input value(s) to format (either single value or vector)
    /// - `args`: Formatting arguments
    /// - `list_values`: List of values for reference
    /// - `default` (optional): Default value(s) to use when input is None
    ///
    /// # Behavior Selection
    /// The macro determines behavior based on:
    /// - Number of arguments (3 vs 4)
    /// - Presence of `; single` suffix
    ///
    /// # Notes
    /// - All variants maintain type safety through Rust's type system
    /// - The `single` variants automatically handle Option-wrapped inputs
    #[macro_export]
    macro_rules! format_values {
        // basic: (input, args, list_values)
        ($input:expr, $args:expr, $list:expr $(,)?) => {
            $crate::format::formatting::format_values($input, $args, $list, Vec::new())
        };

        // with_default: (input, args, list_values, default_vec)
        ($input:expr, $args:expr, $list:expr, $default:expr) => {
            $crate::format::formatting::format_values($input, $args, $list, $default)
        };

        // single: (input, args, list_values)
        ($input:expr, $args:expr, $list:expr; single) => {{
            $crate::format::formatting::get_and_format(&$input, $args, $list)
        }};

        // single with default: (input, args, list_values, default)
        ($input:expr, $args:expr, $list:expr, $default:expr; single) => {{
            $crate::format::formatting::get_and_format(&$input.unwrap_or($default), $args, $list)
        }};
    }

    pub fn format_values<'c, I, T>(
        input: I,
        args: &Value,
        list_values: &[T],
        default: Vec<CowStr<'c>>,
    ) -> Vec<CowStr<'static>>
    where
        I: Into<Option<Vec<CowStr<'c>>>>,
        T: AsRef<str>,
    {
        input
            .into()
            .unwrap_or(default)
            .into_iter()
            .map(|x| get_and_format(&x, args, list_values))
            .collect()
    }

    pub fn get_and_format<T: AsRef<str>>(
        string: &str,
        args: &Value,
        list_values: &[T],
    ) -> CowStr<'static> {
        if !string.contains("{{") {
            return CowStr::Owned(string.to_string());
        }

        let path_server_name = Args::get(args, "path_server_name", "server_name".to_string());
        let path_thread_id = Args::get(args, "path_thread_id", "message_thread_id".to_string());

        CowStr::Owned(
            RE.replace_all(string, |caps: &Captures| {
                let key = &caps[1];

                if let Ok(index) = key.parse::<usize>() {
                    return list_values
                        .get(index)
                        .map_or("", AsRef::<str>::as_ref)
                        .to_string();
                }

                if key == "server_name" {
                    return Args::get(args, &path_server_name, String::new());
                }
                if key == "message_thread_id" {
                    return Args::get::<i64, Value>(args, &path_thread_id, -1).to_string();
                }

                let mut current_value = args;
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
}

#[cfg(test)]
mod tests {
    use crate::format::formatting;
    use crate::model::CowStr;
    use serde_yaml::Value;

    #[test]
    fn test_no_placeholders() {
        let input = "Hello world";
        let args = Value::Null;
        let list_values: Vec<String> = vec![];

        let result = formatting::get_and_format(input, &args, &list_values);
        assert_eq!(result, CowStr::Owned("Hello world".to_string()));
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
        let list_values: Vec<String> = vec![];

        let result = formatting::get_and_format(input, &args, &list_values);
        assert_eq!(result, CowStr::Owned("Hello Alice".to_string()));
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
        let list_values: Vec<String> = vec![];

        let result = formatting::get_and_format(input, &args, &list_values);
        assert_eq!(result, CowStr::Owned("Hello, Bob!".to_string()));
    }

    #[test]
    fn test_missing_placeholder() {
        let input = "Hello {{name}}";
        let args = Value::Null;
        let list_values: Vec<String> = vec![];

        let result = formatting::get_and_format(input, &args, &list_values);
        assert_eq!(result, CowStr::Owned("Hello ".to_string()));
    }

    #[test]
    fn test_list_index_placeholder() {
        let input = "Item 0: {{0}}, Item 1: {{1}}";
        let args = Value::Null;
        let list_values = vec!["Apple".to_string(), "Banana".to_string()];

        let result = formatting::get_and_format(input, &args, &list_values);
        assert_eq!(
            result,
            CowStr::Owned("Item 0: Apple, Item 1: Banana".to_string())
        );
    }

    #[test]
    fn test_nested_placeholders() {
        let input = "{{user.name}} ({{user.id}})";
        let args = {
            Value::Mapping({
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
            })
        };
        let result = formatting::get_and_format(input, &args, &[] as &[&str]);
        assert_eq!(result, CowStr::Owned("Bob (789)".to_string()));
    }
}
