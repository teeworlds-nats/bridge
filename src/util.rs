use crate::model::CowStr;
use log::warn;
use regex::Captures;
use std::borrow::Cow;

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

pub fn escape_string(cow: CowStr) -> CowStr {
    if cow.contains(['"', '\'', '\\']) {
        let mut out = String::with_capacity(cow.len());
        for ch in cow.chars() {
            match ch {
                '\\' => out.push_str("\\\\"),
                '"' => out.push_str("\\\""),
                '\'' => out.push_str("\\'"),
                _ => out.push(ch),
            }
        }
        Cow::Owned(out)
    } else {
        cow
    }
}

pub fn captures_to_list<'a>(caps: &'a Captures<'a>) -> Vec<&'a str> {
    let mut out = Vec::with_capacity(caps.len());
    for cap in caps.iter().flatten() {
        out.push(cap.as_str());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_escaping_needed() {
        let input = CowStr::Owned("normal string".to_string());
        let result = escape_string(input);
        assert_eq!(result, CowStr::Owned("normal string".to_string()));
    }

    #[test]
    fn test_escape_single_quotes() {
        let input = CowStr::Owned("text with 'quotes'".to_string());
        let result = escape_string(input);
        assert_eq!(result, CowStr::Owned(r#"text with \'quotes\'"#.into()));
    }

    #[test]
    fn test_escape_all_special_chars() {
        let input = CowStr::Owned(r#"mixed \ '" all"#.to_string());
        let result = escape_string(input);
        assert_eq!(result, CowStr::Owned(r#"mixed \\ \'\" all"#.to_string()));
    }

    #[test]
    fn test_empty_string() {
        let input = CowStr::Owned("".to_string());
        let result = escape_string(input);
        assert_eq!(result, CowStr::Owned("".to_string()));
    }

    #[test]
    fn test_owned_string_no_escape() {
        let input = CowStr::Owned("hello".into());
        let result = escape_string(input);
        assert!(matches!(result, Cow::Owned(_)));
        assert_eq!(result.to_string(), "hello");
    }

    #[test]
    fn test_owned_string_with_escape() {
        let input = CowStr::Owned(r#"hello"world"#.into());
        let result = escape_string(input);
        assert!(matches!(result, Cow::Owned(_)));
        assert_eq!(result.to_string(), r#"hello\"world"#);
    }
}
