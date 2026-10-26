//! Deterministic tests for the character controller's rules: landing, walls, corners, slopes,
//! stairs, ceilings, ledges (coyote time), jump buffering, variable jump height, moving
//! platforms, and frame-rate independence through the fixed-step clock.

use glam::{Mat4, Vec3};
use lantern_core::time::{FIXED_STEP_SECONDS, FixedStepClock, MAX_STEPS_PER_FRAME};
use lantern_physics::{
    CharacterController, ColliderId, CollisionWorld, ControllerEvents, ControllerInput, ControllerSettings, Triangle,
    box_triangles,
};

const DT: f32 = FIXED_STEP_SECONDS;
const FORWARD: Vec3 = Vec3::NEG_Z;

/// A 40 x 40 m floor whose top is y = 0.
fn floor() -> CollisionWorld {
    let mut world = CollisionWorld::new();
    world.add_static_box(Vec3::new(0.0, -0.5, 0.0), Vec3::new(20.0, 0.5, 20.0));
    world
}

/// A wedge rising toward -Z from height 0 at `center.z + length / 2`, like `primitives::ramp`.
fn ramp_triangles(center: Vec3, width: f32, height: f32, length: f32) -> Vec<Triangle> {
    let (w, l) = (width * 0.5, length * 0.5);
    let p = |x: f32, y: f32, z: f32| center + Vec3::new(x, y, z);
    let quad = |a, b, c, d| [Triangle::new(a, b, c), Triangle::new(a, c, d)];
    let mut triangles = Vec::new();
    triangles.extend(quad(p(-w, 0.0, l), p(w, 0.0, l), p(w, height, -l), p(-w, height, -l)));
    triangles.extend(quad(p(w, 0.0, -l), p(-w, 0.0, -l), p(-w, height, -l), p(w, height, -l)));
    triangles.push(Triangle::new(p(w, 0.0, l), p(w, 0.0, -l), p(w, height, -l)));
    triangles.push(Triangle::new(p(-w, 0.0, -l), p(-w, 0.0, l), p(-w, height, -l)));
    triangles
}

fn controller_at(feet: Vec3) -> CharacterController {
    CharacterController::new(ControllerSettings::default(), feet)
}

fn walk(direction: Vec3) -> ControllerInput {
    ControllerInput { move_direction: direction, ..Default::default() }
}

/// Runs `seconds` of fixed steps with the same input, collecting the events.
fn run(
    controller: &mut CharacterController,
    world: &CollisionWorld,
    input: ControllerInput,
    seconds: f32,
) -> Vec<ControllerEvents> {
    let steps = (seconds / DT).round() as usize;
    (0..steps).map(|_| controller.step(world, &input, DT)).collect()
}

/// Settles the controller onto the ground.
fn settle(controller: &mut CharacterController, world: &CollisionWorld) {
    run(controller, world, ControllerInput::default(), 0.5);
    assert!(controller.grounded, "did not settle at {:?}", controller.feet_position);
}

#[test]
fn falls_and_lands_once_resting_on_the_floor() {
    let world = floor();
    let mut controller = controller_at(Vec3::new(0.0, 3.0, 0.0));
    let events = run(&mut controller, &world, ControllerInput::default(), 2.0);
    let landings: Vec<f32> = events.iter().filter_map(|events| events.landed_with_speed).collect();
    assert_eq!(landings.len(), 1, "{landings:?}");
    // From 3 m under 28 m/s²: about sqrt(2 * 28 * 3) = 13 m/s.
    assert!((landings[0] - 12.96).abs() < 1.0, "{}", landings[0]);
    assert!(controller.grounded);
    assert!(controller.feet_position.y.abs() < 0.03, "{:?}", controller.feet_position);
}

#[test]
fn walls_stop_the_player_without_penetration() {
    let mut world = floor();
    world.add_static_box(Vec3::new(0.0, 1.0, -3.0), Vec3::new(5.0, 1.0, 0.5)); // face at z = -2.5
    let mut controller = controller_at(Vec3::ZERO);
    settle(&mut controller, &world);
    run(&mut controller, &world, walk(FORWARD), 2.0);
    let radius = controller.settings.radius;
    assert!(controller.feet_position.z > -2.5 + radius - 1e-3, "{:?}", controller.feet_position);
    assert!(controller.feet_position.z < -2.5 + radius + 0.05, "{:?}", controller.feet_position);
    assert!(controller.velocity.z.abs() < 1e-3);
    assert!(controller.grounded);
}

#[test]
fn diagonal_input_slides_along_a_wall() {
    let mut world = floor();
    world.add_static_box(Vec3::new(0.0, 1.0, -3.0), Vec3::new(10.0, 1.0, 0.5));
    let mut controller = controller_at(Vec3::ZERO);
    settle(&mut controller, &world);
    run(&mut controller, &world, walk(Vec3::new(1.0, 0.0, -1.0).normalize()), 2.0);
    // Blocked in z, still moving in x at about walk_speed * cos(45°).
    assert!(controller.feet_position.x > 5.0, "{:?}", controller.feet_position);
    assert!((controller.velocity.x - 6.0 * std::f32::consts::FRAC_1_SQRT_2).abs() < 0.3);
}

#[test]
fn inside_corners_hold_still_without_jitter() {
    let mut world = floor();
    world.add_static_box(Vec3::new(0.0, 1.0, -3.0), Vec3::new(5.0, 1.0, 0.5));
    world.add_static_box(Vec3::new(-3.0, 1.0, 0.0), Vec3::new(0.5, 1.0, 5.0));
    let mut controller = controller_at(Vec3::ZERO);
    settle(&mut controller, &world);
    let into_corner = walk(Vec3::new(-1.0, 0.0, -1.0).normalize());
    run(&mut controller, &world, into_corner, 2.0);
    let resting = controller.feet_position;
    for _ in 0..60 {
        controller.step(&world, &into_corner, DT);
        assert!(
            controller.feet_position.distance(resting) < 1e-3,
            "jitter: {:?} vs {resting:?}",
            controller.feet_position
        );
    }
    assert!(resting.x > -2.5 + 0.35 - 1e-3 && resting.z > -2.5 + 0.35 - 1e-3, "{resting:?}");
}

#[test]
fn walks_up_and_down_a_gentle_ramp_staying_grounded() {
    let mut world = floor();
    // 20 degrees: 1.5 m over 4 m, from z = -1 (low) to z = -5 (high), then a ledge.
    world.add_static_triangles(ramp_triangles(Vec3::new(0.0, 0.0, -3.0), 3.0, 1.5, 4.0));
    world.add_static_box(Vec3::new(0.0, 0.75, -6.5), Vec3::new(1.5, 0.75, 1.5));
    let mut controller = controller_at(Vec3::new(0.0, 0.0, 1.0));
    settle(&mut controller, &world);
    let mut was_ever_airborne = false;
    for _ in 0..90 {
        controller.step(&world, &walk(FORWARD), DT);
        was_ever_airborne |= !controller.grounded;
    }
    assert!(!was_ever_airborne, "lost the ground walking up");
    assert!((controller.feet_position.y - 1.5).abs() < 0.05, "{:?}", controller.feet_position);
    for _ in 0..90 {
        controller.step(&world, &walk(-FORWARD), DT);
        was_ever_airborne |= !controller.grounded;
    }
    assert!(!was_ever_airborne, "lost the ground walking down");
    assert!(controller.feet_position.y.abs() < 0.05, "{:?}", controller.feet_position);
}

#[test]
fn steep_slopes_cannot_be_walked_up() {
    let mut world = floor();
    // 59 degrees.
    world.add_static_triangles(ramp_triangles(Vec3::new(0.0, 0.0, -3.0), 3.0, 2.5, 1.5));
    let mut controller = controller_at(Vec3::ZERO);
    settle(&mut controller, &world);
    run(&mut controller, &world, walk(FORWARD), 3.0);
    assert!(controller.feet_position.y < 0.05, "climbed to {:?}", controller.feet_position);
    assert!(controller.grounded);
}

#[test]
fn landing_on_a_steep_slope_slides_off() {
    let mut world = floor();
    world.add_static_triangles(ramp_triangles(Vec3::new(0.0, 0.0, -3.0), 3.0, 2.5, 1.5));
    // Drop onto the middle of the slope.
    let mut controller = controller_at(Vec3::new(0.0, 3.0, -3.0));
    run(&mut controller, &world, ControllerInput::default(), 2.0);
    assert!(controller.grounded);
    assert!(controller.feet_position.y < 0.05, "stuck on the slope at {:?}", controller.feet_position);
}

#[test]
fn stairs_are_climbed_and_tall_ledges_are_not() {
    // Three 0.25 m risers at z = -1.5, -2.5 and -3.5, each step running back to z = -10.
    let mut world = floor();
    for (index, top) in [0.25f32, 0.5, 0.75].into_iter().enumerate() {
        let front_z = -1.5 - index as f32;
        let back_z = -10.0;
        let center = Vec3::new(0.0, top * 0.5, (front_z + back_z) * 0.5);
        world.add_static_box(center, Vec3::new(1.5, top * 0.5, (front_z - back_z) * 0.5));
    }
    let mut controller = controller_at(Vec3::new(0.0, 0.0, 1.0));
    settle(&mut controller, &world);
    run(&mut controller, &world, walk(FORWARD), 1.2);
    assert!((controller.feet_position.y - 0.75).abs() < 0.05, "stairs up: {:?}", controller.feet_position);
    run(&mut controller, &world, walk(-FORWARD), 1.5);
    assert!(controller.feet_position.y.abs() < 0.05, "stairs down: {:?}", controller.feet_position);
    assert!(controller.grounded);

    let mut world = floor();
    world.add_static_box(Vec3::new(0.0, 0.3, -3.0), Vec3::new(1.5, 0.3, 1.0)); // a 0.6 m ledge
    let mut controller = controller_at(Vec3::ZERO);
    settle(&mut controller, &world);
    run(&mut controller, &world, walk(FORWARD), 2.0);
    assert!(controller.feet_position.y < 0.05, "climbed a 0.6 m ledge: {:?}", controller.feet_position);
}

#[test]
fn ceilings_stop_the_jump() {
    let mut world = floor();
    world.add_static_box(Vec3::new(0.0, 2.15, 0.0), Vec3::new(1.5, 0.15, 1.5)); // underside at 2.0
    let mut controller = controller_at(Vec3::ZERO);
    settle(&mut controller, &world);
    let mut events = vec![controller.step(
        &world,
        &ControllerInput { jump_pressed: true, jump_held: true, ..Default::default() },
        DT,
    )];
    let mut highest_feet: f32 = 0.0;
    for _ in 0..90 {
        events.push(controller.step(&world, &ControllerInput { jump_held: true, ..Default::default() }, DT));
        highest_feet = highest_feet.max(controller.feet_position.y);
    }
    assert!(events[0].jumped);
    assert!(events.iter().any(|events| events.hit_ceiling));
    let head_room = 2.0 - controller.settings.height;
    assert!(highest_feet <= head_room + 1e-3, "{highest_feet} > {head_room}");
    assert!(controller.grounded, "came back down");
}

fn walk_off_ledge_then_jump(delay_steps: usize) -> bool {
    let mut world = CollisionWorld::new();
    world.add_static_box(Vec3::new(0.0, -0.5, 0.0), Vec3::new(2.0, 0.5, 2.0)); // edge at z = -2
    let mut controller = controller_at(Vec3::new(0.0, 0.0, -1.0));
    settle(&mut controller, &world);
    // Walk until the probe loses the ground.
    while controller.grounded {
        controller.step(&world, &walk(FORWARD), DT);
    }
    for _ in 0..delay_steps {
        controller.step(&world, &walk(FORWARD), DT);
    }
    let events =
        controller.step(&world, &ControllerInput { move_direction: FORWARD, jump_pressed: true, jump_held: true }, DT);
    events.jumped
}

#[test]
fn coyote_time_allows_a_late_jump_but_not_a_very_late_one() {
    assert!(walk_off_ledge_then_jump(0));
    assert!(walk_off_ledge_then_jump(4)); // ~67 ms after leaving the ground
    assert!(!walk_off_ledge_then_jump(12)); // 200 ms
}

#[test]
fn a_jump_pressed_just_before_landing_is_buffered() {
    let world = floor();
    let mut controller = controller_at(Vec3::new(0.0, 2.0, 0.0));
    // Fall until about 5 steps from the ground (the fall speed is ~10 m/s near the floor).
    while controller.feet_position.y > 0.5 {
        controller.step(&world, &ControllerInput::default(), DT);
    }
    assert!(!controller.grounded);
    let press = ControllerInput { jump_pressed: true, jump_held: true, ..Default::default() };
    let mut jumped = controller.step(&world, &press, DT).jumped;
    let held = ControllerInput { jump_held: true, ..Default::default() };
    for _ in 0..6 {
        jumped |= controller.step(&world, &held, DT).jumped;
    }
    assert!(jumped, "the buffered press was lost");
    assert!(controller.velocity.y > 0.0 || controller.feet_position.y > 0.1);
}

fn apex_height(hold_steps: usize) -> f32 {
    let world = floor();
    let mut controller = controller_at(Vec3::ZERO);
    settle(&mut controller, &world);
    controller.step(&world, &ControllerInput { jump_pressed: true, jump_held: true, ..Default::default() }, DT);
    let mut apex: f32 = 0.0;
    for step in 0..120 {
        let held = ControllerInput { jump_held: step < hold_steps, ..Default::default() };
        controller.step(&world, &held, DT);
        apex = apex.max(controller.feet_position.y);
    }
    apex
}

#[test]
fn holding_jump_goes_higher_than_tapping() {
    let full = apex_height(120);
    let tapped = apex_height(3);
    assert!((full - ControllerSettings::default().jump_height).abs() < 0.1, "{full}");
    assert!(tapped < full * 0.6, "tap {tapped} vs hold {full}");
}

#[test]
fn moving_platforms_carry_the_player() {
    let mut world = CollisionWorld::new();
    let platform = world.add_body(box_triangles(Vec3::ZERO, Vec3::new(2.0, 0.25, 2.0)), Mat4::IDENTITY);
    let mut controller = controller_at(Vec3::new(0.0, 0.3, 0.0));
    let mut position = Vec3::ZERO;
    for step in 0..120 {
        world.begin_step();
        if step >= 30 {
            position.x += 1.0 * DT;
            world.set_body_transform(platform, Mat4::from_translation(position));
        }
        controller.step(&world, &ControllerInput::default(), DT);
    }
    assert_eq!(controller.ground_collider, Some(ColliderId::Body(platform)));
    // The platform moved 1.5 m; the player rode along.
    assert!((controller.feet_position.x - position.x).abs() < 0.02, "{:?} vs {position:?}", controller.feet_position);
    assert!((controller.feet_position.y - 0.25).abs() < 0.03);
}

#[test]
fn platforms_moving_into_the_player_push_it_out() {
    let mut world = floor();
    let platform = world.add_body(
        box_triangles(Vec3::ZERO, Vec3::new(0.5, 1.0, 0.5)),
        Mat4::from_translation(Vec3::new(-3.0, 1.0, 0.0)),
    );
    let mut controller = controller_at(Vec3::ZERO);
    settle(&mut controller, &world);
    let mut x = -3.0;
    for _ in 0..120 {
        world.begin_step();
        x += 2.0 * DT;
        world.set_body_transform(platform, Mat4::from_translation(Vec3::new(x, 1.0, 0.0)));
        controller.step(&world, &ControllerInput::default(), DT);
        let overlap = world.capsule_contacts(controller.settings.capsule(), controller.capsule_center(), None);
        assert!(overlap.iter().all(|contact| contact.depth < 0.05), "{overlap:?}");
    }
    assert!(controller.feet_position.x > x + 0.5 + 0.3, "{:?} vs block at {x}", controller.feet_position);
}

/// Simulates display frames at `display_hz`. Input is decided per display frame (as the input
/// router does) and fed to every fixed step of that frame, with the jump press latched to the
/// frame's first step. The course: walk forward, jump the 2 m gap at the floor's edge, land.
fn traverse_gap_at_display_rate(display_hz: f64) -> Vec3 {
    let mut world = CollisionWorld::new();
    world.add_static_box(Vec3::new(0.0, -0.5, 0.0), Vec3::new(2.0, 0.5, 2.0)); // to z = -2
    world.add_static_box(Vec3::new(0.0, -0.5, -17.0), Vec3::new(2.0, 0.5, 13.0)); // from z = -4
    let mut controller = controller_at(Vec3::new(0.0, 0.0, 1.0));
    let mut clock = FixedStepClock::new(f64::from(DT), MAX_STEPS_PER_FRAME);
    let mut jump_sent = false;
    for _ in 0..(3.0 * display_hz) as usize {
        let plan = clock.advance(1.0 / display_hz);
        let jump_now = !jump_sent && controller.grounded && controller.feet_position.z < -1.4;
        for step in 0..plan.step_count {
            let input =
                ControllerInput { move_direction: FORWARD, jump_pressed: jump_now && step == 0, jump_held: true };
            controller.step(&world, &input, DT);
        }
        jump_sent |= jump_now && plan.step_count > 0;
    }
    controller.feet_position
}

#[test]
fn the_gap_is_cleared_at_every_display_rate() {
    let reference = traverse_gap_at_display_rate(60.0);
    for display_hz in [30.0, 60.0, 75.0, 144.0, 240.0] {
        let final_position = traverse_gap_at_display_rate(display_hz);
        assert!(
            final_position.y.abs() < 0.05 && final_position.z < -5.0,
            "{display_hz} Hz ended at {final_position:?}"
        );
        // Display rate only shifts when the jump press lands by at most one fixed step, and the
        // total number of steps by at most one, so the paths end within a step's travel.
        assert!(final_position.distance(reference) < 0.25, "{display_hz} Hz: {final_position:?} vs {reference:?}");
    }
}
