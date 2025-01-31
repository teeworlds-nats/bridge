use crate::model::RegexData;
use once_cell::sync::Lazy;

pub static DEFAULT_REGEX: Lazy<Vec<RegexData>> = Lazy::new(|| {
    vec![
        RegexData::new(
            "DDNet".to_string(),
            r"^(\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}) (I|E) ([a-z]+): (.*)".to_string(),
        ),
        RegexData::new(
            "Teeworlds".to_string(),
            r"^\[(\w+)\]: (.*)".to_string(),
        ),
    ]
});
