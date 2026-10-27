//! The second half of the C API (`include/lantern_editor.h`): the editor tools added for the
//! fleshed-out editor. Same rules as `ffi.rs`: one thread, borrowed strings in, strings out
//! valid until the next string-returning call, no panic crosses the boundary.

use std::ffi::{c_char, c_float};
use std::path::Path;

use lantern_editor_core::{Axis, GizmoSpace, PivotMode};

use crate::ffi::{LanternEditor, ReturnedString, id, key, read_string, with_editor};

/// Returns a string the editor computes, or "" for a null handle.
///
/// # Safety
/// See the module rules.
unsafe fn string_from(handle: *mut LanternEditor, compute: impl FnOnce(&mut LanternEditor) -> String) -> *const c_char {
    // SAFETY: forwarded from the caller.
    unsafe {
        with_editor(handle, |editor| {
            let text = compute(editor);
            ReturnedString(editor.give_string(&text))
        })
    }
    .0
}

fn axis_from(name: *const c_char) -> Option<Axis> {
    // SAFETY: the caller passes null or a NUL-terminated string.
    Axis::from_name(unsafe { read_string(name) })
}

// ----- Recent files and preferences --------------------------------------------------------------

/// # Safety
/// See the module rules.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lantern_editor_recent_file_count(handle: *mut LanternEditor) -> u32 {
    // SAFETY: see the module rules.
    unsafe { with_editor(handle, |editor| editor.backend.session.recent_files().len() as u32) }
}

/// # Safety
/// See the module rules.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lantern_editor_recent_file(handle: *mut LanternEditor, index: u32) -> *const c_char {
    // SAFETY: see the module rules.
    unsafe {
        string_from(handle, |editor| {
            editor
                .backend
                .session
                .recent_files()
                .get(index as usize)
                .map(|path| path.display().to_string())
                .unwrap_or_default()
        })
    }
}

/// A preference the front end keeps (`window.width`, `split.outliner`...). "" when unset.
///
/// # Safety
/// See the module rules.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lantern_editor_preference(handle: *mut LanternEditor, name: *const c_char) -> *const c_char {
    // SAFETY: see the module rules.
    let name = unsafe { read_string(name) }.to_string();
    // SAFETY: see the module rules.
    unsafe { string_from(handle, |editor| editor.backend.preferences.get(&name).unwrap_or_default().to_string()) }
}

/// Sets a preference; "" removes it. Nothing is written until `lantern_editor_save_preferences`.
///
/// # Safety
/// See the module rules.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lantern_editor_set_preference(
    handle: *mut LanternEditor,
    name: *const c_char,
    value: *const c_char,
) -> bool {
    // SAFETY: see the module rules.
    let (name, value) = unsafe { (read_string(name).to_string(), read_string(value).to_string()) };
    // SAFETY: see the module rules.
    unsafe { with_editor(handle, |editor| editor.backend.preferences.set(&name, &value)) }
}

/// # Safety
/// See the module rules.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lantern_editor_save_preferences(handle: *mut LanternEditor) -> bool {
    // SAFETY: see the module rules.
    unsafe { with_editor(handle, |editor| editor.backend.save_preferences()) }
}

// ----- Gizmo space, pivot, snapping and view options --------------------------------------------

/// `world` or `local`.
///
/// # Safety
/// See the module rules.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lantern_editor_set_space(handle: *mut LanternEditor, name: *const c_char) {
    // SAFETY: see the module rules.
    let Some(space) = GizmoSpace::from_name(unsafe { read_string(name) }) else { return };
    // SAFETY: see the module rules.
    unsafe { with_editor(handle, |editor| editor.backend.session.set_space(space)) }
}

/// # Safety
/// See the module rules.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lantern_editor_space(handle: *mut LanternEditor) -> *const c_char {
    // SAFETY: see the module rules.
    unsafe { string_from(handle, |editor| editor.backend.session.space().name().to_string()) }
}

/// `primary` or `center`.
///
/// # Safety
/// See the module rules.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lantern_editor_set_pivot_mode(handle: *mut LanternEditor, name: *const c_char) {
    // SAFETY: see the module rules.
    let Some(mode) = PivotMode::from_name(unsafe { read_string(name) }) else { return };
    // SAFETY: see the module rules.
    unsafe { with_editor(handle, |editor| editor.backend.session.set_pivot_mode(mode)) }
}

/// # Safety
/// See the module rules.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lantern_editor_pivot_mode(handle: *mut LanternEditor) -> *const c_char {
    // SAFETY: see the module rules.
    unsafe { string_from(handle, |editor| editor.backend.session.pivot_mode().name().to_string()) }
}

/// Sets the `translate`, `rotate` (degrees) or `scale` snap step.
///
/// # Safety
/// See the module rules.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lantern_editor_set_snap_step(
    handle: *mut LanternEditor,
    name: *const c_char,
    value: c_float,
) -> bool {
    // SAFETY: see the module rules.
    let name = unsafe { read_string(name) }.to_string();
    // SAFETY: see the module rules.
    unsafe { with_editor(handle, |editor| editor.backend.session.set_snap_step(&name, value)) }
}

/// # Safety
/// See the module rules.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lantern_editor_snap_step(handle: *mut LanternEditor, name: *const c_char) -> c_float {
    // SAFETY: see the module rules.
    let name = unsafe { read_string(name) }.to_string();
    // SAFETY: see the module rules.
    unsafe { with_editor(handle, |editor| editor.backend.session.snap_step(&name).unwrap_or(0.0)) }
}

/// Sets a view option (`grid`, `grid_spacing`, `colliders`, `markers`, `wireframe`,
/// `level_bounds`, `hidden`, `stats`) from text: `true`/`false`, or a number for the spacing.
///
/// # Safety
/// See the module rules.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lantern_editor_set_view_option(
    handle: *mut LanternEditor,
    name: *const c_char,
    value: *const c_char,
) -> bool {
    // SAFETY: see the module rules.
    let (name, value) = unsafe { (read_string(name).to_string(), read_string(value).to_string()) };
    // SAFETY: see the module rules.
    unsafe { with_editor(handle, |editor| editor.backend.session.set_view_option(&name, &value)) }
}

/// # Safety
/// See the module rules.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lantern_editor_view_option(handle: *mut LanternEditor, name: *const c_char) -> *const c_char {
    // SAFETY: see the module rules.
    let name = unsafe { read_string(name) }.to_string();
    // SAFETY: see the module rules.
    unsafe { string_from(handle, |editor| editor.backend.session.view.get(&name).unwrap_or_default()) }
}

/// The gizmo handle under the pointer ("X", "XZ", "all"), "" when none.
///
/// # Safety
/// See the module rules.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lantern_editor_hovered_handle(handle: *mut LanternEditor) -> *const c_char {
    // SAFETY: see the module rules.
    unsafe {
        string_from(handle, |editor| {
            editor.backend.session.hovered_handle().map(|handle| handle.describe()).unwrap_or_default()
        })
    }
}

// ----- Camera ----------------------------------------------------------------------------------

/// `perspective`, `front`, `back`, `left`, `right`, `top` or `bottom`.
///
/// # Safety
/// See the module rules.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lantern_editor_camera_preset(handle: *mut LanternEditor, name: *const c_char) -> bool {
    // SAFETY: see the module rules.
    let name = unsafe { read_string(name) }.to_string();
    // SAFETY: see the module rules.
    unsafe { with_editor(handle, |editor| editor.backend.session.camera_preset(&name)) }
}

/// # Safety
/// See the module rules.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lantern_editor_set_orthographic(handle: *mut LanternEditor, orthographic: bool) {
    // SAFETY: see the module rules.
    unsafe { with_editor(handle, |editor| editor.backend.session.set_orthographic(orthographic)) }
}

/// # Safety
/// See the module rules.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lantern_editor_is_orthographic(handle: *mut LanternEditor) -> bool {
    // SAFETY: see the module rules.
    unsafe { with_editor(handle, |editor| editor.backend.session.camera.orthographic) }
}

/// # Safety
/// See the module rules.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lantern_editor_camera_description(handle: *mut LanternEditor) -> *const c_char {
    // SAFETY: see the module rules.
    unsafe { string_from(handle, |editor| editor.backend.session.camera_description()) }
}

/// # Safety
/// See the module rules.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lantern_editor_frame_all(handle: *mut LanternEditor) {
    // SAFETY: see the module rules.
    unsafe { with_editor(handle, |editor| editor.backend.session.frame_all()) }
}

/// Camera bookmarks are slots 0 to 8 (shown as 1 to 9).
///
/// # Safety
/// See the module rules.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lantern_editor_set_bookmark(handle: *mut LanternEditor, slot: u32) -> bool {
    // SAFETY: see the module rules.
    unsafe { with_editor(handle, |editor| editor.backend.session.set_bookmark(slot as usize)) }
}

/// # Safety
/// See the module rules.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lantern_editor_go_to_bookmark(handle: *mut LanternEditor, slot: u32) -> bool {
    // SAFETY: see the module rules.
    unsafe { with_editor(handle, |editor| editor.backend.session.go_to_bookmark(slot as usize)) }
}

/// # Safety
/// See the module rules.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lantern_editor_has_bookmark(handle: *mut LanternEditor, slot: u32) -> bool {
    // SAFETY: see the module rules.
    unsafe { with_editor(handle, |editor| editor.backend.session.has_bookmark(slot as usize)) }
}

/// # Safety
/// See the module rules.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lantern_editor_align_camera_to_selection(handle: *mut LanternEditor) -> bool {
    // SAFETY: see the module rules.
    unsafe { with_editor(handle, |editor| editor.backend.session.align_camera_to_selection()) }
}

/// # Safety
/// See the module rules.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lantern_editor_align_selection_to_camera(handle: *mut LanternEditor) -> bool {
    // SAFETY: see the module rules.
    unsafe { with_editor(handle, |editor| editor.backend.session.align_selection_to_camera()) }
}

// ----- Outline, visibility and selection ----------------------------------------------------------

/// # Safety
/// See the module rules.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lantern_editor_entity_count(handle: *mut LanternEditor) -> u32 {
    // SAFETY: see the module rules.
    unsafe { with_editor(handle, |editor| editor.backend.session.world().entities.len() as u32) }
}

/// The parent of the entity on an outline row, 0 at the root.
///
/// # Safety
/// See the module rules.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lantern_editor_outline_parent(handle: *mut LanternEditor, row: u32) -> i64 {
    // SAFETY: see the module rules.
    unsafe {
        with_editor(handle, |editor| crate::ffi::outline_row(editor, row).and_then(|row| row.parent).map_or(0, id))
    }
}

/// # Safety
/// See the module rules.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lantern_editor_outline_has_children(handle: *mut LanternEditor, row: u32) -> bool {
    // SAFETY: see the module rules.
    unsafe { with_editor(handle, |editor| crate::ffi::outline_row(editor, row).is_some_and(|row| row.has_children)) }
}

/// # Safety
/// See the module rules.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lantern_editor_is_hidden(handle: *mut LanternEditor, entity: i64) -> bool {
    // SAFETY: see the module rules.
    unsafe { with_editor(handle, |editor| editor.backend.session.is_hidden(key(entity))) }
}

/// # Safety
/// See the module rules.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lantern_editor_set_hidden(handle: *mut LanternEditor, entity: i64, hidden: bool) {
    // SAFETY: see the module rules.
    unsafe { with_editor(handle, |editor| editor.backend.session.set_hidden(key(entity), hidden)) }
}

/// # Safety
/// See the module rules.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lantern_editor_is_locked(handle: *mut LanternEditor, entity: i64) -> bool {
    // SAFETY: see the module rules.
    unsafe { with_editor(handle, |editor| editor.backend.session.is_locked(key(entity))) }
}

/// # Safety
/// See the module rules.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lantern_editor_set_locked(handle: *mut LanternEditor, entity: i64, locked: bool) {
    // SAFETY: see the module rules.
    unsafe { with_editor(handle, |editor| editor.backend.session.set_locked(key(entity), locked)) }
}

/// # Safety
/// See the module rules.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lantern_editor_hide_selection(handle: *mut LanternEditor) {
    // SAFETY: see the module rules.
    unsafe { with_editor(handle, |editor| editor.backend.session.hide_selection()) }
}

/// # Safety
/// See the module rules.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lantern_editor_unhide_all(handle: *mut LanternEditor) {
    // SAFETY: see the module rules.
    unsafe { with_editor(handle, |editor| editor.backend.session.unhide_all()) }
}

/// # Safety
/// See the module rules.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lantern_editor_lock_selection(handle: *mut LanternEditor, locked: bool) {
    // SAFETY: see the module rules.
    unsafe { with_editor(handle, |editor| editor.backend.session.lock_selection(locked)) }
}

/// # Safety
/// See the module rules.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lantern_editor_select_all(handle: *mut LanternEditor) {
    // SAFETY: see the module rules.
    unsafe { with_editor(handle, |editor| editor.backend.session.select_all()) }
}

/// # Safety
/// See the module rules.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lantern_editor_invert_selection(handle: *mut LanternEditor) {
    // SAFETY: see the module rules.
    unsafe { with_editor(handle, |editor| editor.backend.session.invert_selection()) }
}

/// Selects every entity of a kind.
///
/// # Safety
/// See the module rules.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lantern_editor_select_kind(handle: *mut LanternEditor, kind: *const c_char) {
    // SAFETY: see the module rules.
    let kind = unsafe { read_string(kind) }.to_string();
    // SAFETY: see the module rules.
    unsafe { with_editor(handle, |editor| editor.backend.session.select_kind(&kind)) }
}

/// # Safety
/// See the module rules.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lantern_editor_select_children(handle: *mut LanternEditor) {
    // SAFETY: see the module rules.
    unsafe { with_editor(handle, |editor| editor.backend.session.select_children()) }
}

/// Parents `child` under `parent`, or at the root when `parent` is 0. Refuses cycles.
///
/// # Safety
/// See the module rules.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lantern_editor_set_parent(handle: *mut LanternEditor, child: i64, parent: i64) -> bool {
    let parent = (parent > 0).then(|| key(parent));
    // SAFETY: see the module rules.
    unsafe {
        with_editor(handle, |editor| {
            editor.backend.session.commit_field();
            editor.backend.session.set_parent(key(child), parent)
        })
    }
}

// ----- Inspector and editing ----------------------------------------------------------------------

/// Sets a field on every selected entity in one undo step (the name only on the primary).
///
/// # Safety
/// See the module rules.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lantern_editor_set_selection_field(
    handle: *mut LanternEditor,
    field: *const c_char,
    text: *const c_char,
) -> bool {
    // SAFETY: see the module rules.
    let (field, text) = unsafe { (read_string(field).to_string(), read_string(text).to_string()) };
    // SAFETY: see the module rules.
    unsafe { with_editor(handle, |editor| editor.backend.session.set_field_on_selection(&field, &text)) }
}

/// # Safety
/// See the module rules.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lantern_editor_rename_property(
    handle: *mut LanternEditor,
    old_name: *const c_char,
    new_name: *const c_char,
) -> bool {
    // SAFETY: see the module rules.
    let (old_name, new_name) = unsafe { (read_string(old_name).to_string(), read_string(new_name).to_string()) };
    // SAFETY: see the module rules.
    unsafe { with_editor(handle, |editor| editor.backend.session.rename_property(&old_name, &new_name)) }
}

/// Adds a template's entity where the pointer last was in the viewport (a drop from the
/// asset browser), and returns its id.
///
/// # Safety
/// See the module rules.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lantern_editor_add_entity_at_cursor(
    handle: *mut LanternEditor,
    template: *const c_char,
) -> i64 {
    // SAFETY: see the module rules.
    let template = unsafe { read_string(template) }.to_string();
    // SAFETY: see the module rules.
    unsafe {
        with_editor(handle, |editor| {
            editor.backend.session.commit_field();
            editor.backend.session.add_entity(&template, true).map_or(0, id)
        })
    }
}

/// # Safety
/// See the module rules.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lantern_editor_duplicate_along(handle: *mut LanternEditor, axis: *const c_char) -> bool {
    let Some(axis) = axis_from(axis) else { return false };
    // SAFETY: see the module rules.
    unsafe { with_editor(handle, |editor| editor.backend.session.duplicate_selection_along(axis)) }
}

/// # Safety
/// See the module rules.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lantern_editor_mirror_selection(handle: *mut LanternEditor, axis: *const c_char) -> bool {
    let Some(axis) = axis_from(axis) else { return false };
    // SAFETY: see the module rules.
    unsafe { with_editor(handle, |editor| editor.backend.session.mirror_selection(axis)) }
}

/// # Safety
/// See the module rules.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lantern_editor_snap_selection_to_ground(handle: *mut LanternEditor) -> bool {
    // SAFETY: see the module rules.
    unsafe { with_editor(handle, |editor| editor.backend.session.snap_selection_to_ground()) }
}

/// # Safety
/// See the module rules.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lantern_editor_group_selection(handle: *mut LanternEditor) -> bool {
    // SAFETY: see the module rules.
    unsafe { with_editor(handle, |editor| editor.backend.session.group_selection()) }
}

/// # Safety
/// See the module rules.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lantern_editor_align_selection(handle: *mut LanternEditor, axis: *const c_char) -> bool {
    let Some(axis) = axis_from(axis) else { return false };
    // SAFETY: see the module rules.
    unsafe { with_editor(handle, |editor| editor.backend.session.align_selection(axis)) }
}

/// # Safety
/// See the module rules.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lantern_editor_distribute_selection(handle: *mut LanternEditor, axis: *const c_char) -> bool {
    let Some(axis) = axis_from(axis) else { return false };
    // SAFETY: see the module rules.
    unsafe { with_editor(handle, |editor| editor.backend.session.distribute_selection(axis)) }
}

/// # Safety
/// See the module rules.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lantern_editor_revert_to_template(handle: *mut LanternEditor) -> bool {
    // SAFETY: see the module rules.
    unsafe { with_editor(handle, |editor| editor.backend.session.revert_selection_to_template()) }
}

/// # Safety
/// See the module rules.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lantern_editor_apply_to_template(handle: *mut LanternEditor) -> bool {
    // SAFETY: see the module rules.
    unsafe { with_editor(handle, |editor| editor.backend.session.apply_primary_to_template()) }
}

// ----- History and log -----------------------------------------------------------------------------

/// # Safety
/// See the module rules.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lantern_editor_history_count(handle: *mut LanternEditor) -> u32 {
    // SAFETY: see the module rules.
    unsafe { with_editor(handle, |editor| editor.backend.session.history_labels().len() as u32) }
}

/// # Safety
/// See the module rules.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lantern_editor_history_label(handle: *mut LanternEditor, index: u32) -> *const c_char {
    // SAFETY: see the module rules.
    unsafe {
        string_from(handle, |editor| {
            editor.backend.session.history_labels().get(index as usize).cloned().unwrap_or_default()
        })
    }
}

/// How many history steps are applied; steps from here on are undone.
///
/// # Safety
/// See the module rules.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lantern_editor_history_position(handle: *mut LanternEditor) -> u32 {
    // SAFETY: see the module rules.
    unsafe { with_editor(handle, |editor| editor.backend.session.history_position() as u32) }
}

/// # Safety
/// See the module rules.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lantern_editor_jump_history(handle: *mut LanternEditor, position: u32) -> bool {
    // SAFETY: see the module rules.
    unsafe {
        with_editor(handle, |editor| {
            editor.backend.session.commit_field();
            editor.backend.session.jump_history(position as usize)
        })
    }
}

/// # Safety
/// See the module rules.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lantern_editor_log_count(handle: *mut LanternEditor) -> u32 {
    // SAFETY: see the module rules.
    unsafe { with_editor(handle, |editor| editor.backend.session.log().len() as u32) }
}

/// # Safety
/// See the module rules.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lantern_editor_log_text(handle: *mut LanternEditor, index: u32) -> *const c_char {
    // SAFETY: see the module rules.
    unsafe {
        string_from(handle, |editor| {
            editor.backend.session.log().get(index as usize).map(|entry| entry.text.clone()).unwrap_or_default()
        })
    }
}

/// `info`, `warning` or `error`.
///
/// # Safety
/// See the module rules.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lantern_editor_log_level(handle: *mut LanternEditor, index: u32) -> *const c_char {
    // SAFETY: see the module rules.
    unsafe {
        string_from(handle, |editor| {
            editor
                .backend
                .session
                .log()
                .get(index as usize)
                .map(|entry| entry.level.name().to_string())
                .unwrap_or_default()
        })
    }
}

/// # Safety
/// See the module rules.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lantern_editor_clear_log(handle: *mut LanternEditor) {
    // SAFETY: see the module rules.
    unsafe { with_editor(handle, |editor| editor.backend.session.clear_log()) }
}

// ----- Play in editor, statistics and previews -------------------------------------------------------

/// Saves the level to a scratch file and launches the game on it.
///
/// # Safety
/// See the module rules.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lantern_editor_play(handle: *mut LanternEditor) -> bool {
    // SAFETY: see the module rules.
    unsafe { with_editor(handle, |editor| editor.backend.start_play()) }
}

/// # Safety
/// See the module rules.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lantern_editor_stop_play(handle: *mut LanternEditor) -> bool {
    // SAFETY: see the module rules.
    unsafe { with_editor(handle, |editor| editor.backend.stop_play()) }
}

/// # Safety
/// See the module rules.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lantern_editor_is_playing(handle: *mut LanternEditor) -> bool {
    // SAFETY: see the module rules.
    unsafe { with_editor(handle, |editor| editor.backend.is_playing()) }
}

/// A statistic as text: `render_ms`, `fps`, `draws`, `culled`, `triangles`, `lines`,
/// `entities`, `selected`, `play_seconds`.
///
/// # Safety
/// See the module rules.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lantern_editor_stat(handle: *mut LanternEditor, name: *const c_char) -> *const c_char {
    // SAFETY: see the module rules.
    let name = unsafe { read_string(name) }.to_string();
    // SAFETY: see the module rules.
    unsafe {
        string_from(handle, |editor| match name.as_str() {
            "entities" => editor.backend.session.world().entities.len().to_string(),
            "selected" => editor.backend.session.selection().len().to_string(),
            "play_seconds" => format!("{:.0}", editor.backend.play_seconds()),
            other => editor.backend.stats.get(other).unwrap_or_default(),
        })
    }
}

/// Draws a template on its own, turned by `spin` radians, for the preview viewport.
///
/// # Safety
/// See the module rules.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lantern_editor_render_preview(
    handle: *mut LanternEditor,
    template: *const c_char,
    width: u32,
    height: u32,
    spin: c_float,
) -> bool {
    // SAFETY: see the module rules.
    let template = unsafe { read_string(template) }.to_string();
    // SAFETY: see the module rules.
    unsafe {
        with_editor(handle, |editor| match editor.backend.render_template_preview(&template, width, height, spin) {
            Ok(()) => true,
            Err(error) => {
                if editor.api_status.as_deref() != Some(error.as_str()) {
                    editor.api_status = Some(error);
                }
                false
            }
        })
    }
}

struct Pixels(*const u8, u32, u32);

impl Default for Pixels {
    fn default() -> Self {
        Pixels(std::ptr::null(), 0, 0)
    }
}

/// The last preview: RGBA8, top row first, valid until the next preview or thumbnail call.
///
/// # Safety
/// See the module rules; `width` and `height` are null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lantern_editor_preview_pixels(
    handle: *mut LanternEditor,
    width: *mut u32,
    height: *mut u32,
) -> *const u8 {
    // SAFETY: see the module rules.
    let pixels = unsafe {
        with_editor(handle, |editor| {
            let (bytes, pixel_width, pixel_height) = editor.backend.preview_pixels();
            if bytes.is_empty() { Pixels::default() } else { Pixels(bytes.as_ptr(), pixel_width, pixel_height) }
        })
    };
    // SAFETY: the caller passes null or writable pointers.
    unsafe {
        if !width.is_null() {
            *width = pixels.1;
        }
        if !height.is_null() {
            *height = pixels.2;
        }
    }
    pixels.0
}

/// A square thumbnail of a template, `size` pixels a side, drawn once and kept: RGBA8, top row
/// first, valid until the palette changes. Null when it couldn't be drawn.
///
/// # Safety
/// See the module rules; `actual_size` is null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lantern_editor_thumbnail(
    handle: *mut LanternEditor,
    template: *const c_char,
    size: u32,
    actual_size: *mut u32,
) -> *const u8 {
    // SAFETY: see the module rules.
    let template = unsafe { read_string(template) }.to_string();
    // SAFETY: see the module rules.
    let pixels = unsafe {
        with_editor(handle, |editor| match editor.backend.thumbnail(&template, size) {
            Ok((bytes, side)) => Pixels(bytes.as_ptr(), side, side),
            Err(error) => {
                if editor.api_status.as_deref() != Some(error.as_str()) {
                    editor.api_status = Some(error);
                }
                Pixels::default()
            }
        })
    };
    // SAFETY: the caller passes null or a writable pointer.
    unsafe {
        if !actual_size.is_null() {
            *actual_size = pixels.1;
        }
    }
    pixels.0
}

/// The directory the editor's assets came from.
///
/// # Safety
/// See the module rules.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lantern_editor_assets_directory(handle: *mut LanternEditor) -> *const c_char {
    // SAFETY: see the module rules.
    unsafe { string_from(handle, |editor| Path::display(editor.backend.assets_directory()).to_string()) }
}
