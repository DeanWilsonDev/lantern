//! The inspector as data: the fields an entity shows, and turning typed text back into an
//! edited entity. The front end only lays the fields out; every value crosses as text, and all
//! parsing and validation happens here, so the UI language needs no number parsing.
//!
//! Field keys are stable strings the UI passes back: `name`, `kind`, `position.x`,
//! `rotation.y`, `scale.z`, `shape`, `size.x` (a box's full size), `width`/`height`/`length`
//! (a ramp), `material`, `tint`, `collider`, and `property:<name>` for game properties.

use lantern_world_format::{ColliderKind, PropertyValue, Shape, WorldEntity, WorldFile};

use crate::palette::Palette;

#[derive(Clone, Debug, PartialEq)]
pub enum FieldKind {
    Text,
    Number,
    /// One of a fixed list. The list may include the current value even if unusual.
    Choice(Vec<String>),
    ReadOnly,
}

impl FieldKind {
    pub fn name(&self) -> &'static str {
        match self {
            FieldKind::Text => "text",
            FieldKind::Number => "number",
            FieldKind::Choice(_) => "choice",
            FieldKind::ReadOnly => "readonly",
        }
    }

    pub fn choices(&self) -> &[String] {
        match self {
            FieldKind::Choice(choices) => choices,
            _ => &[],
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct InspectorField {
    pub key: String,
    pub label: String,
    /// Section heading: Entity, Transform, Shape, Look, Collision, Properties.
    pub group: &'static str,
    pub value: String,
    pub kind: FieldKind,
}

/// Formats a number the way the inspector shows it: shortest form, no trailing zeros.
pub fn format_number(value: f32) -> String {
    let rounded = (value * 10_000.0).round() / 10_000.0;
    if rounded == 0.0 { "0".to_string() } else { format!("{rounded}") }
}

fn format_vector(value: [f32; 3]) -> String {
    format!("{}, {}, {}", format_number(value[0]), format_number(value[1]), format_number(value[2]))
}

pub fn format_property(value: &PropertyValue) -> String {
    match value {
        PropertyValue::Bool(flag) => flag.to_string(),
        PropertyValue::Number(number) => format_number(*number),
        PropertyValue::Text(text) => text.clone(),
        PropertyValue::Vector(vector) => format_vector(*vector),
    }
}

/// Reads a property from text: `true`/`false`, a number, three comma-separated numbers, or
/// otherwise text.
pub fn parse_property(text: &str) -> PropertyValue {
    let trimmed = text.trim();
    match trimmed {
        "true" => return PropertyValue::Bool(true),
        "false" => return PropertyValue::Bool(false),
        _ => {}
    }
    if let Ok(number) = trimmed.parse::<f32>() {
        if number.is_finite() {
            return PropertyValue::Number(number);
        }
    }
    let parts: Vec<&str> = trimmed.trim_start_matches('[').trim_end_matches(']').split(',').collect();
    if parts.len() == 3 {
        let numbers: Vec<f32> = parts.iter().filter_map(|part| part.trim().parse::<f32>().ok()).collect();
        if numbers.len() == 3 && numbers.iter().all(|number| number.is_finite()) {
            return PropertyValue::Vector([numbers[0], numbers[1], numbers[2]]);
        }
    }
    PropertyValue::Text(text.to_string())
}

fn parse_number(text: &str) -> Result<f32, String> {
    let value: f32 = text.trim().parse().map_err(|_| format!("'{}' is not a number", text.trim()))?;
    if value.is_finite() { Ok(value) } else { Err(format!("'{}' is not a finite number", text.trim())) }
}

fn shape_name(shape: Option<Shape>) -> &'static str {
    match shape {
        None => "none",
        Some(Shape::Box { .. }) => "box",
        Some(Shape::Ramp { .. }) => "ramp",
    }
}

fn collider_name(collider: Option<ColliderKind>) -> &'static str {
    match collider {
        None => "none",
        Some(ColliderKind::Static) => "static",
        Some(ColliderKind::Kinematic) => "kinematic",
    }
}

const AXES: [&str; 3] = ["x", "y", "z"];

fn with_current(mut choices: Vec<String>, current: &str) -> Vec<String> {
    if !current.is_empty() && !choices.iter().any(|choice| choice == current) {
        choices.push(current.to_string());
    }
    choices
}

/// The fields `entity` shows, in display order.
pub fn fields_for(entity: &WorldEntity, world: &WorldFile, palette: &Palette) -> Vec<InspectorField> {
    let mut fields = Vec::new();
    let mut push = |key: String, label: &str, group: &'static str, value: String, kind: FieldKind| {
        fields.push(InspectorField { key, label: label.to_string(), group, value, kind });
    };
    push("name".into(), "Name", "Entity", entity.name.clone(), FieldKind::Text);
    push(
        "kind".into(),
        "Kind",
        "Entity",
        entity.kind.clone(),
        FieldKind::Choice(with_current(palette.kinds(), &entity.kind)),
    );
    let parent_name = entity
        .parent
        .and_then(|parent| world.entity(parent))
        .map_or_else(|| "none".to_string(), |parent| parent.name.clone());
    push("parent".into(), "Parent", "Entity", parent_name, FieldKind::ReadOnly);
    push("id".into(), "Id", "Entity", entity.id.0.to_string(), FieldKind::ReadOnly);

    let transform = &entity.transform;
    for (group_key, label, values) in [
        ("position", "Position", transform.position),
        ("rotation", "Rotation", transform.rotation_degrees),
        ("scale", "Scale", transform.scale),
    ] {
        for (index, axis) in AXES.iter().enumerate() {
            push(
                format!("{group_key}.{axis}"),
                &format!("{label} {}", axis.to_uppercase()),
                "Transform",
                format_number(values[index]),
                FieldKind::Number,
            );
        }
    }

    let shapes = ["none", "box", "ramp"].map(String::from).to_vec();
    push("shape".into(), "Shape", "Shape", shape_name(entity.shape).into(), FieldKind::Choice(shapes));
    match entity.shape {
        Some(Shape::Box { half_extents }) => {
            for (index, axis) in AXES.iter().enumerate() {
                push(
                    format!("size.{axis}"),
                    &format!("Size {}", axis.to_uppercase()),
                    "Shape",
                    format_number(half_extents[index] * 2.0),
                    FieldKind::Number,
                );
            }
        }
        Some(Shape::Ramp { width, height, length }) => {
            push("width".into(), "Width", "Shape", format_number(width), FieldKind::Number);
            push("height".into(), "Height", "Shape", format_number(height), FieldKind::Number);
            push("length".into(), "Length", "Shape", format_number(length), FieldKind::Number);
        }
        None => {}
    }

    if entity.shape.is_some() {
        let material = entity.material.clone().unwrap_or_default();
        let mut materials = vec!["none".to_string()];
        materials.extend(palette.materials.iter().cloned());
        push(
            "material".into(),
            "Material",
            "Look",
            if material.is_empty() { "none".into() } else { material.clone() },
            FieldKind::Choice(with_current(materials, &material)),
        );
        push("tint".into(), "Tint", "Look", entity.tint.clone().unwrap_or_default(), FieldKind::Text);
        let colliders = ["none", "static", "kinematic"].map(String::from).to_vec();
        push(
            "collider".into(),
            "Collider",
            "Collision",
            collider_name(entity.collider).into(),
            FieldKind::Choice(colliders),
        );
    }

    for (key, value) in &entity.properties {
        push(format!("property:{key}"), key, "Properties", format_property(value), FieldKind::Text);
    }
    fields
}

/// `entity` with `field` set from `text`. Returns an error message for bad input; the entity
/// is unchanged in that case.
pub fn apply_field(entity: &WorldEntity, field: &str, text: &str) -> Result<WorldEntity, String> {
    let mut edited = entity.clone();
    let axis_index =
        |suffix: &str| AXES.iter().position(|axis| *axis == suffix).ok_or(format!("unknown field '{field}'"));
    if let Some(property) = field.strip_prefix("property:") {
        if property.trim().is_empty() {
            return Err("a property needs a name".to_string());
        }
        if text.trim().is_empty() {
            edited.properties.remove(property);
        } else {
            edited.properties.insert(property.to_string(), parse_property(text));
        }
        return Ok(edited);
    }
    if let Some((group, suffix)) = field.split_once('.') {
        let index = axis_index(suffix)?;
        let value = parse_number(text)?;
        match group {
            "position" => edited.transform.position[index] = value,
            "rotation" => edited.transform.rotation_degrees[index] = value,
            "scale" => {
                if value.abs() < 1e-4 {
                    return Err("scale cannot be zero".to_string());
                }
                edited.transform.scale[index] = value;
            }
            "size" => match &mut edited.shape {
                Some(Shape::Box { half_extents }) => {
                    if value <= 0.0 {
                        return Err("size must be positive".to_string());
                    }
                    half_extents[index] = value * 0.5;
                }
                _ => return Err("only a box has a size".to_string()),
            },
            _ => return Err(format!("unknown field '{field}'")),
        }
        return Ok(edited);
    }
    match field {
        "name" => {
            if text.trim().is_empty() {
                return Err("a name cannot be empty".to_string());
            }
            edited.name = text.trim().to_string();
        }
        "kind" => {
            if text.trim().is_empty() {
                return Err("a kind cannot be empty".to_string());
            }
            edited.kind = text.trim().to_string();
        }
        "shape" => {
            edited.shape = match text.trim() {
                "none" => None,
                "box" => Some(match entity.shape {
                    Some(Shape::Box { .. }) => entity.shape.expect("matched"),
                    _ => Shape::Box { half_extents: [0.5; 3] },
                }),
                "ramp" => Some(match entity.shape {
                    Some(Shape::Ramp { .. }) => entity.shape.expect("matched"),
                    _ => Shape::Ramp { width: 2.0, height: 1.0, length: 3.0 },
                }),
                other => return Err(format!("unknown shape '{other}'")),
            };
            if edited.shape.is_none() {
                edited.material = None;
                edited.tint = None;
                edited.collider = None;
            } else if entity.shape.is_none() {
                edited.collider = Some(ColliderKind::Static);
            }
        }
        "width" | "height" | "length" => {
            let value = parse_number(text)?;
            if value <= 0.0 {
                return Err(format!("{field} must be positive"));
            }
            match &mut edited.shape {
                Some(Shape::Ramp { width, height, length }) => match field {
                    "width" => *width = value,
                    "height" => *height = value,
                    _ => *length = value,
                },
                _ => return Err("only a ramp has a width, height and length".to_string()),
            }
        }
        "material" => {
            let name = text.trim();
            edited.material = if name.is_empty() || name == "none" { None } else { Some(name.to_string()) };
        }
        "tint" => {
            let tint = text.trim();
            if tint.is_empty() {
                edited.tint = None;
            } else {
                let normalised =
                    if tint.starts_with('#') { tint.to_uppercase() } else { format!("#{}", tint.to_uppercase()) };
                if lantern_world_format::parse_hex_color(&normalised).is_none() {
                    return Err(format!("'{tint}' is not a colour; use #RRGGBB"));
                }
                edited.tint = Some(normalised);
            }
        }
        "collider" => {
            edited.collider = match text.trim() {
                "none" => None,
                "static" => Some(ColliderKind::Static),
                "kinematic" => Some(ColliderKind::Kinematic),
                other => return Err(format!("unknown collider '{other}'")),
            };
        }
        "parent" | "id" => return Err(format!("{field} cannot be edited here")),
        _ => return Err(format!("unknown field '{field}'")),
    }
    Ok(edited)
}

#[cfg(test)]
mod tests {
    use super::*;
    use lantern_world_format::{EntityKey, TransformData};

    fn block() -> WorldEntity {
        WorldEntity {
            id: EntityKey(7),
            name: "wall".into(),
            parent: None,
            kind: "block".into(),
            transform: TransformData::default(),
            shape: Some(Shape::Box { half_extents: [1.0, 0.5, 2.0] }),
            material: Some("wall".into()),
            tint: None,
            collider: Some(ColliderKind::Static),
            properties: Default::default(),
        }
    }

    #[test]
    fn fields_show_sizes_as_full_extents() {
        let entity = block();
        let world = WorldFile::new("w");
        let fields = fields_for(&entity, &world, &Palette::fallback());
        let size_z = fields.iter().find(|field| field.key == "size.z").unwrap();
        assert_eq!(size_z.value, "4");
        assert!(fields.iter().any(|field| field.key == "collider" && field.value == "static"));
    }

    #[test]
    fn editing_fields_validates_input() {
        let entity = block();
        assert_eq!(apply_field(&entity, "position.y", "2.5").unwrap().transform.position[1], 2.5);
        assert!(apply_field(&entity, "position.y", "abc").is_err());
        assert!(apply_field(&entity, "scale.x", "0").is_err());
        assert_eq!(
            apply_field(&entity, "size.x", "3").unwrap().shape,
            Some(Shape::Box { half_extents: [1.5, 0.5, 2.0] })
        );
        assert_eq!(apply_field(&entity, "tint", "ffe0a0").unwrap().tint.as_deref(), Some("#FFE0A0"));
        let marker = apply_field(&entity, "shape", "none").unwrap();
        assert!(marker.shape.is_none() && marker.collider.is_none() && marker.material.is_none());
    }

    #[test]
    fn properties_parse_by_shape() {
        assert_eq!(parse_property("true"), PropertyValue::Bool(true));
        assert_eq!(parse_property(" 6 "), PropertyValue::Number(6.0));
        assert_eq!(parse_property("0, 0, -6"), PropertyValue::Vector([0.0, 0.0, -6.0]));
        assert_eq!(parse_property("[1,2,3]"), PropertyValue::Vector([1.0, 2.0, 3.0]));
        assert_eq!(parse_property("hello"), PropertyValue::Text("hello".into()));
        let entity = apply_field(&block(), "property:speed", "4").unwrap();
        assert_eq!(entity.number_property("speed"), Some(4.0));
        assert!(apply_field(&entity, "property:speed", "").unwrap().properties.is_empty());
    }
}
