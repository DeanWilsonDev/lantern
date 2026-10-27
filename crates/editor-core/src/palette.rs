//! The palette: what the game tells the editor about its content, as data.
//!
//! The editor never knows what a gem or a spawn point is. The game ships
//! `assets/editor/palette.json`, which lists:
//!
//! - **templates** for the "Add" menu: a label and a partial entity (kind, shape, material,
//!   collider, properties) that a new entity starts from;
//! - **markers**: how to draw an entity that has no shape, by kind (a gem mesh, a capsule...);
//! - **materials**: the names the inspector offers, each a texture under `assets/textures/`.
//!
//! A missing or broken palette is not fatal: [`Palette::fallback`] gives plain blocks, ramps
//! and markers so the editor still works on any world.

use std::collections::BTreeMap;
use std::path::Path;

use lantern_world_format::{ColliderKind, PropertyValue, Shape};
use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub struct Palette {
    pub templates: Vec<EntityTemplate>,
    #[serde(default)]
    pub markers: BTreeMap<String, MarkerStyle>,
    #[serde(default)]
    pub materials: Vec<String>,
}

/// A starting point for a new entity. Fields left out stay empty.
#[derive(Clone, Debug, Deserialize)]
pub struct EntityTemplate {
    /// The id the UI passes back to add one (`"block"`).
    pub id: String,
    /// What the Add menu shows (`"Block"`).
    pub label: String,
    pub kind: String,
    #[serde(default)]
    pub shape: Option<Shape>,
    #[serde(default)]
    pub material: Option<String>,
    #[serde(default)]
    pub tint: Option<String>,
    #[serde(default)]
    pub collider: Option<ColliderKind>,
    #[serde(default)]
    pub properties: BTreeMap<String, PropertyValue>,
    /// New entities are placed so their base sits on the surface under the cursor; this lifts
    /// them further (a gem floats above the ground).
    #[serde(default)]
    pub lift: f32,
}

/// The mesh drawn for a shapeless entity.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MarkerMesh {
    #[default]
    Cube,
    Sphere,
    Gem,
    /// A standing capsule with its base at the entity's origin (a character's feet).
    Capsule,
    /// A large flat square of lines at the entity's height, no mesh (a kill plane).
    Plane,
}

#[derive(Clone, Debug, Deserialize)]
pub struct MarkerStyle {
    #[serde(default)]
    pub mesh: MarkerMesh,
    /// `"#RRGGBB"`.
    #[serde(default = "default_marker_color")]
    pub color: String,
}

fn default_marker_color() -> String {
    "#D0D0E0".to_string()
}

impl Default for MarkerStyle {
    fn default() -> Self {
        MarkerStyle { mesh: MarkerMesh::Cube, color: default_marker_color() }
    }
}

impl Palette {
    pub fn load(path: &Path) -> Result<Palette, String> {
        let text = std::fs::read_to_string(path).map_err(|error| format!("{}: {error}", path.display()))?;
        serde_json::from_str(&text).map_err(|error| format!("{}: {error}", path.display()))
    }

    /// Generic content for when the game has no palette.
    pub fn fallback() -> Palette {
        let template = |id: &str, label: &str, shape: Option<Shape>| EntityTemplate {
            id: id.to_string(),
            label: label.to_string(),
            kind: id.to_string(),
            shape,
            material: None,
            tint: None,
            collider: shape.map(|_| ColliderKind::Static),
            properties: BTreeMap::new(),
            lift: 0.0,
        };
        Palette {
            templates: vec![
                template("block", "Block", Some(Shape::Box { half_extents: [1.0, 0.5, 1.0] })),
                template("ramp", "Ramp", Some(Shape::Ramp { width: 2.0, height: 1.0, length: 3.0 })),
                template("marker", "Marker", None),
            ],
            markers: BTreeMap::new(),
            materials: Vec::new(),
        }
    }

    pub fn template(&self, id: &str) -> Option<&EntityTemplate> {
        self.templates.iter().find(|template| template.id == id)
    }

    pub fn marker_style(&self, kind: &str) -> MarkerStyle {
        self.markers.get(kind).cloned().unwrap_or_default()
    }

    /// Every kind the palette mentions, for the inspector's kind choices.
    pub fn kinds(&self) -> Vec<String> {
        let mut kinds: Vec<String> = self.templates.iter().map(|template| template.kind.clone()).collect();
        kinds.extend(self.markers.keys().cloned());
        kinds.sort();
        kinds.dedup();
        kinds
    }
}
