//! The kinematic character controller: input → velocity → move-and-slide → grounded state.
//!
//! The player is not a rigid body. Each fixed step the controller decides a velocity from the
//! input and its rules (acceleration, gravity, jumping), then moves a capsule through the
//! collision world with casts, sliding along whatever it hits. Behaviour is fully decided by
//! the inputs and `dt`, so the same input sequence always produces the same path.
//!
//! Rules, in the order `step` applies them:
//! 1. Ride: standing on a moving platform carries the player by the platform's motion.
//! 2. Timers: jump buffering (a press shortly before landing still jumps) and coyote time (a
//!    jump shortly after walking off a ledge still works).
//! 3. Horizontal velocity accelerates toward the input; on the ground it follows the slope.
//! 4. Gravity, stronger while rising if the jump button was released (variable jump height).
//! 5. Jump.
//! 6. Move and slide: walls and steep slopes block (with step-up for small ledges), ceilings
//!    stop upward speed.
//! 7. Ground probe: snap down onto walkable ground within reach, so walking down slopes and
//!    stairs stays grounded.
//!
//! Two techniques carry most of the robustness:
//! - Movement casts use the capsule inflated by `skin_width`. Stopping at a hit therefore
//!   leaves exactly the skin as a gap, so the next cast starts out of contact, and sliding
//!   along a surface never re-hits it.
//! - The rounded bottom of a capsule touching a ledge's edge reports a tilted contact normal
//!   that looks like a steep slope. For "can I stand here?" the controller therefore also
//!   checks the face beside the contact with a short ray (as PhysX's controller uses the
//!   touched triangle's normal), so stair edges count as ground and real steep slopes do not.
//! 8. Depenetration: anything that moved into the player (a platform) pushes it out.

use glam::Vec3;

use crate::world::{Capsule, ColliderId, CollisionWorld};

#[derive(Clone, Copy, Debug)]
pub struct ControllerSettings {
    pub radius: f32,
    pub height: f32,
    /// Gap kept between the capsule and surfaces, so the next cast does not start in contact.
    pub skin_width: f32,
    /// Ledges up to this high are climbed by walking.
    pub step_height: f32,
    /// Steeper surfaces are walls: they block and you slide off them.
    pub max_walkable_slope_degrees: f32,
    /// While grounded, ground this far below is snapped to (walking down slopes and stairs).
    pub ground_snap_distance: f32,
    pub walk_speed: f32,
    pub ground_acceleration: f32,
    pub air_acceleration: f32,
    pub gravity: f32,
    /// Apex height of a full (held) jump.
    pub jump_height: f32,
    /// Gravity multiplier while rising with the jump button released.
    pub jump_release_gravity_multiplier: f32,
    pub max_fall_speed: f32,
    pub coyote_seconds: f32,
    pub jump_buffer_seconds: f32,
}

impl Default for ControllerSettings {
    fn default() -> Self {
        ControllerSettings {
            radius: 0.35,
            height: 1.7,
            skin_width: 0.01,
            step_height: 0.3,
            max_walkable_slope_degrees: 46.0,
            ground_snap_distance: 0.3,
            walk_speed: 6.0,
            ground_acceleration: 50.0,
            air_acceleration: 20.0,
            gravity: 28.0,
            jump_height: 1.5,
            jump_release_gravity_multiplier: 2.5,
            max_fall_speed: 30.0,
            coyote_seconds: 0.1,
            jump_buffer_seconds: 0.12,
        }
    }
}

impl ControllerSettings {
    /// The launch speed that reaches `jump_height` under `gravity`: v = sqrt(2 g h).
    pub fn jump_speed(&self) -> f32 {
        (2.0 * self.gravity * self.jump_height).sqrt()
    }

    pub fn capsule(&self) -> Capsule {
        Capsule::upright(self.radius, self.height)
    }

    fn min_ground_normal_y(&self) -> f32 {
        self.max_walkable_slope_degrees.to_radians().cos()
    }

    /// The capsule used for movement casts: inflated by the skin (see the module docs).
    fn movement_capsule(&self) -> Capsule {
        Capsule { radius: self.radius + self.skin_width, half_segment: self.capsule().half_segment }
    }
}

/// One fixed step's input, already in world space.
#[derive(Clone, Copy, Debug, Default)]
pub struct ControllerInput {
    /// Desired horizontal direction; length 0..=1 (the y component is ignored).
    pub move_direction: Vec3,
    /// Jump was pressed this step (an edge).
    pub jump_pressed: bool,
    /// Jump is held (for variable jump height).
    pub jump_held: bool,
}

/// What happened during a step; the game turns these into sounds and script events.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ControllerEvents {
    pub jumped: bool,
    /// Landed this step, with the downward speed at impact.
    pub landed_with_speed: Option<f32>,
    pub hit_ceiling: bool,
}

#[derive(Clone, Debug)]
pub struct CharacterController {
    pub settings: ControllerSettings,
    /// The bottom of the capsule.
    pub feet_position: Vec3,
    pub velocity: Vec3,
    pub grounded: bool,
    pub ground_normal: Vec3,
    pub ground_collider: Option<ColliderId>,
    seconds_since_grounded: f32,
    jump_buffer_remaining: f32,
    rising_from_jump: bool,
}

const MAX_SLIDE_ITERATIONS: usize = 4;
const MAX_DEPENETRATION_ITERATIONS: usize = 4;

fn horizontal(vector: Vec3) -> Vec3 {
    Vec3::new(vector.x, 0.0, vector.z)
}

fn move_towards_vector(current: Vec3, target: Vec3, max_delta: f32) -> Vec3 {
    let difference = target - current;
    let distance = difference.length();
    if distance <= max_delta || distance < 1e-9 { target } else { current + difference / distance * max_delta }
}

impl CharacterController {
    pub fn new(settings: ControllerSettings, feet_position: Vec3) -> CharacterController {
        CharacterController {
            settings,
            feet_position,
            velocity: Vec3::ZERO,
            grounded: false,
            ground_normal: Vec3::Y,
            ground_collider: None,
            seconds_since_grounded: f32::INFINITY,
            jump_buffer_remaining: 0.0,
            rising_from_jump: false,
        }
    }

    /// Puts the player at `feet_position`, at rest (used by reset).
    pub fn teleport(&mut self, feet_position: Vec3) {
        *self = CharacterController::new(self.settings, feet_position);
    }

    pub fn capsule_center(&self) -> Vec3 {
        self.feet_position + Vec3::Y * self.settings.height * 0.5
    }

    fn set_capsule_center(&mut self, center: Vec3) {
        self.feet_position = center - Vec3::Y * self.settings.height * 0.5;
    }

    pub fn step(&mut self, world: &CollisionWorld, input: &ControllerInput, dt: f32) -> ControllerEvents {
        let settings = self.settings;
        let mut events = ControllerEvents::default();
        let was_grounded = self.grounded;

        // 1. Ride the platform we stand on.
        if let (true, Some(ColliderId::Body(body))) = (self.grounded, self.ground_collider) {
            let motion = world.body_step_motion(body);
            let carried = motion.transform_point3(self.feet_position) - self.feet_position;
            if carried.length_squared() > 0.0 {
                self.move_without_sliding(world, carried, Some(ColliderId::Body(body)));
            }
        }

        // 2. Timers.
        if input.jump_pressed {
            self.jump_buffer_remaining = settings.jump_buffer_seconds;
        } else {
            self.jump_buffer_remaining = (self.jump_buffer_remaining - dt).max(0.0);
        }
        self.seconds_since_grounded = if self.grounded { 0.0 } else { self.seconds_since_grounded + dt };

        // 3. Horizontal velocity.
        let desired = horizontal(input.move_direction).clamp_length_max(1.0) * settings.walk_speed;
        let acceleration = if self.grounded { settings.ground_acceleration } else { settings.air_acceleration };
        let planar = move_towards_vector(horizontal(self.velocity), desired, acceleration * dt);
        self.velocity.x = planar.x;
        self.velocity.z = planar.z;

        // 4. Gravity.
        if self.grounded {
            self.velocity.y = 0.0;
        } else {
            let released_early = self.rising_from_jump && !input.jump_held && self.velocity.y > 0.0;
            let multiplier = if released_early { settings.jump_release_gravity_multiplier } else { 1.0 };
            self.velocity.y = (self.velocity.y - settings.gravity * multiplier * dt).max(-settings.max_fall_speed);
        }

        // 5. Jump: needs a buffered press and ground (or ground a moment ago).
        let can_jump = self.grounded || self.seconds_since_grounded <= settings.coyote_seconds;
        if self.jump_buffer_remaining > 0.0 && can_jump {
            self.velocity.y = settings.jump_speed();
            self.jump_buffer_remaining = 0.0;
            self.seconds_since_grounded = f32::INFINITY; // no second coyote jump
            self.grounded = false;
            self.ground_collider = None;
            self.rising_from_jump = true;
            events.jumped = true;
        }

        // 6. Move and slide. On the ground, horizontal motion follows the ground plane, so
        //    walking up and down ramps keeps full horizontal speed and stays in contact.
        let mut displacement = self.velocity * dt;
        if self.grounded {
            let normal = self.ground_normal;
            displacement.y = -(normal.x * displacement.x + normal.z * displacement.z) / normal.y.max(1e-3);
        }
        let falling_speed = -self.velocity.y;
        self.move_and_slide(world, displacement, &mut events);

        // 7. Ground probe.
        if self.velocity.y <= 0.0 {
            self.rising_from_jump = false;
            let reach = if was_grounded { settings.ground_snap_distance } else { settings.skin_width };
            self.probe_ground(world, reach);
        } else {
            self.grounded = false;
            self.ground_collider = None;
        }
        if self.grounded {
            self.velocity.y = 0.0;
            if !was_grounded {
                events.landed_with_speed = Some(falling_speed.max(0.0));
            }
        }

        // 8. Depenetration.
        self.depenetrate(world);
        events
    }

    fn is_walkable(&self, normal: Vec3) -> bool {
        normal.y >= self.settings.min_ground_normal_y()
    }

    /// Moves by `displacement`, stopping at the first contact (no sliding). Used for being
    /// carried by a platform.
    fn move_without_sliding(&mut self, world: &CollisionWorld, displacement: Vec3, ignore: Option<ColliderId>) {
        let capsule = self.settings.capsule();
        let center = self.capsule_center();
        match world.capsule_cast(capsule, center, displacement, ignore) {
            None => self.set_capsule_center(center + displacement),
            Some(hit) => {
                self.set_capsule_center(center + displacement * hit.fraction + hit.normal * self.settings.skin_width)
            }
        }
    }

    fn move_and_slide(&mut self, world: &CollisionWorld, displacement: Vec3, events: &mut ControllerEvents) {
        let capsule = self.settings.movement_capsule();
        let mut center = self.capsule_center();
        let mut remaining = displacement;
        let mut slide_normals: Vec<Vec3> = Vec::with_capacity(MAX_SLIDE_ITERATIONS);
        for _ in 0..MAX_SLIDE_ITERATIONS {
            if remaining.length_squared() < 1e-12 {
                break;
            }
            let Some(hit) = world.capsule_cast(capsule, center, remaining, None) else {
                center += remaining;
                break;
            };
            center += remaining * hit.fraction;
            remaining *= 1.0 - hit.fraction;
            let walkable = self.is_walkable(hit.normal);

            if self.grounded && !walkable {
                self.set_capsule_center(center);
                if let Some(stepped_center) = self.try_step_up(world, horizontal(remaining)) {
                    center = stepped_center;
                    remaining.x = 0.0;
                    remaining.z = 0.0;
                    continue;
                }
            }
            if hit.normal.y < -0.3 && self.velocity.y > 0.0 {
                self.velocity.y = 0.0;
                events.hit_ceiling = true;
            }

            // Grounded players treat steep surfaces as vertical walls, so they cannot be
            // pushed up a slope they are not allowed to walk on.
            let slide_normal = if self.grounded && !walkable {
                horizontal(hit.normal).try_normalize().unwrap_or(hit.normal)
            } else {
                hit.normal
            };
            // Velocity into the surface is removed too, so it does not build up against walls.
            let into_surface = self.velocity.dot(slide_normal);
            if into_surface < 0.0 && !(walkable && self.grounded) {
                self.velocity -= slide_normal * into_surface;
            }
            slide_normals.push(slide_normal);
            remaining = match slide_normals.len() {
                1 => remaining - slide_normal * remaining.dot(slide_normal),
                // In a crease between two surfaces, slide along the crease line only;
                // projecting on each plane in turn would bounce between them.
                2 => {
                    let crease = slide_normals[0].cross(slide_normals[1]);
                    match crease.try_normalize() {
                        Some(direction) => direction * remaining.dot(direction),
                        None => remaining - slide_normal * remaining.dot(slide_normal),
                    }
                }
                _ => Vec3::ZERO,
            };
        }
        self.set_capsule_center(center);
    }

    /// Tries to climb a ledge no higher than `step_height`: up, forward, then down onto
    /// walkable ground. Returns the new capsule centre, or `None` (nothing changes) if the
    /// ledge is too high, blocked above, or not walkable on top.
    fn try_step_up(&self, world: &CollisionWorld, forward: Vec3) -> Option<Vec3> {
        let settings = &self.settings;
        let capsule = settings.movement_capsule();
        let forward_length = forward.length();
        if forward_length < 1e-5 {
            return None;
        }
        let forward_direction = forward / forward_length;
        // Move at least a little forward, so a step blocked at the very end of a move is still
        // climbed on the next step.
        let forward = forward_direction * forward_length.max(settings.skin_width * 4.0);
        let start = self.capsule_center();

        let up = Vec3::Y * settings.step_height;
        let raised = start + up * world.capsule_cast(capsule, start, up, None).map_or(1.0, |hit| hit.fraction);
        let advanced =
            raised + forward * world.capsule_cast(capsule, raised, forward, None).map_or(1.0, |hit| hit.fraction);
        if horizontal(advanced - start).length() < settings.skin_width {
            return None;
        }
        let down = Vec3::Y * -(raised.y - start.y + settings.skin_width);
        let hit = world.capsule_cast(capsule, advanced, down, None)?;
        let lands_on_walkable =
            self.is_walkable(hit.normal) || self.walkable_face_near(world, hit.point, forward_direction).is_some();
        if !lands_on_walkable {
            return None;
        }
        let landed = advanced + down * hit.fraction;
        (landed.y > start.y + settings.skin_width).then_some(landed)
    }

    /// Looks for a walkable face just beside `contact` (offset by 2 cm along `direction`),
    /// within a step's height below. Returns its face normal.
    fn walkable_face_near(&self, world: &CollisionWorld, contact: Vec3, direction: Vec3) -> Option<Vec3> {
        const SIDE_OFFSET: f32 = 0.02;
        const START_ABOVE: f32 = 0.05;
        let origin = contact + horizontal(direction).normalize_or_zero() * SIDE_OFFSET + Vec3::Y * START_ABOVE;
        let hit = world.raycast(origin, Vec3::NEG_Y, START_ABOVE + self.settings.step_height)?;
        self.is_walkable(hit.normal).then_some(hit.normal)
    }

    fn probe_ground(&mut self, world: &CollisionWorld, reach: f32) {
        let center = self.capsule_center();
        let down = Vec3::Y * -reach;
        let ground = world.capsule_cast(self.settings.movement_capsule(), center, down, None).and_then(|hit| {
            if self.is_walkable(hit.normal) {
                return Some((hit, hit.normal));
            }
            // An edge contact: stand on it if the face beside it, toward where we are going or
            // back under the capsule, is walkable.
            let toward_axis = horizontal(center - hit.point);
            [horizontal(self.velocity), toward_axis]
                .into_iter()
                .filter(|direction| direction.length_squared() > 1e-8)
                .find_map(|direction| self.walkable_face_near(world, hit.point, direction))
                .map(|face_normal| (hit, face_normal))
        });
        match ground {
            Some((hit, normal)) => {
                self.set_capsule_center(center + down * hit.fraction);
                self.grounded = true;
                self.ground_normal = normal;
                self.ground_collider = Some(hit.collider);
            }
            None => {
                self.grounded = false;
                self.ground_collider = None;
                self.ground_normal = Vec3::Y;
            }
        }
    }

    fn depenetrate(&mut self, world: &CollisionWorld) {
        let capsule = self.settings.capsule();
        for _ in 0..MAX_DEPENETRATION_ITERATIONS {
            let contacts = world.capsule_contacts(capsule, self.capsule_center(), None);
            let Some(deepest) = contacts.first() else { break };
            if deepest.depth <= 1e-4 {
                break;
            }
            let center = self.capsule_center() + deepest.normal * (deepest.depth + self.settings.skin_width * 0.5);
            self.set_capsule_center(center);
            let into_surface = self.velocity.dot(deepest.normal);
            if into_surface < 0.0 {
                self.velocity -= deepest.normal * into_surface;
            }
        }
    }
}
