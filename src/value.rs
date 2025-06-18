pub trait ValueExt {
    fn get(&self, index: &str) -> Option<&Self>;
    fn as_str(&self) -> Option<&str>;
    #[warn(dead_code)]
    fn as_i64(&self) -> Option<i64>;
    #[warn(dead_code)]
    fn as_bool(&self) -> Option<bool>;
    #[warn(dead_code)]
    fn as_f64(&self) -> Option<f64>;
}

impl ValueExt for serde_json::Value {
    fn get(&self, index: &str) -> Option<&Self> {
        self.get(index)
    }
    fn as_str(&self) -> Option<&str> {
        self.as_str()
    }
    fn as_i64(&self) -> Option<i64> {
        self.as_i64()
    }
    fn as_bool(&self) -> Option<bool> {
        self.as_bool()
    }

    fn as_f64(&self) -> Option<f64> {
        self.as_f64()
    }
}

impl ValueExt for serde_yaml::Value {
    fn get(&self, index: &str) -> Option<&Self> {
        self.get(index)
    }
    fn as_str(&self) -> Option<&str> {
        self.as_str()
    }
    fn as_i64(&self) -> Option<i64> {
        self.as_i64()
    }
    fn as_bool(&self) -> Option<bool> {
        self.as_bool()
    }

    fn as_f64(&self) -> Option<f64> {
        self.as_f64()
    }
}
