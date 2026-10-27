//! The serde types for a world file, plus loading, saving and validation.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt::{self, Write as _};
use std::path::{Path, PathBuf};

use glam::{EulerRot, Mat4, Quat, Vec3};
use lantern_core::color::Color;
use serde::{Deserialize, Serialize};

/// Bumped when a change to the file shape needs a migration. Loading refuses newer versions.
pub const CURRENT_FORMAT_VERSION: u32 = 1;

/// A whole world: what one `.world.json` file holds.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WorldFile {
    pub format_version: u32,
    pub name: String,
    pub entities: Vec<WorldEntity>,
}

/// An entity's id. Stable across saves, so game code, undo history and parent links can refer
/// to an entity even when it is renamed or reordered. Never reused within a file.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct EntityKey(pub u64);

impl fmt::Display for EntityKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "#{}", self.0)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WorldEntity {
    pub id: EntityKey,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<EntityKey>,
    /// What the entity is to the game, for example `"gem"`. See [`crate::kinds`].
    pub kind: String,
    #[serde(default)]
    pub transform: TransformData,
    /// A solid, drawn shape. Entities without one (spawn points, gems) are markers.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shape: Option<Shape>,
    /// Which surface texture a shape uses, for example `"ground"`, `"wall"` or `"platform"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub material: Option<String>,
    /// A colour multiplied into the shape's texture, as `"#RRGGBB"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tint: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub collider: Option<ColliderKind>,
    /// Game-specific values, interpreted only by game code.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub properties: BTreeMap<String, PropertyValue>,
}

/// A local transform: relative to the parent, or to the world when there is none.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct TransformData {
    pub position: [f32; 3],
    /// Euler angles in degrees, applied yaw (Y), then pitch (X), then roll (Z).
    #[serde(default)]
    pub rotation_degrees: [f32; 3],
    #[serde(default = "unit_scale")]
    pub scale: [f32; 3],
}

fn unit_scale() -> [f32; 3] {
    [1.0; 3]
}

impl Default for TransformData {
    fn default() -> TransformData {
        TransformData { position: [0.0; 3], rotation_degrees: [0.0; 3], scale: unit_scale() }
    }
}

impl TransformData {
    pub fn from_position(position: Vec3) -> TransformData {
        TransformData { position: position.to_array(), ..TransformData::default() }
    }

    pub fn position(&self) -> Vec3 {
        Vec3::from_array(self.position)
    }

    pub fn scale(&self) -> Vec3 {
        Vec3::from_array(self.scale)
    }

    pub fn rotation(&self) -> Quat {
        let [x, y, z] = self.rotation_degrees;
        Quat::from_euler(EulerRot::YXZ, y.to_radians(), x.to_radians(), z.to_radians())
    }

    /// `translation * rotation * scale`, the engine's composition order.
    pub fn matrix(&self) -> Mat4 {
        Mat4::from_scale_rotation_translation(self.scale(), self.rotation(), self.position())
    }
}

/// A solid shape, centred as the matching `lantern_renderer_3d::primitives` function centres it.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Shape {
    /// A box centred on the entity's position.
    Box { half_extents: [f32; 3] },
    /// A wedge whose footprint is centred on the entity's position, rising from height 0 at +Z
    /// to `height` at -Z (`primitives::ramp`).
    Ramp { width: f32, height: f32, length: f32 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ColliderKind {
    /// Never moves: its triangles go into the static collision grid.
    Static,
    /// Moved by game rules: a kinematic body the player can ride.
    Kinematic,
}

/// A game property. Untagged, so the file reads naturally: `true`, `2.5`, `"text"`, `[x, y, z]`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PropertyValue {
    Bool(bool),
    Number(f32),
    Text(String),
    Vector([f32; 3]),
}

#[derive(Debug)]
pub enum WorldFileError {
    Io { path: PathBuf, source: std::io::Error },
    Parse { path: PathBuf, source: serde_json::Error },
    Invalid { path: PathBuf, problems: Vec<String> },
}

impl fmt::Display for WorldFileError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WorldFileError::Io { path, source } => write!(formatter, "{}: {source}", path.display()),
            WorldFileError::Parse { path, source } => write!(formatter, "{}: {source}", path.display()),
            WorldFileError::Invalid { path, problems } => {
                write!(formatter, "{}: {} problem(s): {}", path.display(), problems.len(), problems.join("; "))
            }
        }
    }
}

impl std::error::Error for WorldFileError {}

impl WorldFile {
    pub fn new(name: impl Into<String>) -> WorldFile {
        WorldFile { format_version: CURRENT_FORMAT_VERSION, name: name.into(), entities: Vec::new() }
    }

    /// Parses and validates a world from JSON text. `path` is only used in error messages.
    pub fn from_json(text: &str, path: &Path) -> Result<WorldFile, WorldFileError> {
        let world: WorldFile =
            serde_json::from_str(text).map_err(|source| WorldFileError::Parse { path: path.to_path_buf(), source })?;
        let problems = world.validate();
        if problems.is_empty() {
            Ok(world)
        } else {
            Err(WorldFileError::Invalid { path: path.to_path_buf(), problems })
        }
    }

    pub fn load(path: &Path) -> Result<WorldFile, WorldFileError> {
        let text =
            std::fs::read_to_string(path).map_err(|source| WorldFileError::Io { path: path.to_path_buf(), source })?;
        WorldFile::from_json(&text, path)
    }

    /// Writes the world atomically: to a temporary file beside `path`, then renamed over it, so
    /// a crash mid-save never leaves a half-written world.
    pub fn save(&self, path: &Path) -> Result<(), WorldFileError> {
        let problems = self.validate();
        if !problems.is_empty() {
            return Err(WorldFileError::Invalid { path: path.to_path_buf(), problems });
        }
        let io_error = |source| WorldFileError::Io { path: path.to_path_buf(), source };
        let temporary_path = path.with_extension("json.saving");
        std::fs::write(&temporary_path, self.to_json()).map_err(io_error)?;
        std::fs::rename(&temporary_path, path).map_err(io_error)
    }

    /// Pretty JSON with short numeric arrays kept on one line (`"position": [0, 1.5, -3]`), so a
    /// world file diffs one value per line.
    pub fn to_json(&self) -> String {
        let value = serde_json::to_value(self).expect("world types always serialize");
        let mut text = String::new();
        write_json_value(&mut text, &value, 0);
        text.push('\n');
        text
    }

    /// Every problem that would make the world unusable. Empty means valid.
    pub fn validate(&self) -> Vec<String> {
        let mut problems = Vec::new();
        if self.format_version > CURRENT_FORMAT_VERSION {
            problems.push(format!(
                "format_version {} is newer than this build understands ({CURRENT_FORMAT_VERSION})",
                self.format_version
            ));
        }
        let mut seen = HashSet::new();
        for entity in &self.entities {
            if !seen.insert(entity.id) {
                problems.push(format!("entity id {} is used more than once", entity.id));
            }
            if entity.kind.trim().is_empty() {
                problems.push(format!("entity {} '{}' has an empty kind", entity.id, entity.name));
            }
            let all_finite = entity
                .transform
                .position
                .iter()
                .chain(&entity.transform.rotation_degrees)
                .chain(&entity.transform.scale)
                .all(|value| value.is_finite());
            if !all_finite {
                problems.push(format!("entity {} '{}' has a non-finite transform", entity.id, entity.name));
            }
            if entity.transform.scale.iter().any(|&value| value.abs() < 1e-6) {
                problems.push(format!("entity {} '{}' has a zero scale", entity.id, entity.name));
            }
            if let Some(shape) = entity.shape {
                if !shape.is_valid() {
                    problems
                        .push(format!("entity {} '{}' has a shape with a non-positive size", entity.id, entity.name));
                }
            }
            if let Some(tint) = &entity.tint {
                if parse_hex_color(tint).is_none() {
                    problems.push(format!(
                        "entity {} '{}' has tint '{tint}', expected \"#RRGGBB\"",
                        entity.id, entity.name
                    ));
                }
            }
        }
        let parents: HashMap<EntityKey, Option<EntityKey>> =
            self.entities.iter().map(|entity| (entity.id, entity.parent)).collect();
        for entity in &self.entities {
            let Some(parent) = entity.parent else { continue };
            if !parents.contains_key(&parent) {
                problems.push(format!("entity {} '{}' has missing parent {parent}", entity.id, entity.name));
                continue;
            }
            // Walk up; more steps than entities means a cycle.
            let mut current = Some(parent);
            let mut steps = 0;
            while let Some(key) = current {
                steps += 1;
                if key == entity.id || steps > self.entities.len() {
                    problems.push(format!("entity {} '{}' is its own ancestor", entity.id, entity.name));
                    break;
                }
                current = parents.get(&key).copied().flatten();
            }
        }
        problems
    }

    pub fn entity(&self, id: EntityKey) -> Option<&WorldEntity> {
        self.entities.iter().find(|entity| entity.id == id)
    }

    pub fn entity_mut(&mut self, id: EntityKey) -> Option<&mut WorldEntity> {
        self.entities.iter_mut().find(|entity| entity.id == id)
    }

    /// Entities whose `kind` matches, in file order.
    pub fn entities_of_kind<'a>(&'a self, kind: &'a str) -> impl Iterator<Item = &'a WorldEntity> + 'a {
        self.entities.iter().filter(move |entity| entity.kind == kind)
    }

    /// One more than the largest id in use, so a new entity never reuses an id.
    pub fn next_entity_key(&self) -> EntityKey {
        EntityKey(self.entities.iter().map(|entity| entity.id.0).max().map_or(1, |largest| largest + 1))
    }
}

impl Shape {
    pub fn is_valid(&self) -> bool {
        match *self {
            Shape::Box { half_extents } => half_extents.iter().all(|&value| value > 0.0 && value.is_finite()),
            Shape::Ramp { width, height, length } => {
                [width, height, length].iter().all(|&value| value > 0.0 && value.is_finite())
            }
        }
    }
}

impl WorldEntity {
    /// The tint as a colour; white when unset or malformed.
    pub fn tint_color(&self) -> Color {
        self.tint.as_deref().and_then(parse_hex_color).unwrap_or(Color::WHITE)
    }

    pub fn number_property(&self, key: &str) -> Option<f32> {
        match self.properties.get(key) {
            Some(PropertyValue::Number(value)) => Some(*value),
            _ => None,
        }
    }

    pub fn text_property(&self, key: &str) -> Option<&str> {
        match self.properties.get(key) {
            Some(PropertyValue::Text(value)) => Some(value),
            _ => None,
        }
    }
}

/// `"#RRGGBB"` → colour.
pub fn parse_hex_color(text: &str) -> Option<Color> {
    let digits = text.strip_prefix('#')?;
    if digits.len() != 6 {
        return None;
    }
    u32::from_str_radix(digits, 16).ok().map(Color::from_hex)
}

fn write_indent(text: &mut String, depth: usize) {
    for _ in 0..depth {
        text.push_str("  ");
    }
}

/// Numbers are written through `f32`'s shortest round-trip form, since every float in a world
/// file is an `f32` (serde_json's `Value` widens to `f64`, which would print `0.30000001`).
fn write_json_number(text: &mut String, number: &serde_json::Number) {
    if let Some(integer) = number.as_u64() {
        let _ = write!(text, "{integer}");
    } else if let Some(integer) = number.as_i64() {
        let _ = write!(text, "{integer}");
    } else {
        let value = number.as_f64().unwrap_or(0.0) as f32;
        if value == value.trunc() && value.abs() < 1e7 {
            let _ = write!(text, "{}", value as i64);
        } else {
            let _ = write!(text, "{value}");
        }
    }
}

fn write_json_value(text: &mut String, value: &serde_json::Value, depth: usize) {
    use serde_json::Value;
    match value {
        Value::Null => text.push_str("null"),
        Value::Bool(flag) => text.push_str(if *flag { "true" } else { "false" }),
        Value::Number(number) => write_json_number(text, number),
        Value::String(string) => text.push_str(&serde_json::to_string(string).expect("strings serialize")),
        Value::Array(items) if items.iter().all(Value::is_number) => {
            text.push('[');
            for (index, item) in items.iter().enumerate() {
                if index > 0 {
                    text.push_str(", ");
                }
                write_json_value(text, item, depth);
            }
            text.push(']');
        }
        Value::Array(items) => {
            if items.is_empty() {
                text.push_str("[]");
                return;
            }
            text.push_str("[\n");
            for (index, item) in items.iter().enumerate() {
                write_indent(text, depth + 1);
                write_json_value(text, item, depth + 1);
                text.push_str(if index + 1 < items.len() { ",\n" } else { "\n" });
            }
            write_indent(text, depth);
            text.push(']');
        }
        Value::Object(fields) => {
            if fields.is_empty() {
                text.push_str("{}");
                return;
            }
            text.push_str("{\n");
            for (index, (key, field)) in fields.iter().enumerate() {
                write_indent(text, depth + 1);
                text.push_str(&serde_json::to_string(key).expect("strings serialize"));
                text.push_str(": ");
                write_json_value(text, field, depth + 1);
                text.push_str(if index + 1 < fields.len() { ",\n" } else { "\n" });
            }
            write_indent(text, depth);
            text.push('}');
        }
    }
}
