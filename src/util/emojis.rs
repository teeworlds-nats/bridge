use std::collections::HashMap;
use std::fs;
use once_cell::sync::Lazy;

static TO_EMOJIES: Lazy<HashMap<char, String>> = Lazy::new(|| {
    let file_content = fs::read_to_string("emojis.txt").expect("Failed to read file: emojis.txt");
    let mut map = HashMap::new();
    for line in file_content.lines() {
        let mut parts = line.split(',');
        let emoji = parts.next().unwrap().chars().next().unwrap();
        let title = parts.next().unwrap().to_string(); // Convert to String
        map.insert(emoji, title);
    }
    map
});

/// Replaces text with emojis
#[allow(dead_code)]
pub fn replace_to_emoji(text: String) -> String {
    let mut result = text.to_string();
    for (orig, replace) in &*TO_EMOJIES {
        result = result.replace(*orig, replace);
    }
    result
}

/// Replaces emojis with text
pub fn replace_from_emoji(text: String) -> String {
    let mut result = text.to_string();
    for (orig, replace) in &*TO_EMOJIES {
        result = result.replace(replace, &orig.to_string());
    }
    result
}