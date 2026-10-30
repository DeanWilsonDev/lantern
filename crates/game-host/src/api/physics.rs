//! `Physics` queries and the `CharacterController` type.
//!
//! The collision world is the engine's: world files add their static triangles and kinematic
//! bodies (`lantern:::world:::Load`), moving an entity moves its body (`lantern:::world:::SetPosition`), and the
//! engine calls `begin_step` before each fixed step. Scripts query it and drive controllers.

use std::cell::RefCell;

use glam::Vec3;
use lantern_physics::{Capsule, CharacterController, ControllerEvents, ControllerInput, ControllerSettings};
use nyx::{Receiver, Runtime};

use super::{
    HostNamespace, SharedState, SharedTypes, bool_argument, f32_argument, null, some_bool, some_f32, vec3_argument,
    wrap_vec3,
};

pub struct ScriptRayHit {
    pub distance: f32,
    pub point: Vec3,
    pub normal: Vec3,
}

pub struct ScriptControllerStep(pub ControllerEvents);

/// A kinematic capsule that walks, jumps, climbs steps and rides moving platforms
/// (`lantern_physics::CharacterController`). A method gets `&self`, hence the `RefCell`.
pub struct ScriptCharacterController(RefCell<CharacterController>);

pub fn register(runtime: &Runtime, state: &SharedState, types: &SharedTypes) -> nyx::Result<()> {
    let ray_types = types.clone();
    let raycast = (state.clone(), types.clone());
    let sphere_cast = (state.clone(), types.clone());

    let ray_hit = runtime
        .register_type::<ScriptRayHit>("lantern::physics::RayHit")
        .method("Distance", "() -> f32", Receiver::Immutable, |context, hit, _| some_f32(context, hit.distance))
        .method("Point", "() -> lantern:::math:::Vec3", Receiver::Immutable, move |_context, hit, _| {
            wrap_vec3(&ray_types, hit.point)
        })
        .method("Normal", "() -> lantern:::math:::Vec3", Receiver::Immutable, {
            let types = types.clone();
            move |_context, hit, _| wrap_vec3(&types, hit.normal)
        })
        .register()?;

    let controller_step = runtime
        .register_type::<ScriptControllerStep>("lantern::physics::ControllerStep")
        .method("Jumped", "() -> bool", Receiver::Immutable, |context, step, _| some_bool(context, step.0.jumped))
        .method("Landed", "() -> bool", Receiver::Immutable, |context, step, _| {
            some_bool(context, step.0.landed_with_speed.is_some())
        })
        // The downward speed at impact when `Landed()`, else 0.
        .method("LandingSpeed", "() -> f32", Receiver::Immutable, |context, step, _| {
            some_f32(context, step.0.landed_with_speed.unwrap_or(0.0))
        })
        .method("HitCeiling", "() -> bool", Receiver::Immutable, |context, step, _| {
            some_bool(context, step.0.hit_ceiling)
        })
        .register()?;

    HostNamespace::new(runtime, "lantern::physics")
        // The first surface along the ray, or null.
        .function(
            "Raycast",
            "(origin: lantern:::math:::Vec3, direction: lantern:::math:::Vec3, maxDistance: f32) -> lantern:::physics:::RayHit?",
            move |context, arguments| {
                let origin = vec3_argument(&raycast.1, arguments, 0)?;
                let direction = vec3_argument(&raycast.1, arguments, 1)?.normalize_or_zero();
                let max_distance = f32_argument(arguments, 2)?;
                let hit = raycast.0.borrow().collision.raycast(origin, direction, max_distance);
                match hit {
                    Some(hit) => {
                        let types = raycast.1.borrow();
                        let ray_hit = types
                            .ray_hit
                            .as_ref()
                            .ok_or_else(|| nyx::CallbackError::Refusal("lantern:::physics:::RayHit is not registered".into()))?;
                        Ok(Some(ray_hit.wrap(ScriptRayHit {
                            distance: hit.distance,
                            point: hit.point,
                            normal: hit.normal,
                        })?))
                    }
                    None => null(context),
                }
            },
        )
        // Sweeps a sphere from `origin` by `motion`: the fraction of `motion` before the first
        // contact (0..1), or null when the way is clear.
        .function(
            "SphereCast",
            "(radius: f32, origin: lantern:::math:::Vec3, motion: lantern:::math:::Vec3) -> f32?",
            move |context, arguments| {
                let radius = f32_argument(arguments, 0)?;
                let origin = vec3_argument(&sphere_cast.1, arguments, 1)?;
                let motion = vec3_argument(&sphere_cast.1, arguments, 2)?;
                let hit = sphere_cast.0.borrow().collision.capsule_cast(Capsule::sphere(radius), origin, motion, None);
                match hit {
                    Some(hit) => some_f32(context, hit.fraction),
                    None => null(context),
                }
            },
        )
        .register()?;

    let step_state = state.clone();
    let step_types = types.clone();
    let feet_types = types.clone();
    let center_types = types.clone();
    let velocity_types = types.clone();
    let normal_types = types.clone();
    let teleport_types = types.clone();
    let constructor_types = types.clone();
    runtime
        .register_type::<ScriptCharacterController>("lantern::physics::CharacterController")
        .constructor("(radius: f32, height: f32, feet: lantern:::math:::Vec3) -> void", move |_context, arguments| {
            let settings = ControllerSettings {
                radius: f32_argument(arguments, 0)?,
                height: f32_argument(arguments, 1)?,
                ..ControllerSettings::default()
            };
            let feet = vec3_argument(&constructor_types, arguments, 2)?;
            Ok(ScriptCharacterController(RefCell::new(CharacterController::new(settings, feet))))
        })
        // One fixed step. `moveDirection` is world space, length 0..1 (y ignored).
        .method(
            "Step",
            "(moveDirection: lantern:::math:::Vec3, jumpPressed: bool, jumpHeld: bool, dt: f32) -> lantern:::physics:::ControllerStep",
            Receiver::Mutable,
            move |_context, controller, arguments| {
                let input = ControllerInput {
                    move_direction: vec3_argument(&step_types, arguments, 0)?,
                    jump_pressed: bool_argument(arguments, 1)?,
                    jump_held: bool_argument(arguments, 2)?,
                };
                let dt = f32_argument(arguments, 3)?;
                let events = controller.0.borrow_mut().step(&step_state.borrow().collision, &input, dt);
                let types = step_types.borrow();
                let step_type = types
                    .controller_step
                    .as_ref()
                    .ok_or_else(|| nyx::CallbackError::Refusal("lantern:::physics:::ControllerStep is not registered".into()))?;
                Ok(Some(step_type.wrap(ScriptControllerStep(events))?))
            },
        )
        .method("Teleport", "(feet: lantern:::math:::Vec3) -> void", Receiver::Mutable, move |_context, controller, arguments| {
            let feet = vec3_argument(&teleport_types, arguments, 0)?;
            controller.0.borrow_mut().teleport(feet);
            Ok(None)
        })
        .method("FeetPosition", "() -> lantern:::math:::Vec3", Receiver::Immutable, move |_context, controller, _| {
            wrap_vec3(&feet_types, controller.0.borrow().feet_position)
        })
        .method("CapsuleCenter", "() -> lantern:::math:::Vec3", Receiver::Immutable, move |_context, controller, _| {
            wrap_vec3(&center_types, controller.0.borrow().capsule_center())
        })
        .method("Velocity", "() -> lantern:::math:::Vec3", Receiver::Immutable, move |_context, controller, _| {
            wrap_vec3(&velocity_types, controller.0.borrow().velocity)
        })
        .method("GroundNormal", "() -> lantern:::math:::Vec3", Receiver::Immutable, move |_context, controller, _| {
            wrap_vec3(&normal_types, controller.0.borrow().ground_normal)
        })
        .method("IsGrounded", "() -> bool", Receiver::Immutable, |context, controller, _| {
            some_bool(context, controller.0.borrow().grounded)
        })
        .method("Radius", "() -> f32", Receiver::Immutable, |context, controller, _| {
            some_f32(context, controller.0.borrow().settings.radius)
        })
        .method("Height", "() -> f32", Receiver::Immutable, |context, controller, _| {
            some_f32(context, controller.0.borrow().settings.height)
        })
        // Half the length of the capsule's straight middle: its axis runs from
        // `CapsuleCenter() - HalfSegment()` to `+ HalfSegment()` in y.
        .method("HalfSegment", "() -> f32", Receiver::Immutable, |context, controller, _| {
            some_f32(context, controller.0.borrow().settings.capsule().half_segment)
        })
        // Feel: the game's tuning (`settings/player-tuning.nyx` in the platformer).
        .method(
            "SetWalkSpeed",
            "(metresPerSecond: f32) -> void",
            Receiver::Mutable,
            |_context, controller, arguments| {
                controller.0.borrow_mut().settings.walk_speed = f32_argument(arguments, 0)?;
                Ok(None)
            },
        )
        .method("SetJumpHeight", "(metres: f32) -> void", Receiver::Mutable, |_context, controller, arguments| {
            controller.0.borrow_mut().settings.jump_height = f32_argument(arguments, 0)?;
            Ok(None)
        })
        .method(
            "SetGravity",
            "(metresPerSecondSquared: f32) -> void",
            Receiver::Mutable,
            |_context, controller, arguments| {
                controller.0.borrow_mut().settings.gravity = f32_argument(arguments, 0)?;
                Ok(None)
            },
        )
        .register()?;

    let mut types = types.borrow_mut();
    types.ray_hit = Some(ray_hit);
    types.controller_step = Some(controller_step);
    Ok(())
}
