//! `Vec3`, `Vec2` and `Angle`: vector values for scripts.
//!
//! Vectors are value types: scripts read and write `x`, `y` and `z` as fields, build one with
//! braces (`Vec3 { x: 1.0f, y: 2.0f }`, missing fields are 0) and get a copy at every binding,
//! argument and return. Arithmetic is operators the host declares: `a + b`, `a - b`, `v * s`,
//! `v / s` and `-v`, with the vector on the left.

use glam::{Vec2, Vec3};
use lantern_core::math::{wrap_angle, yaw_from_direction};
use nyx::{CallbackError, HostTypeBuilder, Receiver, Runtime, Value};

use super::{SharedTypes, f32_argument, some_f32, wrap_vec2, wrap_vec3};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[repr(transparent)]
pub struct ScriptVec3(pub Vec3);

#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[repr(transparent)]
pub struct ScriptVec2(pub Vec2);

/// Marker for `Angle`'s static helpers.
struct AngleApi;

fn other_vec3(types: &SharedTypes, arguments: &[Value]) -> Result<Vec3, CallbackError> {
    super::vec3_argument(types, arguments, 0)
}

/// A `Vec3` method that returns a new `Vec3`.
fn vec3_to_vec3(
    builder: HostTypeBuilder<ScriptVec3>,
    types: &SharedTypes,
    name: &str,
    signature: &str,
    operation: impl Fn(Vec3, &SharedTypes, &[Value]) -> Result<Vec3, CallbackError> + 'static,
) -> HostTypeBuilder<ScriptVec3> {
    let types = types.clone();
    builder.method(name, signature, Receiver::Immutable, move |_context, vector, arguments| {
        let result = operation(vector.0, &types, arguments)?;
        wrap_vec3(&types, result)
    })
}

/// A `Vec3` operator with a `Vec3` right operand.
fn vec3_with_vec3(
    builder: HostTypeBuilder<ScriptVec3>,
    symbol: &str,
    operation: fn(Vec3, Vec3) -> Vec3,
) -> HostTypeBuilder<ScriptVec3> {
    builder.operator(symbol, "(other: Vec3) -> Vec3", move |_context, own_type, vector, operands| {
        let other = own_type.get(super::argument(operands, 0)?)?.0;
        Ok(Some(own_type.wrap(ScriptVec3(operation(vector.0, other)))?))
    })
}

/// A `Vec3` operator with an `f32` right operand.
fn vec3_with_f32(
    builder: HostTypeBuilder<ScriptVec3>,
    symbol: &str,
    operation: fn(Vec3, f32) -> Vec3,
) -> HostTypeBuilder<ScriptVec3> {
    builder.operator(symbol, "(scalar: f32) -> Vec3", move |_context, own_type, vector, operands| {
        Ok(Some(own_type.wrap(ScriptVec3(operation(vector.0, f32_argument(operands, 0)?)))?))
    })
}

/// A `Vec2` operator with a `Vec2` right operand.
fn vec2_with_vec2(
    builder: HostTypeBuilder<ScriptVec2>,
    symbol: &str,
    operation: fn(Vec2, Vec2) -> Vec2,
) -> HostTypeBuilder<ScriptVec2> {
    builder.operator(symbol, "(other: Vec2) -> Vec2", move |_context, own_type, vector, operands| {
        let other = own_type.get(super::argument(operands, 0)?)?.0;
        Ok(Some(own_type.wrap(ScriptVec2(operation(vector.0, other)))?))
    })
}

/// A `Vec2` operator with an `f32` right operand.
fn vec2_with_f32(
    builder: HostTypeBuilder<ScriptVec2>,
    symbol: &str,
    operation: fn(Vec2, f32) -> Vec2,
) -> HostTypeBuilder<ScriptVec2> {
    builder.operator(symbol, "(scalar: f32) -> Vec2", move |_context, own_type, vector, operands| {
        Ok(Some(own_type.wrap(ScriptVec2(operation(vector.0, f32_argument(operands, 0)?)))?))
    })
}

/// A `Vec3` method that returns an `f32`.
fn vec3_to_f32(
    builder: HostTypeBuilder<ScriptVec3>,
    types: &SharedTypes,
    name: &str,
    signature: &str,
    operation: impl Fn(Vec3, &SharedTypes, &[Value]) -> Result<f32, CallbackError> + 'static,
) -> HostTypeBuilder<ScriptVec3> {
    let types = types.clone();
    builder.method(name, signature, Receiver::Immutable, move |context, vector, arguments| {
        let result = operation(vector.0, &types, arguments)?;
        some_f32(context, result)
    })
}

pub fn register(runtime: &Runtime, types: &SharedTypes) -> nyx::Result<()> {
    let mut vec3 = runtime
        .register_value_type::<ScriptVec3>("Vec3", ScriptVec3::default())
        .field("x", |v| &mut v.0.x)
        .field("y", |v| &mut v.0.y)
        .field("z", |v| &mut v.0.z)
        .static_method("Zero", "() -> Vec3", |_context, own_type, _| Ok(Some(own_type.wrap(ScriptVec3(Vec3::ZERO))?)))
        .static_method("Up", "() -> Vec3", |_context, own_type, _| Ok(Some(own_type.wrap(ScriptVec3(Vec3::Y))?)))
        .static_method("Down", "() -> Vec3", |_context, own_type, _| Ok(Some(own_type.wrap(ScriptVec3(Vec3::NEG_Y))?)))
        .static_method("FromYaw", "(yaw: f32) -> Vec3", |_context, own_type, arguments| {
            let direction = lantern_core::math::direction_from_yaw(f32_argument(arguments, 0)?);
            Ok(Some(own_type.wrap(ScriptVec3(direction))?))
        });
    vec3 = vec3_with_vec3(vec3, "+", |left, right| left + right);
    vec3 = vec3_with_vec3(vec3, "-", |left, right| left - right);
    vec3 = vec3_with_f32(vec3, "*", |vector, scalar| vector * scalar);
    vec3 = vec3_with_f32(vec3, "/", |vector, scalar| vector / scalar);
    vec3 = vec3.operator("-", "() -> Vec3", |_context, own_type, vector, _| Ok(Some(own_type.wrap(ScriptVec3(-vector.0))?)));
    vec3 = vec3_to_vec3(vec3, types, "Cross", "(other: Vec3) -> Vec3", |v, t, a| Ok(v.cross(other_vec3(t, a)?)));
    vec3 = vec3_to_vec3(vec3, types, "Normalized", "() -> Vec3", |v, _, _| Ok(v.normalize_or_zero()));
    vec3 = vec3_to_vec3(vec3, types, "Horizontal", "() -> Vec3", |v, _, _| Ok(Vec3::new(v.x, 0.0, v.z)));
    vec3 =
        vec3_to_vec3(vec3, types, "WithX", "(x: f32) -> Vec3", |v, _, a| Ok(Vec3::new(f32_argument(a, 0)?, v.y, v.z)));
    vec3 =
        vec3_to_vec3(vec3, types, "WithY", "(y: f32) -> Vec3", |v, _, a| Ok(Vec3::new(v.x, f32_argument(a, 0)?, v.z)));
    vec3 =
        vec3_to_vec3(vec3, types, "WithZ", "(z: f32) -> Vec3", |v, _, a| Ok(Vec3::new(v.x, v.y, f32_argument(a, 0)?)));
    vec3 = vec3_to_vec3(vec3, types, "ClampLength", "(maximum: f32) -> Vec3", |v, _, a| {
        Ok(v.clamp_length_max(f32_argument(a, 0)?))
    });
    vec3 = vec3_to_vec3(vec3, types, "Lerp", "(other: Vec3, t: f32) -> Vec3", |v, t, a| {
        Ok(v.lerp(other_vec3(t, a)?, f32_argument(a, 1)?))
    });
    vec3 = vec3_to_vec3(vec3, types, "Clamp", "(minimum: Vec3, maximum: Vec3) -> Vec3", |v, t, a| {
        Ok(v.clamp(super::vec3_argument(t, a, 0)?, super::vec3_argument(t, a, 1)?))
    });
    vec3 = vec3_to_f32(vec3, types, "Dot", "(other: Vec3) -> f32", |v, t, a| Ok(v.dot(other_vec3(t, a)?)));
    vec3 = vec3_to_f32(vec3, types, "Length", "() -> f32", |v, _, _| Ok(v.length()));
    vec3 = vec3_to_f32(vec3, types, "LengthSquared", "() -> f32", |v, _, _| Ok(v.length_squared()));
    vec3 = vec3_to_f32(vec3, types, "DistanceTo", "(other: Vec3) -> f32", |v, t, a| Ok(v.distance(other_vec3(t, a)?)));
    vec3 = vec3_to_f32(vec3, types, "Yaw", "() -> f32", |v, _, _| Ok(yaw_from_direction(v)));
    let vec3 = vec3.register()?;

    let clamp_types = types.clone();
    let normalized_types = types.clone();
    let mut vec2 = runtime
        .register_value_type::<ScriptVec2>("Vec2", ScriptVec2::default())
        .field("x", |v| &mut v.0.x)
        .field("y", |v| &mut v.0.y)
        .static_method("Zero", "() -> Vec2", |_context, own_type, _| Ok(Some(own_type.wrap(ScriptVec2(Vec2::ZERO))?)))
        .operator("-", "() -> Vec2", |_context, own_type, vector, _| Ok(Some(own_type.wrap(ScriptVec2(-vector.0))?)));
    vec2 = vec2_with_vec2(vec2, "+", |left, right| left + right);
    vec2 = vec2_with_vec2(vec2, "-", |left, right| left - right);
    vec2 = vec2_with_f32(vec2, "*", |vector, scalar| vector * scalar);
    vec2 = vec2_with_f32(vec2, "/", |vector, scalar| vector / scalar);
    let vec2 = vec2
        .method("Length", "() -> f32", Receiver::Immutable, |context, vector, _| some_f32(context, vector.0.length()))
        .method("ClampLength", "(maximum: f32) -> Vec2", Receiver::Immutable, move |_context, vector, arguments| {
            wrap_vec2(&clamp_types, vector.0.clamp_length_max(f32_argument(arguments, 0)?))
        })
        .method("Normalized", "() -> Vec2", Receiver::Immutable, move |_context, vector, _| {
            wrap_vec2(&normalized_types, vector.0.normalize_or_zero())
        })
        .register()?;

    runtime
        .register_type::<AngleApi>("Angle")
        .static_method("Wrap", "(radians: f32) -> f32", |context, _, arguments| {
            some_f32(context, wrap_angle(f32_argument(arguments, 0)?))
        })
        .static_method("ToDegrees", "(radians: f32) -> f32", |context, _, arguments| {
            some_f32(context, f32_argument(arguments, 0)?.to_degrees())
        })
        .register()?;

    let mut types = types.borrow_mut();
    types.vec3 = Some(vec3);
    types.vec2 = Some(vec2);
    Ok(())
}
