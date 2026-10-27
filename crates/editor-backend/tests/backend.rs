//! The backend through its C API, the way the C++ shell uses it. The viewport render needs a
//! display and a GPU (or lavapipe); without one that part prints why and passes, unless
//! `LANTERN_REQUIRE_GPU=1`.

use std::ffi::{CStr, CString};
use std::path::Path;

use lantern_editor_backend::*;

fn assets() -> CString {
    CString::new(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets").to_str().unwrap()).unwrap()
}

fn text(pointer: *const std::ffi::c_char) -> String {
    unsafe { CStr::from_ptr(pointer) }.to_str().unwrap().to_string()
}

#[test]
fn panels_through_the_c_api() {
    unsafe {
        let editor = lantern_editor_create(assets().as_ptr(), std::ptr::null());
        assert!(!editor.is_null());
        assert_eq!(text(lantern_editor_title(editor)), "course.world.json");
        let rows = lantern_editor_outline_count(editor);
        assert_eq!(rows, 29);
        let first = lantern_editor_outline_id(editor, 0);
        assert_eq!(text(lantern_editor_entity_name(editor, first)), "start_floor");
        assert_eq!(lantern_editor_field_count(editor), 0);

        let revision = lantern_editor_revision(editor);
        lantern_editor_select(editor, first, false);
        assert!(lantern_editor_revision(editor) > revision);
        assert_eq!(lantern_editor_primary_selection(editor), first);
        let count = lantern_editor_field_count(editor);
        let fields: Vec<(String, String)> = (0..count)
            .map(|index| (text(lantern_editor_field(editor, index, 0)), text(lantern_editor_field(editor, index, 3))))
            .collect();
        assert!(fields.contains(&("position.y".to_string(), "-0.5".to_string())), "{fields:?}");
        let material = (0..count).find(|index| text(lantern_editor_field(editor, *index, 0)) == "material").unwrap();
        assert_eq!(text(lantern_editor_field(editor, material, 4)), "choice");
        assert!(lantern_editor_field_choice_count(editor, material) >= 3);

        let field = CString::new("position.y").unwrap();
        let bad = CString::new("down").unwrap();
        assert!(!lantern_editor_set_field(editor, first, field.as_ptr(), bad.as_ptr()));
        assert!(text(lantern_editor_status(editor)).contains("not a number"));
        let good = CString::new("-2").unwrap();
        assert!(lantern_editor_set_field(editor, first, field.as_ptr(), good.as_ptr()));
        assert!(lantern_editor_is_dirty(editor));
        assert!(lantern_editor_undo(editor));
        assert!(!lantern_editor_is_dirty(editor));

        assert_eq!(text(lantern_editor_template(editor, 0, 1)), "Block");
        let block = CString::new("block").unwrap();
        let added = lantern_editor_add_entity(editor, block.as_ptr());
        assert!(added > 0);
        assert_eq!(lantern_editor_outline_count(editor), 30);
        let rotate = CString::new("rotate").unwrap();
        lantern_editor_set_tool(editor, rotate.as_ptr());
        assert_eq!(text(lantern_editor_tool(editor)), "rotate");
        let w = CString::new("w").unwrap();
        assert!(lantern_editor_key(editor, w.as_ptr(), true, 0));
        assert_eq!(text(lantern_editor_tool(editor)), "move");
        assert!(lantern_editor_delete_selection(editor));
        assert_eq!(lantern_editor_outline_count(editor), 29);
        lantern_editor_destroy(editor);
    }
}

#[test]
fn the_tools_through_the_c_api() {
    unsafe {
        let editor = lantern_editor_create(assets().as_ptr(), std::ptr::null());
        let c = |s: &str| CString::new(s).unwrap();

        assert_eq!(lantern_editor_entity_count(editor), 29);
        let first = lantern_editor_outline_id(editor, 0);
        assert_eq!(lantern_editor_outline_parent(editor, 0), 0);
        lantern_editor_set_hidden(editor, first, true);
        assert!(lantern_editor_is_hidden(editor, first));
        lantern_editor_unhide_all(editor);
        assert!(!lantern_editor_is_hidden(editor, first));

        lantern_editor_set_space(editor, c("local").as_ptr());
        assert_eq!(text(lantern_editor_space(editor)), "local");
        lantern_editor_set_pivot_mode(editor, c("center").as_ptr());
        assert_eq!(text(lantern_editor_pivot_mode(editor)), "center");
        assert!(lantern_editor_set_snap_step(editor, c("translate").as_ptr(), 0.5));
        assert_eq!(lantern_editor_snap_step(editor, c("translate").as_ptr()), 0.5);
        assert!(lantern_editor_set_view_option(editor, c("wireframe").as_ptr(), c("true").as_ptr()));
        assert_eq!(text(lantern_editor_view_option(editor, c("wireframe").as_ptr())), "true");
        assert!(!lantern_editor_set_view_option(editor, c("nonsense").as_ptr(), c("true").as_ptr()));

        assert!(lantern_editor_camera_preset(editor, c("top").as_ptr()));
        assert!(lantern_editor_is_orthographic(editor));
        assert!(text(lantern_editor_camera_description(editor)).starts_with("ortho"));
        assert!(lantern_editor_set_bookmark(editor, 0));
        assert!(lantern_editor_has_bookmark(editor, 0));
        assert!(!lantern_editor_has_bookmark(editor, 1));

        lantern_editor_select(editor, first, false);
        let second = lantern_editor_outline_id(editor, 1);
        lantern_editor_select(editor, second, true);
        assert_eq!(lantern_editor_selection_count(editor), 2);
        assert!(lantern_editor_set_selection_field(editor, c("property:speed").as_ptr(), c("2").as_ptr()));
        assert_eq!(text(lantern_editor_field(editor, lantern_editor_field_count(editor) - 1, 5)), "false");
        assert!(lantern_editor_group_selection(editor));
        assert_eq!(lantern_editor_entity_count(editor), 30);
        assert_eq!(lantern_editor_history_count(editor), 2);
        assert_eq!(lantern_editor_history_position(editor), 2);
        assert_eq!(text(lantern_editor_history_label(editor, 0)), "Edit speed on 2 entities");
        assert!(lantern_editor_jump_history(editor, 0));
        assert_eq!(lantern_editor_entity_count(editor), 29);
        assert_eq!(lantern_editor_history_count(editor), 2, "undone steps stay in the list");
        assert!(lantern_editor_log_count(editor) > 0);
        assert_eq!(text(lantern_editor_log_level(editor, 0)), "info");
        lantern_editor_clear_log(editor);
        assert_eq!(lantern_editor_log_count(editor), 0);

        assert!(lantern_editor_set_preference(editor, c("window.width").as_ptr(), c("1600").as_ptr()));
        assert_eq!(text(lantern_editor_preference(editor, c("window.width").as_ptr())), "1600");
        assert_eq!(text(lantern_editor_stat(editor, c("entities").as_ptr())), "29");
        assert!(!lantern_editor_is_playing(editor));
        assert!(text(lantern_editor_assets_directory(editor)).ends_with("assets"));
        lantern_editor_destroy(editor);
    }
}

#[test]
fn the_viewport_renders_the_course() {
    let require = std::env::var("LANTERN_REQUIRE_GPU").is_ok_and(|value| value == "1");
    unsafe {
        let editor = lantern_editor_create(assets().as_ptr(), std::ptr::null());
        if !lantern_editor_render(editor, 640, 360) {
            let reason = text(lantern_editor_status(editor));
            assert!(!require, "GPU required: {reason}");
            eprintln!("skipping viewport render: {reason}");
            lantern_editor_destroy(editor);
            return;
        }
        // Select a pillar so the outline and gizmo are drawn too, and render again.
        let pillar = (0..lantern_editor_outline_count(editor))
            .map(|row| lantern_editor_outline_id(editor, row))
            .find(|id| text(lantern_editor_entity_name(editor, *id)) == "pillar_2")
            .unwrap();
        lantern_editor_select(editor, pillar, false);
        assert!(lantern_editor_render(editor, 640, 360));
        let (mut width, mut height) = (0, 0);
        let pixels = lantern_editor_pixels(editor, &mut width, &mut height);
        assert_eq!((width, height), (640, 360));
        let bytes = std::slice::from_raw_parts(pixels, (width * height * 4) as usize);
        // The sky is a flat colour; the level must cover a good part of the image.
        let sky = &bytes[0..4];
        let different = bytes.chunks_exact(4).filter(|pixel| pixel != &sky).count();
        assert!(different > (width * height / 5) as usize, "only {different} pixels differ from the sky");
        if let Ok(path) = std::env::var("LANTERN_EDITOR_CAPTURE") {
            let image = lantern_renderer_3d::TextureData {
                width,
                height,
                rgba8: bytes.to_vec(),
                filter: lantern_renderer_3d::texture::TextureFilter::Nearest,
                wrap: lantern_renderer_3d::texture::TextureWrap::Clamp,
            };
            std::fs::write(path, image.to_png_bytes()).unwrap();
        }
        lantern_editor_destroy(editor);
    }
}
