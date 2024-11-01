use std::collections::HashMap;
use std::fs;
use once_cell::sync::Lazy;

static TO_EMOJIES: Lazy<HashMap<char, String>> = Lazy::new(|| {
    // TODO: https://crates.io/crates/tap
    let lines = fs::read_to_string("emojis.txt")
        .expect("Failed to read file: emojis.txt");
    let map: HashMap<char, String> = lines.lines()
        .flat_map(|line| {
            let mut parts = line.split(',');
            let emoji = parts.next().and_then(|s| s.chars().next());
            let title = parts.next().map(|s| s.to_string());

            emoji.zip(title) // Соединяем emoji и title в кортеж
        })
        .map(|(emoji, title)| (emoji, title.to_uppercase())) // Пример преобразования заголовка
        .collect::<HashMap<_, _>>();
    map
});

/// Replaces text with emojis
#[allow(dead_code)]
pub fn replace_to_emoji(text: String) -> String {
    TO_EMOJIES.iter().fold(text, |result, (orig, replace)| {
        result.replace(*orig, replace)
    })
}

/// Replaces emojis with text
pub fn replace_from_emoji(text: String) -> String {
    // Используем fold для замены
    TO_EMOJIES.iter().fold(text, |result, (orig, replace)| {
        result.replace(replace, &orig.to_string())
    })
}
