//! The demo level as plain data: static blocks, ramps, one moving platform, gem spots and the
//! spawn point.
//!
//! The level is code rather than a file because the brief puts scene serialization out of
//! scope. Every piece is an axis-aligned box or a ramp, so the same data builds both the render
//! meshes (`scene.rs`) and the collision triangles (`lantern-physics`), and they cannot drift.
//!
//! Layout, seen from above (-Z is "forward", away from the spawn):
//!
//! ```text
//!   z = +8   back wall
//!            start floor (16 x 16 m) with stairs (left), steep ramp (left),
//!            low ceiling (left), gentle ramp to a ledge (right)
//!   z = -8   edge of the floor: below is the pit (fall = reset)
//!            pillar 1 → pillar 2 → pillar 3 (rising)
//!            moving platform (back and forth along Z)
//!   z = -31  goal platform
//! ```

use glam::Vec3;

/// Which texture a surface uses. Purely visual.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SurfaceMaterial {
    Ground,
    Wall,
    Platform,
}

#[derive(Clone, Debug)]
pub struct LevelBlock {
    pub name: &'static str,
    pub center: Vec3,
    pub half_extents: Vec3,
    pub material: SurfaceMaterial,
}

/// A wedge rising toward -Z. `base_center` is the middle of its footprint on the ground, which
/// is the origin of `primitives::ramp`.
#[derive(Clone, Debug)]
pub struct LevelRamp {
    pub name: &'static str,
    pub base_center: Vec3,
    pub width: f32,
    pub height: f32,
    pub length: f32,
}

impl LevelRamp {
    #[cfg(test)]
    pub fn slope_degrees(&self) -> f32 {
        (self.height / self.length).atan().to_degrees()
    }
}

/// A platform that moves between two points. The motion itself is a game rule (see the
/// scripts); the level only says where the platform starts and how big it is.
#[derive(Clone, Debug)]
pub struct LevelMovingPlatform {
    pub name: &'static str,
    pub start_center: Vec3,
    pub half_extents: Vec3,
}

#[derive(Clone, Debug)]
pub struct Level {
    pub blocks: Vec<LevelBlock>,
    pub ramps: Vec<LevelRamp>,
    pub moving_platforms: Vec<LevelMovingPlatform>,
    /// Gem centres. Gems float about 0.6 m above the surface they sit on.
    pub gem_positions: Vec<Vec3>,
    /// Where the player's feet start.
    pub spawn_position: Vec3,
    /// Falling below this height resets the player to the spawn.
    pub kill_height: f32,
}

/// A pillar standing out of the pit: `top` is the walkable height, `size` its square footprint.
fn pillar(name: &'static str, x: f32, z: f32, top: f32, size: f32) -> LevelBlock {
    const PILLAR_BOTTOM: f32 = -6.0;
    LevelBlock {
        name,
        center: Vec3::new(x, (top + PILLAR_BOTTOM) * 0.5, z),
        half_extents: Vec3::new(size * 0.5, (top - PILLAR_BOTTOM) * 0.5, size * 0.5),
        material: SurfaceMaterial::Platform,
    }
}

fn block(name: &'static str, center: Vec3, half_extents: Vec3, material: SurfaceMaterial) -> LevelBlock {
    LevelBlock { name, center, half_extents, material }
}

pub fn demo_level() -> Level {
    use SurfaceMaterial::{Ground, Platform, Wall};
    let blocks = vec![
        block("start_floor", Vec3::new(0.0, -0.5, 0.0), Vec3::new(8.0, 0.5, 8.0), Ground),
        block("back_wall", Vec3::new(0.0, 1.0, 8.5), Vec3::new(8.5, 1.0, 0.5), Wall),
        block("left_wall", Vec3::new(-8.5, 1.0, 0.0), Vec3::new(0.5, 1.0, 8.0), Wall),
        block("right_wall", Vec3::new(8.5, 1.0, 0.0), Vec3::new(0.5, 1.0, 8.0), Wall),
        // Stairs: 0.25 m risers, below the controller's step height, so walking climbs them.
        block("stair_1", Vec3::new(-5.5, 0.125, 5.0), Vec3::new(1.5, 0.125, 1.5), Platform),
        block("stair_2", Vec3::new(-5.5, 0.25, 3.0), Vec3::new(1.5, 0.25, 0.5), Platform),
        block("stair_3", Vec3::new(-5.5, 0.375, 2.0), Vec3::new(1.5, 0.375, 0.5), Platform),
        block("stair_top", Vec3::new(-5.5, 0.5, 0.5), Vec3::new(1.5, 0.5, 1.0), Platform),
        // A low canopy near the start: jumping under it bumps your head.
        block("low_ceiling", Vec3::new(2.0, 2.15, 4.0), Vec3::new(1.5, 0.15, 1.5), Wall),
        block("canopy_post_1", Vec3::new(0.6, 1.0, 2.6), Vec3::new(0.1, 1.0, 0.1), Wall),
        block("canopy_post_2", Vec3::new(3.4, 1.0, 2.6), Vec3::new(0.1, 1.0, 0.1), Wall),
        block("canopy_post_3", Vec3::new(0.6, 1.0, 5.4), Vec3::new(0.1, 1.0, 0.1), Wall),
        block("canopy_post_4", Vec3::new(3.4, 1.0, 5.4), Vec3::new(0.1, 1.0, 0.1), Wall),
        // Ledge at the top of the gentle ramp.
        block("ramp_ledge", Vec3::new(5.0, 0.75, -6.5), Vec3::new(1.5, 0.75, 1.5), Platform),
        // The jump course over the pit.
        pillar("pillar_1", 0.0, -11.0, 0.5, 2.0),
        pillar("pillar_2", 2.5, -14.5, 1.2, 2.0),
        pillar("pillar_3", 0.0, -18.0, 2.0, 2.0),
        pillar("goal", 0.0, -31.0, 2.0, 5.0),
    ];
    let ramps = vec![
        // 1.5 m over 4 m: about 20 degrees, walkable.
        LevelRamp { name: "gentle_ramp", base_center: Vec3::new(5.0, 0.0, -3.0), width: 3.0, height: 1.5, length: 4.0 },
        // 2.5 m over 1.5 m: about 59 degrees, steeper than the walkable limit, so you slide.
        LevelRamp { name: "steep_ramp", base_center: Vec3::new(-5.5, 0.0, -3.0), width: 3.0, height: 2.5, length: 1.5 },
    ];
    let moving_platforms = vec![LevelMovingPlatform {
        name: "moving_platform",
        start_center: Vec3::new(0.0, 1.8, -21.0),
        half_extents: Vec3::new(1.25, 0.2, 1.25),
    }];
    let gem_positions = vec![
        Vec3::new(-5.5, 1.6, 0.5),
        Vec3::new(5.0, 2.1, -6.5),
        Vec3::new(0.0, 1.1, -11.0),
        Vec3::new(2.5, 1.8, -14.5),
        Vec3::new(0.0, 2.6, -18.0),
        Vec3::new(0.0, 2.6, -31.0),
    ];
    Level {
        blocks,
        ramps,
        moving_platforms,
        gem_positions,
        spawn_position: Vec3::new(-1.0, 0.0, 4.0),
        kill_height: -8.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ramps_straddle_the_walkable_limit() {
        let level = demo_level();
        let slope = |name| level.ramps.iter().find(|ramp| ramp.name == name).unwrap().slope_degrees();
        assert!(slope("gentle_ramp") < 30.0);
        assert!(slope("steep_ramp") > 50.0);
    }

    #[test]
    fn names_are_unique() {
        let level = demo_level();
        let mut names: Vec<&str> = level.blocks.iter().map(|block| block.name).collect();
        names.extend(level.ramps.iter().map(|ramp| ramp.name));
        names.extend(level.moving_platforms.iter().map(|platform| platform.name));
        let count = names.len();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), count);
    }

    #[test]
    fn spawn_is_above_the_kill_height() {
        let level = demo_level();
        assert!(level.spawn_position.y > level.kill_height);
    }
}
