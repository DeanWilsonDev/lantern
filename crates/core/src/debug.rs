//! Debug values: a path-keyed list of named values that any system can publish
//! (`"Player/Position"`).
//!
//! Entries keep the order they were first set in, so the
//! HUD doesn't reshuffle between frames.

use glam::Vec3;
use std::collections::HashMap;
use std::fmt;

#[derive(Clone, Debug, PartialEq)]
pub enum DebugValue {
    Text(String),
    Number(f64),
    Integer(i64),
    Bool(bool),
    Vector(Vec3),
}

impl fmt::Display for DebugValue {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DebugValue::Text(text) => formatter.write_str(text),
            DebugValue::Number(number) => write!(formatter, "{number:.2}"),
            DebugValue::Integer(integer) => write!(formatter, "{integer}"),
            DebugValue::Bool(value) => formatter.write_str(if *value { "yes" } else { "no" }),
            DebugValue::Vector(vector) => write!(formatter, "{:6.2} {:6.2} {:6.2}", vector.x, vector.y, vector.z),
        }
    }
}

impl From<&str> for DebugValue {
    fn from(text: &str) -> Self {
        DebugValue::Text(text.to_string())
    }
}
impl From<String> for DebugValue {
    fn from(text: String) -> Self {
        DebugValue::Text(text)
    }
}
impl From<f64> for DebugValue {
    fn from(number: f64) -> Self {
        DebugValue::Number(number)
    }
}
impl From<f32> for DebugValue {
    fn from(number: f32) -> Self {
        DebugValue::Number(f64::from(number))
    }
}
impl From<i64> for DebugValue {
    fn from(integer: i64) -> Self {
        DebugValue::Integer(integer)
    }
}
impl From<u64> for DebugValue {
    fn from(integer: u64) -> Self {
        DebugValue::Integer(integer as i64)
    }
}
impl From<u32> for DebugValue {
    fn from(integer: u32) -> Self {
        DebugValue::Integer(i64::from(integer))
    }
}
impl From<usize> for DebugValue {
    fn from(integer: usize) -> Self {
        DebugValue::Integer(integer as i64)
    }
}
impl From<bool> for DebugValue {
    fn from(value: bool) -> Self {
        DebugValue::Bool(value)
    }
}
impl From<Vec3> for DebugValue {
    fn from(vector: Vec3) -> Self {
        DebugValue::Vector(vector)
    }
}

#[derive(Clone, Debug)]
pub struct DebugEntry {
    /// `"Group/Name"`. The part before the first `/` is the HUD section.
    pub path: String,
    pub value: DebugValue,
}

impl DebugEntry {
    pub fn group(&self) -> &str {
        self.path.split_once('/').map_or("", |(group, _)| group)
    }

    pub fn name(&self) -> &str {
        self.path.split_once('/').map_or(self.path.as_str(), |(_, name)| name)
    }
}

#[derive(Default, Debug)]
pub struct DebugValues {
    entries: Vec<DebugEntry>,
    index_by_path: HashMap<String, usize>,
}

impl DebugValues {
    pub fn set(&mut self, path: &str, value: impl Into<DebugValue>) {
        let value = value.into();
        if let Some(&index) = self.index_by_path.get(path) {
            self.entries[index].value = value;
            return;
        }
        self.index_by_path.insert(path.to_string(), self.entries.len());
        self.entries.push(DebugEntry { path: path.to_string(), value });
    }

    pub fn get(&self, path: &str) -> Option<&DebugValue> {
        self.index_by_path.get(path).map(|&index| &self.entries[index].value)
    }

    /// Removes every entry whose path starts with `prefix`.
    pub fn remove_prefix(&mut self, prefix: &str) {
        self.entries.retain(|entry| !entry.path.starts_with(prefix));
        self.index_by_path =
            self.entries.iter().enumerate().map(|(index, entry)| (entry.path.clone(), index)).collect();
    }

    pub fn entries(&self) -> &[DebugEntry] {
        &self.entries
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.index_by_path.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entries_keep_first_insertion_order_and_update_in_place() {
        let mut values = DebugValues::default();
        values.set("Frame/FPS", 60.0);
        values.set("Player/Grounded", true);
        values.set("Frame/FPS", 59.0);
        let paths: Vec<&str> = values.entries().iter().map(|entry| entry.path.as_str()).collect();
        assert_eq!(paths, ["Frame/FPS", "Player/Grounded"]);
        assert_eq!(values.get("Frame/FPS"), Some(&DebugValue::Number(59.0)));
        assert_eq!(values.entries()[1].group(), "Player");
        assert_eq!(values.entries()[1].name(), "Grounded");
    }

    #[test]
    fn remove_prefix_reindexes() {
        let mut values = DebugValues::default();
        values.set("Script/Error", "x");
        values.set("Frame/FPS", 1.0);
        values.remove_prefix("Script/");
        values.set("Frame/FPS", 2.0);
        assert_eq!(values.entries().len(), 1);
        assert_eq!(values.get("Frame/FPS"), Some(&DebugValue::Number(2.0)));
    }
}
