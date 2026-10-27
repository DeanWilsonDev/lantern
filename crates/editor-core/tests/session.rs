//! Drives `EditorSession` the way the front end and viewport do: clicks, drags, keys, inspector
//! edits, undo and redo, and saving.

use std::path::{Path, PathBuf};

use glam::{Vec2, Vec3};
use lantern_editor_core::gizmo::GizmoFrame;
use lantern_editor_core::{
    Axis, EditorKey, EditorSession, GizmoTool, Modifiers, Palette, PointerButton, ViewportProjection,
};
use lantern_world_format::{EntityKey, WorldFile};

fn course_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/worlds/course.world.json")
}

fn palette() -> Palette {
    Palette::load(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/editor/palette.json")).unwrap()
}

fn course_session() -> EditorSession {
    let mut session = EditorSession::open(&course_path(), palette()).unwrap();
    session.set_viewport_size(Vec2::new(1280.0, 720.0));
    session
}

fn key_named(session: &EditorSession, name: &str) -> EntityKey {
    session.world().entities.iter().find(|entity| entity.name == name).unwrap().id
}

fn position(session: &EditorSession, key: EntityKey) -> Vec3 {
    Vec3::from_array(session.world().entity(key).unwrap().transform.position)
}

fn click(session: &mut EditorSession, pixel: Vec2, modifiers: Modifiers) {
    session.pointer_moved(pixel);
    session.pointer_button(PointerButton::Left, true, modifiers);
    session.pointer_button(PointerButton::Left, false, modifiers);
}

fn drag(session: &mut EditorSession, from: Vec2, to: Vec2) {
    session.pointer_moved(from);
    session.pointer_button(PointerButton::Left, true, Modifiers::default());
    for step in 1..=10 {
        session.pointer_moved(from + (to - from) * step as f32 / 10.0);
    }
    session.pointer_button(PointerButton::Left, false, Modifiers::default());
}

fn pixel_of(session: &EditorSession, point: Vec3) -> Vec2 {
    session.projection().project(point).unwrap()
}

#[test]
fn clicking_selects_and_clicking_empty_space_clears() {
    let mut session = course_session();
    let pillar = key_named(&session, "pillar_3");
    let top = position(&session, pillar) + Vec3::Y * 4.0;
    let pixel = pixel_of(&session, top);
    click(&mut session, pixel, Modifiers::default());
    assert_eq!(session.selection(), &[pillar]);
    assert_eq!(session.inspector()[0].value, "pillar_3");
    let sky = Vec2::new(640.0, 2.0);
    click(&mut session, sky, Modifiers::default());
    assert!(session.selection().is_empty());
}

#[test]
fn dragging_the_x_arrow_moves_by_snapped_steps_and_undoes_in_one_step() {
    let mut session = course_session();
    let pillar = key_named(&session, "pillar_2");
    session.select_only(pillar);
    let start = position(&session, pillar);
    let projection: ViewportProjection = session.projection();
    let frame = GizmoFrame::new(GizmoTool::Translate, start, glam::Quat::IDENTITY, &projection);
    let grab = projection.project(start + Vec3::X * frame.size * 0.8).unwrap();
    let release = projection.project(start + Vec3::X * (frame.size * 0.8 + 3.1)).unwrap();
    drag(&mut session, grab, release);
    let moved = position(&session, pillar);
    assert!((moved.x - (start.x + 3.0)).abs() < 1e-4, "snapped to 0.25 m: {moved}");
    assert_eq!(moved.y, start.y);
    assert_eq!(moved.z, start.z);
    assert!(session.document().is_dirty());
    assert_eq!(session.document().undo_label(), Some("Move entity"));
    assert!(session.undo());
    assert_eq!(position(&session, pillar), start);
    assert!(!session.document().is_dirty());
    assert!(session.redo());
    assert!((position(&session, pillar).x - (start.x + 3.0)).abs() < 1e-4);
}

#[test]
fn dragging_a_selected_body_slides_it_on_the_ground_plane() {
    let mut session = course_session();
    let gem = key_named(&session, "gem_0");
    let start = position(&session, gem);
    let from = pixel_of(&session, start);
    let to = pixel_of(&session, start + Vec3::new(1.0, 0.0, -2.0));
    drag(&mut session, from, to);
    assert_eq!(session.selection(), &[gem]);
    let moved = position(&session, gem);
    assert!((moved - (start + Vec3::new(1.0, 0.0, -2.0))).length() < 0.26, "{moved}");
    assert_eq!(moved.y, start.y);
}

#[test]
fn escape_cancels_a_drag() {
    let mut session = course_session();
    let pillar = key_named(&session, "pillar_1");
    session.select_only(pillar);
    let start = position(&session, pillar);
    let projection = session.projection();
    let frame = GizmoFrame::new(GizmoTool::Translate, start, glam::Quat::IDENTITY, &projection);
    let grab = projection.project(start + Vec3::Y * frame.size * 0.8).unwrap();
    session.pointer_moved(grab);
    session.pointer_button(PointerButton::Left, true, Modifiers::default());
    session.pointer_moved(grab - Vec2::new(0.0, 80.0));
    assert_ne!(position(&session, pillar), start);
    assert!(session.key(EditorKey::Escape, true, Modifiers::default()));
    session.pointer_button(PointerButton::Left, false, Modifiers::default());
    assert_eq!(position(&session, pillar), start);
    assert!(!session.document().can_undo());
}

#[test]
fn rotate_and_scale_tools() {
    let mut session = course_session();
    let ramp = key_named(&session, "gentle_ramp");
    session.select_only(ramp);
    assert!(session.key(EditorKey::Letter('e'), true, Modifiers::default()));
    assert_eq!(session.tool(), GizmoTool::Rotate);
    let pivot = position(&session, ramp);
    let projection = session.projection();
    let frame = GizmoFrame::new(GizmoTool::Rotate, pivot, glam::Quat::IDENTITY, &projection);
    // Grab the yaw ring at one point and drag a quarter turn around it.
    let tangent = Vec3::Y.any_orthonormal_vector();
    let bitangent = Vec3::Y.cross(tangent);
    let radius = frame.size * 0.85;
    // Rings cross each other at 0° and 90°, so grab and release between them.
    let ring_point = |degrees: f32| {
        let angle = degrees.to_radians();
        pivot + (tangent * angle.cos() + bitangent * angle.sin()) * radius
    };
    let grab = projection.project(ring_point(45.0)).unwrap();
    assert_eq!(frame.hit_test(GizmoTool::Rotate, &projection, grab), Some(Axis::Y));
    let release = projection.project(ring_point(135.0)).unwrap();
    drag(&mut session, grab, release);
    let rotation = session.world().entity(ramp).unwrap().transform.rotation_degrees;
    assert!((rotation[1].abs() - 90.0).abs() < 1e-3, "{rotation:?}");

    session.key(EditorKey::Letter('r'), true, Modifiers::default());
    assert_eq!(session.tool(), GizmoTool::Scale);
    session.set_field(ramp, "rotation.y", "0");
    session.commit_field();
    let projection = session.projection();
    let frame = GizmoFrame::new(GizmoTool::Scale, pivot, glam::Quat::IDENTITY, &projection);
    let grab = projection.project(pivot + Vec3::X * frame.size * 0.8).unwrap();
    let release = projection.project(pivot + Vec3::X * frame.size * 1.6).unwrap();
    drag(&mut session, grab, release);
    let scale = session.world().entity(ramp).unwrap().transform.scale;
    assert!((scale[0] - 2.0).abs() < 1e-3, "{scale:?}");
    assert_eq!(scale[1], 1.0);
}

#[test]
fn typing_into_a_field_is_one_undo_step() {
    let mut session = course_session();
    let floor = key_named(&session, "start_floor");
    session.select_only(floor);
    assert!(session.set_field(floor, "position.y", "-1"));
    assert!(session.set_field(floor, "position.y", "-1."));
    assert!(session.set_field(floor, "position.y", "-1.5"));
    assert!(!session.set_field(floor, "position.y", "-1.5x"), "bad input is refused");
    assert!(session.status().contains("not a number"));
    assert_eq!(position(&session, floor).y, -1.5);
    session.commit_field();
    assert!(session.set_field(floor, "name", "floor"));
    assert!(session.undo());
    assert_eq!(session.world().entity(floor).unwrap().name, "start_floor");
    assert!(session.undo());
    assert_eq!(position(&session, floor).y, -0.5);
    assert!(!session.document().is_dirty());
}

#[test]
fn add_duplicate_and_delete() {
    let mut session = course_session();
    let count = session.world().entities.len();
    let added = session.add_entity("block", false).unwrap();
    assert_eq!(session.world().entities.len(), count + 1);
    assert_eq!(session.selection(), &[added]);
    let block = session.world().entity(added).unwrap();
    assert_eq!(block.kind, "block");
    assert_eq!(block.name, "block_1");
    assert!(session.key(EditorKey::Letter('d'), true, Modifiers::from_bits(Modifiers::COMMAND)));
    assert_eq!(session.world().entities.len(), count + 2);
    let copy = session.primary_selection().unwrap();
    assert_eq!(session.world().entity(copy).unwrap().name, "block_2");
    session.select_only(added);
    session.toggle_selected(copy);
    assert!(session.key(EditorKey::Delete, true, Modifiers::default()));
    assert_eq!(session.world().entities.len(), count);
    assert!(session.undo());
    assert_eq!(session.world().entities.len(), count + 2);
    assert!(session.world().entity(added).is_some() && session.world().entity(copy).is_some());
}

#[test]
fn deleting_a_parent_takes_its_children_and_outline_nests_them() {
    let mut session = course_session();
    let floor = key_named(&session, "start_floor");
    let post = key_named(&session, "canopy_post_1");
    assert!(session.set_parent(post, Some(floor)));
    // Re-parenting keeps the post where it is in the world.
    let world_position = Vec3::new(0.6, 1.0, 2.6);
    let local = position(&session, post);
    assert!((local - (world_position - Vec3::new(0.0, -0.5, 0.0))).length() < 1e-4, "{local}");
    let outline = session.outline();
    let floor_row = outline.iter().position(|row| row.key == floor).unwrap();
    assert_eq!(outline[floor_row + 1].key, post);
    assert_eq!(outline[floor_row + 1].depth, 1);
    assert_eq!(outline.len(), session.world().entities.len());
    assert!(!session.set_parent(floor, Some(post)), "no cycles");
    session.select_only(floor);
    let count = session.world().entities.len();
    session.delete_selection();
    assert_eq!(session.world().entities.len(), count - 2);
    session.undo();
    assert_eq!(session.world().entity(post).unwrap().parent, Some(floor));
}

#[test]
fn saving_round_trips_and_clears_dirty() {
    let mut session = course_session();
    let directory = std::env::temp_dir().join(format!("lantern-editor-core-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let path = directory.join("copy.world.json");
    let gem = key_named(&session, "gem_3");
    session.set_field(gem, "property:value", "5");
    assert!(session.title().ends_with('*'));
    assert!(session.save_as(&path));
    assert!(!session.document().is_dirty());
    assert_eq!(session.title(), "copy.world.json");
    let reloaded = WorldFile::load(&path).unwrap();
    assert_eq!(&reloaded, session.world());
    assert_eq!(reloaded.entity(gem).unwrap().number_property("value"), Some(5.0));
    std::fs::remove_dir_all(&directory).unwrap();
}

#[test]
fn camera_controls() {
    let mut session = course_session();
    let before = session.camera;
    session.pointer_moved(Vec2::new(600.0, 300.0));
    session.pointer_button(PointerButton::Right, true, Modifiers::default());
    session.pointer_moved(Vec2::new(650.0, 300.0));
    assert_ne!(session.camera.yaw, before.yaw);
    session.key(EditorKey::Letter('w'), true, Modifiers::default());
    assert_eq!(session.tool(), GizmoTool::Translate, "W flies instead of switching tools while orbiting");
    let target = session.camera.target;
    session.update(0.5);
    assert!((session.camera.target - target).length() > 1.0);
    session.key(EditorKey::Letter('w'), false, Modifiers::default());
    session.pointer_button(PointerButton::Right, false, Modifiers::default());
    let distance = session.camera.distance;
    session.wheel(2.0);
    assert!(session.camera.distance < distance);
    session.select_only(key_named(&session, "goal"));
    session.key(EditorKey::Letter('f'), true, Modifiers::default());
    assert!((session.camera.target - Vec3::new(0.0, -2.0, -31.0)).length() < 1e-3);
}
