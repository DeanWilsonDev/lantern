//! M4 gate: a scripted pilot runs the course's jumps, including the moving platform, at several
//! display rates. Collision is built from the real world file (assets/worlds/course.world.json)
//! the way `lantern:::world:::Load` builds it, and the platform follows the path the game's
//! `MovingPlatform` gives it (games/platformer/game-entities/moving-platform.nyx).
//!
//! The pilot works like a player: once per display frame it looks at the state and decides
//! the stick direction and whether to press jump; every fixed step of that frame then sees the
//! same input, with the press latched to the first step (as `InputRouter` does).

use std::path::PathBuf;

use glam::{Mat4, Vec2, Vec3};
use lantern_core::time::{FIXED_STEP_SECONDS, FixedStepClock, MAX_STEPS_PER_FRAME};
use lantern_physics::{CharacterController, CollisionWorld, ControllerInput, ControllerSettings};
use lantern_world_format::{ColliderKind, PropertyValue, WorldFile, build_pieces};

/// A leg of the course: walk toward `target` (x, z); jump when within `jump_distance` of
/// `jump_from` and `jump_condition` holds; the leg ends when the player is grounded within
/// 0.6 m of `target` (horizontally) and near `target.y`.
struct Leg {
    name: &'static str,
    target: Vec3,
    jump_from: Option<Vec3>,
    /// Extra condition for jumping, given the platform's current z.
    wait_for_platform: Option<fn(f32) -> bool>,
}

struct CourseRun {
    collision: CollisionWorld,
    platform: lantern_physics::BodyHandle,
    platform_start: Vec3,
    platform_travel: Vec3,
    platform_period_seconds: f32,
    controller: CharacterController,
    simulation_seconds: f32,
}

impl CourseRun {
    fn new() -> CourseRun {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets/worlds/course.world.json");
        let world_file = WorldFile::load(&path).expect("the course world file");
        let mut collision = CollisionWorld::new();
        let mut platform = None;
        for piece in build_pieces(&world_file) {
            match piece.collider {
                Some(ColliderKind::Static) => collision.add_static_triangles(piece.world_triangles()),
                Some(ColliderKind::Kinematic) => {
                    platform = Some(collision.add_body(piece.local_triangles(), piece.world_matrix));
                }
                None => {}
            }
        }
        let moving_platform = world_file.entities_of_kind("moving_platform").next().expect("a moving platform");
        let platform_travel = match moving_platform.properties.get("travel") {
            Some(PropertyValue::Vector(travel)) => Vec3::from_array(*travel),
            other => panic!("travel: {other:?}"),
        };
        // The pilot starts at the floor's front edge, facing the pit. The game's player tuning
        // (games/platformer/settings/player-tuning.nyx) matches the controller's defaults.
        CourseRun {
            collision,
            platform: platform.expect("the course has a moving platform"),
            platform_start: moving_platform.transform.position(),
            platform_travel,
            platform_period_seconds: moving_platform.number_property("period_seconds").expect("period_seconds"),
            controller: CharacterController::new(ControllerSettings::default(), Vec3::new(0.0, 0.0, -6.0)),
            simulation_seconds: 0.0,
        }
    }

    /// `MovingPlatform.PositionAt`: eases out to `start + travel` and back once per period.
    fn platform_position(&self, time_seconds: f32) -> Vec3 {
        let phase = time_seconds / self.platform_period_seconds * std::f32::consts::TAU;
        self.platform_start + self.platform_travel * (0.5 - 0.5 * phase.cos())
    }

    fn fixed_step(&mut self, input: &ControllerInput) {
        self.collision.begin_step();
        self.simulation_seconds += FIXED_STEP_SECONDS;
        let platform = Mat4::from_translation(self.platform_position(self.simulation_seconds));
        self.collision.set_body_transform(self.platform, platform);
        self.controller.step(&self.collision, input, FIXED_STEP_SECONDS);
    }
}

fn course() -> Vec<Leg> {
    vec![
        Leg { name: "edge", target: Vec3::new(0.0, 0.0, -7.4), jump_from: None, wait_for_platform: None },
        Leg {
            name: "pillar 1",
            target: Vec3::new(0.0, 0.5, -11.0),
            jump_from: Some(Vec3::new(0.0, 0.0, -7.4)),
            wait_for_platform: None,
        },
        Leg {
            name: "pillar 2",
            target: Vec3::new(2.5, 1.2, -14.5),
            jump_from: Some(Vec3::new(0.5, 0.5, -11.6)),
            wait_for_platform: None,
        },
        Leg {
            name: "pillar 3",
            target: Vec3::new(0.0, 2.0, -18.0),
            jump_from: Some(Vec3::new(2.0, 1.2, -15.2)),
            wait_for_platform: None,
        },
        Leg {
            name: "moving platform",
            target: Vec3::new(0.0, 2.0, -21.0),
            jump_from: Some(Vec3::new(0.0, 2.0, -18.6)),
            // Jump when the platform is near its pillar-3 end.
            wait_for_platform: Some(|platform_z| platform_z > -21.4),
        },
        Leg {
            name: "goal",
            target: Vec3::new(0.0, 2.0, -31.0),
            jump_from: Some(Vec3::new(0.0, 2.0, -21.0)),
            // Ride until the platform is near the goal.
            wait_for_platform: Some(|platform_z| platform_z < -26.6),
        },
    ]
}

/// Runs the course at `display_hz`. With `LANTERN_TRACE_DIRECTORY` set, also writes
/// `rust-<hz>hz.trace` there: one line per fixed step with the step number, the feet position,
/// the velocity and whether the player is grounded. The Nox engine's port of this test compares
/// its own run against these files.
fn run_course(display_hz: f64) -> Result<(), String> {
    let mut trace = std::env::var_os("LANTERN_TRACE_DIRECTORY").map(|_| String::new());
    let outcome = run_course_traced(display_hz, &mut trace);
    if let (Some(directory), Some(text)) = (std::env::var_os("LANTERN_TRACE_DIRECTORY"), trace) {
        let path = PathBuf::from(directory).join(format!("rust-{display_hz}hz.trace"));
        std::fs::write(&path, text).expect("the trace file");
    }
    outcome
}

fn run_course_traced(display_hz: f64, trace: &mut Option<String>) -> Result<(), String> {
    let mut run = CourseRun::new();
    let mut clock = FixedStepClock::new(f64::from(FIXED_STEP_SECONDS), MAX_STEPS_PER_FRAME);
    let legs = course();
    let mut leg_index = 0;
    let mut jumped_this_leg = false;
    let frame_limit = (60.0 * display_hz) as usize;
    let mut step_number: u64 = 0;
    for _ in 0..frame_limit {
        let Some(leg) = legs.get(leg_index) else {
            let feet = run.controller.feet_position;
            // On the goal platform (z from -28.5 to -33.5, top at y = 2).
            let on_goal = (-33.5..=-28.5).contains(&feet.z) && (feet.y - 2.0).abs() < 0.05;
            println!("{display_hz} Hz: finished at {feet:?} after {:.1} s", run.simulation_seconds);
            return if on_goal { Ok(()) } else { Err(format!("finished off the goal at {feet:?}")) };
        };
        let controller = &run.controller;
        let feet = controller.feet_position;
        if feet.y < -5.0 {
            return Err(format!("fell during '{}' at {display_hz} Hz ({feet:?})", leg.name));
        }
        let to_target = Vec2::new(leg.target.x - feet.x, leg.target.z - feet.z);
        let arrived = controller.grounded && to_target.length() < 0.6 && (feet.y - leg.target.y).abs() < 0.15;
        // On a platform, "arrived" is relative to the platform, so the goal leg only checks
        // the ride leg's grounding on the body.
        let arrived = arrived
            || (leg.name == "moving platform"
                && controller.grounded
                && controller.ground_collider == Some(lantern_physics::ColliderId::Body(run.platform)));
        if arrived {
            leg_index += 1;
            jumped_this_leg = false;
            continue;
        }
        let platform_z = run.platform_position(run.simulation_seconds).z;
        let platform_ready = leg.wait_for_platform.is_none_or(|ready| ready(platform_z));
        let at_jump_spot =
            leg.jump_from.is_some_and(|spot| Vec2::new(spot.x - feet.x, spot.z - feet.z).length() < 0.35);
        let mut stick = to_target.normalize_or_zero();
        // Wait at the jump spot (standing still) until the platform is in place.
        if let Some(spot) = leg.jump_from
            && !jumped_this_leg
        {
            let to_spot = Vec2::new(spot.x - feet.x, spot.z - feet.z);
            stick = if at_jump_spot {
                if platform_ready { stick } else { Vec2::ZERO }
            } else {
                to_spot.normalize_or_zero()
            };
        }
        // Riding the platform: stand still until it is time to jump.
        if leg.name == "goal" && !jumped_this_leg && !platform_ready {
            stick = Vec2::ZERO;
        }
        let jump_now =
            !jumped_this_leg && controller.grounded && platform_ready && (at_jump_spot || leg.name == "goal");
        // Stick y is forward (-Z); the camera faces -Z, so x maps to x and y maps to -z.
        let move_direction = Vec3::new(stick.x, 0.0, stick.y);
        let plan = clock.advance(1.0 / display_hz);
        for step in 0..plan.step_count {
            let input = ControllerInput { move_direction, jump_pressed: jump_now && step == 0, jump_held: true };
            run.fixed_step(&input);
            step_number += 1;
            if let Some(text) = trace.as_mut() {
                let controller = &run.controller;
                let feet = controller.feet_position;
                let velocity = controller.velocity;
                use std::fmt::Write as _;
                let _ = writeln!(
                    text,
                    "{step_number} {} {} {} {} {} {} {}",
                    feet.x,
                    feet.y,
                    feet.z,
                    velocity.x,
                    velocity.y,
                    velocity.z,
                    u8::from(controller.grounded)
                );
            }
        }
        jumped_this_leg |= jump_now && plan.step_count > 0;
    }
    match legs.get(leg_index) {
        None => Ok(()),
        Some(leg) => {
            Err(format!("timed out during '{}' at {display_hz} Hz ({:?})", leg.name, run.controller.feet_position))
        }
    }
}

#[test]
fn the_jump_course_is_traversable_at_every_display_rate() {
    for display_hz in [30.0, 60.0, 144.0, 240.0] {
        if let Err(problem) = run_course(display_hz) {
            panic!("{problem}");
        }
    }
}
