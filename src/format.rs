use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {
    static ref RE: Regex = Regex::new(r"\{\{([^\}]+)\}\}").unwrap();
}

pub mod format {
    use crate::args::Args;
    use crate::format::RE;
    use crate::model::CowString;
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
            $crate::format::format::format($input, $args, $list, Vec::new())
        };

        // with_default: (input, args, list_values, default_vec)
        ($input:expr, $args:expr, $list:expr, $default:expr) => {
            $crate::format::format::format($input, $args, $list, $default)
        };

        // single: (input, args, list_values)
        ($input:expr, $args:expr, $list:expr; single) => {{
            $crate::format::format::get_and_format(&$input, $args, $list)
        }};

        // single with default: (input, args, list_values, default)
        ($input:expr, $args:expr, $list:expr, $default:expr; single) => {{
            $crate::format::format::get_and_format(&$input.unwrap_or($default), $args, $list)
        }};
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

    pub fn get_and_format(
        string: &str,
        args: &Value,
        list_values: &[String],
    ) -> CowString<'static> {
        if !string.contains("{{") {
            return CowString::Owned(string.to_string());
        }

        let new_args = {
            let mut new_args = args.clone();

            let path_server_name = Args::get(args, "path_server_name", "server_name".to_string());
            let path_thread_id = Args::get(args, "path_thread_id", "message_thread_id".to_string());

            new_args["server_name"] =
                Value::String(Args::get(args, &path_server_name, "".to_string()));
            new_args["message_thread_id"] =
                Value::from(Args::get::<i64, Value>(args, &path_thread_id, -1));

            new_args
        };

        CowString::Owned(
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
}

#[cfg(test)]
mod tests {
    use crate::format::format;
    use crate::model::CowString;
    use serde_yaml::Value;

    #[test]
    fn test_no_placeholders() {
        let input = "Hello world";
        let args = Value::Null;
        let list_values = vec![];

        let result = format::get_and_format(input, &args, &list_values);
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

        let result = format::get_and_format(input, &args, &list_values);
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

        let result = format::get_and_format(input, &args, &list_values);
        assert_eq!(result, CowString::Owned("Hello, Bob!".to_string()));
    }

    #[test]
    fn test_missing_placeholder() {
        let input = "Hello {{name}}";
        let args = Value::Null;
        let list_values = vec![];

        let result = format::get_and_format(input, &args, &list_values);
        assert_eq!(result, CowString::Owned("Hello ".to_string()));
    }

    #[test]
    fn test_list_index_placeholder() {
        let input = "Item 0: {{0}}, Item 1: {{1}}";
        let args = Value::Null;
        let list_values = vec!["Apple".to_string(), "Banana".to_string()];

        let result = format::get_and_format(input, &args, &list_values);
        assert_eq!(
            result,
            CowString::Owned("Item 0: Apple, Item 1: Banana".to_string())
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
        let result = format::get_and_format(input, &args, &[]);
        assert_eq!(result, CowString::Owned("Bob (789)".to_string()));
    }
}
