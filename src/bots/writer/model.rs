use bytes::Bytes;

pub struct TextBuilder {
    text: String,
}

impl TextBuilder {
    pub fn new() -> Self {
        Self {
            text: String::default(),
        }
    }

    pub fn new_with_text(text: String) -> Self {
        Self { text }
    }

    pub fn add(&mut self, text: String) {
        if self.text.is_empty() {
            self.text.push_str(&text)
        }

        self.text.push_str(&format!(";{}", text));
    }

    pub fn to_bytes(&self) -> Bytes {
        Bytes::from(self.text.clone())
    }
}
