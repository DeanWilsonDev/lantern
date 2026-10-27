//! Editor preferences that outlive a session: window size, panel splits, snapping, view options,
//! recent files. A flat map of strings, saved as JSON in the user's config directory, so the
//! front end can keep whatever it likes under its own keys without a schema change here.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EditorPreferences {
    values: BTreeMap<String, String>,
}

impl EditorPreferences {
    /// `$XDG_CONFIG_HOME/lantern/world-editor.json`, `%APPDATA%\lantern\world-editor.json` or
    /// `~/.config/lantern/world-editor.json`; `None` when no home is known.
    pub fn default_path() -> Option<PathBuf> {
        let base = std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("APPDATA").map(PathBuf::from))
            .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
            .or_else(|| std::env::var_os("USERPROFILE").map(|home| PathBuf::from(home).join(".config")))?;
        Some(base.join("lantern").join("world-editor.json"))
    }

    pub fn load(path: &Path) -> Result<EditorPreferences, String> {
        let text = std::fs::read_to_string(path).map_err(|error| format!("{}: {error}", path.display()))?;
        let values: BTreeMap<String, String> =
            serde_json::from_str(&text).map_err(|error| format!("{}: {error}", path.display()))?;
        Ok(EditorPreferences { values })
    }

    pub fn save(&self, path: &Path) -> Result<(), String> {
        if let Some(directory) = path.parent() {
            std::fs::create_dir_all(directory).map_err(|error| format!("{}: {error}", directory.display()))?;
        }
        let text = serde_json::to_string_pretty(&self.values).map_err(|error| error.to_string())?;
        std::fs::write(path, text + "\n").map_err(|error| format!("{}: {error}", path.display()))
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.values.get(key).map(String::as_str)
    }

    /// Sets a value; an empty value removes the key. Returns whether anything changed.
    pub fn set(&mut self, key: &str, value: &str) -> bool {
        if value.is_empty() {
            return self.values.remove(key).is_some();
        }
        self.values.insert(key.to_string(), value.to_string()).as_deref() != Some(value)
    }

    pub fn keys(&self) -> impl Iterator<Item = &str> {
        self.values.keys().map(String::as_str)
    }

    /// Reads a list stored as newline-separated text.
    pub fn list(&self, key: &str) -> Vec<String> {
        self.get(key)
            .map_or_else(Vec::new, |text| text.lines().filter(|line| !line.is_empty()).map(str::to_string).collect())
    }

    pub fn set_list(&mut self, key: &str, items: &[String]) -> bool {
        self.set(key, &items.join("\n"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preferences_round_trip_through_json() {
        let directory = std::env::temp_dir().join(format!("lantern-editor-preferences-{}", std::process::id()));
        let path = directory.join("nested").join("world-editor.json");
        let mut preferences = EditorPreferences::default();
        assert!(preferences.set("window.width", "1600"));
        assert!(!preferences.set("window.width", "1600"));
        assert!(preferences.set_list("recent", &["a.json".to_string(), "b.json".to_string()]));
        preferences.save(&path).unwrap();
        let loaded = EditorPreferences::load(&path).unwrap();
        assert_eq!(loaded, preferences);
        assert_eq!(loaded.list("recent"), vec!["a.json", "b.json"]);
        assert_eq!(loaded.get("window.width"), Some("1600"));
        std::fs::remove_dir_all(&directory).unwrap();
    }
}
