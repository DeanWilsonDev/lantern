//! Render extraction (Umbra's render component manager, inverted): the world is read once per
//! display frame and turned into `DrawItem`s. The renderer never sees entities.

use lantern_renderer_3d::RenderWorld;

use crate::world::World;

/// Appends one draw per visible mesh renderer, at its interpolated transform.
pub fn extract_draws(world: &World, alpha: f32, out: &mut RenderWorld) {
    for (_, entity) in world.iter() {
        let Some(renderer) = entity.mesh_renderer else { continue };
        if !renderer.visible {
            continue;
        }
        out.draw(renderer.mesh, renderer.texture, entity.interpolated_world(alpha), renderer.color);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::{MeshRenderer, SceneId, Transform};
    use glam::Vec3;
    use lantern_core::color::Color;
    use lantern_core::handle::Handle;

    #[test]
    fn only_visible_mesh_renderers_are_drawn() {
        let mut world = World::new();
        let mesh = Handle::from_bits(0);
        world.spawn("empty", SceneId(1), Transform::IDENTITY);
        world.spawn_mesh(
            "shown",
            SceneId(1),
            Transform::from_translation(Vec3::X),
            MeshRenderer::new(mesh, None, Color::RED),
        );
        let hidden =
            world.spawn_mesh("hidden", SceneId(1), Transform::IDENTITY, MeshRenderer::new(mesh, None, Color::RED));
        world.get_mut(hidden).unwrap().mesh_renderer.as_mut().unwrap().visible = false;
        let mut render_world = RenderWorld::default();
        extract_draws(&world, 1.0, &mut render_world);
        assert_eq!(render_world.draws.len(), 1);
        assert_eq!(render_world.draws[0].transform.w_axis.truncate(), Vec3::X);
    }
}
