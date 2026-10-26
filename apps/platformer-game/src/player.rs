//! The player: a character controller plus the entity that shows it.

use glam::{Quat, Vec2, Vec3};
use lantern_engine::world::{EntityId, World};
use lantern_physics::{CharacterController, CollisionWorld, ControllerEvents, ControllerInput, ControllerSettings};

use crate::assets::{PLAYER_HEIGHT, PLAYER_RADIUS};

/// How fast the body turns to face the direction of travel (radians per second).
const TURN_SPEED: f32 = 12.0;

pub struct Player {
    pub entity: EntityId,
    pub controller: CharacterController,
    pub spawn_position: Vec3,
    /// Facing, radians about +Y (0 faces -Z).
    pub facing_yaw: f32,
}

impl Player {
    pub fn new(entity: EntityId, spawn_position: Vec3) -> Player {
        let settings =
            ControllerSettings { radius: PLAYER_RADIUS, height: PLAYER_HEIGHT, ..ControllerSettings::default() };
        Player {
            entity,
            controller: CharacterController::new(settings, spawn_position),
            spawn_position,
            facing_yaw: 0.0,
        }
    }

    pub fn reset(&mut self) {
        self.controller.teleport(self.spawn_position);
        self.facing_yaw = 0.0;
    }

    /// One fixed step. `stick` is the movement action (x right, y forward), mapped onto the
    /// camera's horizontal basis.
    pub fn fixed_update(
        &mut self,
        collision: &CollisionWorld,
        stick: Vec2,
        camera_basis: (Vec3, Vec3),
        jump_pressed: bool,
        jump_held: bool,
        dt: f32,
    ) -> ControllerEvents {
        let (forward, right) = camera_basis;
        let move_direction = (forward * stick.y + right * stick.x).clamp_length_max(1.0);
        let input = ControllerInput { move_direction, jump_pressed, jump_held };
        let events = self.controller.step(collision, &input, dt);
        if move_direction.length_squared() > 0.01 {
            let target = lantern_core::math::yaw_from_direction(move_direction);
            let difference = lantern_core::math::wrap_angle(target - self.facing_yaw);
            self.facing_yaw += difference.clamp(-TURN_SPEED * dt, TURN_SPEED * dt);
        }
        events
    }

    /// Copies the controller's state onto the entity.
    pub fn sync_entity(&self, world: &mut World) {
        if let Some(entity) = world.get_mut(self.entity) {
            entity.local.translation = self.controller.feet_position;
            entity.local.rotation = Quat::from_rotation_y(self.facing_yaw);
        }
    }

    pub fn teleport_entity(&self, world: &mut World) {
        world.teleport(self.entity, self.controller.feet_position);
    }
}
