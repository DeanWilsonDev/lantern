use std::path::{Path, PathBuf};

use glam::Vec3;
use lantern_world_format::{
    ColliderKind, EntityKey, PropertyValue, Shape, TransformData, WorldEntity, WorldFile, build_pieces, kinds,
};

fn course_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/worlds/course.world.json")
}

fn marker(id: u64, name: &str, parent: Option<u64>, position: Vec3) -> WorldEntity {
    WorldEntity {
        id: EntityKey(id),
        name: name.to_string(),
        parent: parent.map(EntityKey),
        kind: "marker".to_string(),
        transform: TransformData::from_position(position),
        shape: None,
        material: None,
        tint: None,
        collider: None,
        properties: Default::default(),
    }
}

#[test]
fn the_course_loads_and_is_in_canonical_form() {
    let path = course_path();
    let text = std::fs::read_to_string(&path).unwrap();
    let world = WorldFile::from_json(&text, &path).unwrap();
    assert_eq!(world.to_json(), text, "run the format-world example on the course");
}

#[test]
fn the_course_has_the_demo_level_pieces() {
    let world = WorldFile::load(&course_path()).unwrap();
    assert_eq!(world.entities_of_kind(kinds::BLOCK).count(), 18);
    assert_eq!(world.entities_of_kind(kinds::RAMP).count(), 2);
    assert_eq!(world.entities_of_kind(kinds::GEM).count(), 6);
    assert_eq!(world.entities_of_kind(kinds::SPAWN).count(), 1);
    assert_eq!(world.entities_of_kind(kinds::KILL_PLANE).count(), 1);
    let platform = world.entities_of_kind(kinds::MOVING_PLATFORM).next().unwrap();
    assert_eq!(platform.collider, Some(ColliderKind::Kinematic));
    assert_eq!(platform.number_property("period_seconds"), Some(6.0));
    assert_eq!(platform.properties.get("travel"), Some(&PropertyValue::Vector([0.0, 0.0, -6.0])));
    let spawn = world.entities_of_kind(kinds::SPAWN).next().unwrap();
    let kill = world.entities_of_kind(kinds::KILL_PLANE).next().unwrap();
    assert!(spawn.transform.position[1] > kill.transform.position[1]);
}

#[test]
fn pieces_match_shapes_and_collide_where_drawn() {
    let world = WorldFile::load(&course_path()).unwrap();
    let pieces = build_pieces(&world);
    assert_eq!(pieces.len(), world.entities.len());
    let floor = pieces.iter().find(|piece| piece.name == "start_floor").unwrap();
    let triangles = floor.world_triangles();
    assert_eq!(triangles.len(), 12);
    // The floor's top face is at y = 0.
    let top = triangles.iter().flat_map(|triangle| [triangle.a.y, triangle.b.y, triangle.c.y]).fold(f32::MIN, f32::max);
    assert!((top - 0.0).abs() < 1e-5);
    let gem = pieces.iter().find(|piece| piece.kind == kinds::GEM).unwrap();
    assert!(gem.mesh.is_none() && gem.world_triangles().is_empty());
}

#[test]
fn parents_compose_transforms() {
    let mut world = WorldFile::new("test");
    world.entities.push(marker(1, "parent", None, Vec3::new(1.0, 0.0, 0.0)));
    let mut child = marker(2, "child", Some(1), Vec3::new(0.0, 0.0, -2.0));
    child.shape = Some(Shape::Box { half_extents: [0.5; 3] });
    world.entities.push(child);
    world.entities[0].transform.rotation_degrees = [0.0, 90.0, 0.0];
    let pieces = build_pieces(&world);
    let child_position = pieces[1].world_matrix.transform_point3(Vec3::ZERO);
    // Yaw +90° (counter-clockwise from above) turns -Z into -X.
    assert!((child_position - Vec3::new(-1.0, 0.0, 0.0)).length() < 1e-5, "{child_position}");
}

#[test]
fn validation_reports_broken_worlds() {
    let mut world = WorldFile::new("broken");
    world.entities.push(marker(1, "a", Some(2), Vec3::ZERO));
    world.entities.push(marker(2, "b", Some(1), Vec3::ZERO));
    world.entities.push(marker(2, "c", Some(9), Vec3::ZERO));
    world.entities[0].shape = Some(Shape::Ramp { width: 1.0, height: 0.0, length: 1.0 });
    world.entities[1].tint = Some("red".to_string());
    let problems = world.validate().join("\n");
    assert!(problems.contains("used more than once"), "{problems}");
    assert!(problems.contains("own ancestor"), "{problems}");
    assert!(problems.contains("missing parent #9"), "{problems}");
    assert!(problems.contains("non-positive size"), "{problems}");
    assert!(problems.contains("expected \"#RRGGBB\""), "{problems}");
    let path = Path::new("broken.world.json");
    assert!(WorldFile::from_json(&world.to_json(), path).is_err());
}

#[test]
fn save_round_trips_and_never_reuses_ids() {
    let mut world = WorldFile::new("round trip");
    world.entities.push(marker(4, "a", None, Vec3::new(0.1, 0.2, 0.3)));
    world.entities[0].properties.insert("flag".to_string(), PropertyValue::Bool(true));
    world.entities[0].properties.insert("label".to_string(), PropertyValue::Text("hi".to_string()));
    assert_eq!(world.next_entity_key(), EntityKey(5));
    let directory = std::env::temp_dir().join(format!("lantern-world-format-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let path = directory.join("round.world.json");
    world.save(&path).unwrap();
    assert_eq!(WorldFile::load(&path).unwrap(), world);
    assert!(std::fs::read_to_string(&path).unwrap().contains("[0.1, 0.2, 0.3]"));
    std::fs::remove_dir_all(&directory).unwrap();
}
