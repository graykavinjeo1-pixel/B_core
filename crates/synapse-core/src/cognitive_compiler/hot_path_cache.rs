use std::collections::HashMap;

#[derive(Debug, Clone, Default)]
pub struct HotPathCache {
    paths: HashMap<String, String>,
}

impl HotPathCache {
    pub fn insert(&mut self, trigger: impl Into<String>, result: impl Into<String>) {
        self.paths.insert(trigger.into(), result.into());
    }

    pub fn get(&self, trigger: &str) -> Option<&str> {
        self.paths.get(trigger).map(String::as_str)
    }
}
