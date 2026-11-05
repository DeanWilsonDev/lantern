/*
 * lantern_editor.h: the C API of Lantern's world editor backend (engine/editor-backend).
 *
 * The world editor's front end (apps/world-editor: Penumbra, Chaos, Cosmos and Nyx) drives the
 * backend through these functions only. The backend owns the world document, undo history,
 * selection, gizmos and the camera, and draws the 3D viewport off-screen into RGBA pixels.
 *
 * Rules:
 * - Use one handle from one thread (the UI thread). A null handle is ignored.
 * - Strings passed in are NUL-terminated UTF-8, borrowed for the call.
 * - Strings returned stay valid until the next call on the same handle that returns a string.
 *   Copy them straight away.
 * - Entity ids are the world file's ids; 0 means none.
 * - Errors show in lantern_editor_status.
 */
#ifndef LANTERN_EDITOR_H
#define LANTERN_EDITOR_H

#include <stdbool.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct LanternEditor LanternEditor;

#define LANTERN_MODIFIER_SHIFT 1u
#define LANTERN_MODIFIER_COMMAND 2u
#define LANTERN_MODIFIER_ALT 4u

#define LANTERN_POINTER_LEFT 0u
#define LANTERN_POINTER_MIDDLE 1u
#define LANTERN_POINTER_RIGHT 2u

#define LANTERN_FIELD_KEY 0u
#define LANTERN_FIELD_LABEL 1u
#define LANTERN_FIELD_GROUP 2u
#define LANTERN_FIELD_VALUE 3u
#define LANTERN_FIELD_KIND 4u
#define LANTERN_FIELD_MIXED 5u

#define LANTERN_TEMPLATE_ID 0u
#define LANTERN_TEMPLATE_LABEL 1u
#define LANTERN_TEMPLATE_KIND 2u

/* Lifetime. world_path may be null to open <assets>/worlds/course.world.json. */
LanternEditor* lantern_editor_create(const char* assets_directory, const char* world_path);
void lantern_editor_destroy(LanternEditor* editor);

/* Document. The revision changes whenever anything the panels show changes. */
uint64_t lantern_editor_revision(LanternEditor* editor);
const char* lantern_editor_status(LanternEditor* editor);
const char* lantern_editor_title(LanternEditor* editor);
const char* lantern_editor_document_path(LanternEditor* editor);
bool lantern_editor_is_dirty(LanternEditor* editor);
bool lantern_editor_open(LanternEditor* editor, const char* path);
bool lantern_editor_save(LanternEditor* editor);
bool lantern_editor_save_as(LanternEditor* editor, const char* path);
void lantern_editor_new_world(LanternEditor* editor, const char* name);

/* Viewport. Pixels are RGBA8 (sRGB), top row first, valid until the next render. */
bool lantern_editor_render(LanternEditor* editor, uint32_t width, uint32_t height);
const uint8_t* lantern_editor_pixels(LanternEditor* editor, uint32_t* width, uint32_t* height);
void lantern_editor_pointer_move(LanternEditor* editor, float x, float y);
void lantern_editor_pointer_button(LanternEditor* editor, uint32_t button, bool down, uint32_t modifiers);
void lantern_editor_wheel(LanternEditor* editor, float notches);
/* Keys are named as the web's KeyboardEvent.key names them: "w", "Escape", "Delete", "F2", "1". */
bool lantern_editor_key(LanternEditor* editor, const char* key_name, bool down, uint32_t modifiers);
/* Forgets every held key; call it when the pointer leaves the viewport. */
void lantern_editor_release_keys(LanternEditor* editor);
void lantern_editor_update(LanternEditor* editor, float seconds);
void lantern_editor_frame_selection(LanternEditor* editor);
void lantern_editor_set_tool(LanternEditor* editor, const char* name);
const char* lantern_editor_tool(LanternEditor* editor);
void lantern_editor_set_snap(LanternEditor* editor, bool enabled);
bool lantern_editor_snap(LanternEditor* editor);

/* Outline and selection. */
uint32_t lantern_editor_outline_count(LanternEditor* editor);
int64_t lantern_editor_outline_id(LanternEditor* editor, uint32_t row);
uint32_t lantern_editor_outline_depth(LanternEditor* editor, uint32_t row);
const char* lantern_editor_entity_name(LanternEditor* editor, int64_t entity);
const char* lantern_editor_entity_kind(LanternEditor* editor, int64_t entity);
bool lantern_editor_is_selected(LanternEditor* editor, int64_t entity);
void lantern_editor_select(LanternEditor* editor, int64_t entity, bool additive);
void lantern_editor_clear_selection(LanternEditor* editor);
int64_t lantern_editor_primary_selection(LanternEditor* editor);
uint32_t lantern_editor_selection_count(LanternEditor* editor);

/* Inspector: fields of the primary selection. */
uint32_t lantern_editor_field_count(LanternEditor* editor);
const char* lantern_editor_field(LanternEditor* editor, uint32_t index, uint32_t part);
uint32_t lantern_editor_field_choice_count(LanternEditor* editor, uint32_t index);
const char* lantern_editor_field_choice(LanternEditor* editor, uint32_t index, uint32_t choice);
bool lantern_editor_set_field(LanternEditor* editor, int64_t entity, const char* field, const char* text);
void lantern_editor_commit_field(LanternEditor* editor);

/* Editing. */
uint32_t lantern_editor_template_count(LanternEditor* editor);
const char* lantern_editor_template(LanternEditor* editor, uint32_t index, uint32_t part);
int64_t lantern_editor_add_entity(LanternEditor* editor, const char* template_id);
bool lantern_editor_delete_selection(LanternEditor* editor);
bool lantern_editor_duplicate_selection(LanternEditor* editor);
bool lantern_editor_undo(LanternEditor* editor);
bool lantern_editor_redo(LanternEditor* editor);
bool lantern_editor_can_undo(LanternEditor* editor);
bool lantern_editor_can_redo(LanternEditor* editor);

/* Recent files and preferences (a flat string map saved in the user's config directory). */
uint32_t lantern_editor_recent_file_count(LanternEditor* editor);
const char* lantern_editor_recent_file(LanternEditor* editor, uint32_t index);
const char* lantern_editor_preference(LanternEditor* editor, const char* name);
bool lantern_editor_set_preference(LanternEditor* editor, const char* name, const char* value);
bool lantern_editor_save_preferences(LanternEditor* editor);
const char* lantern_editor_assets_directory(LanternEditor* editor);

/* Gizmo space ("world"/"local"), pivot ("primary"/"center"), snap steps ("translate", "rotate",
 * "scale") and view options ("grid", "grid_spacing", "colliders", "markers", "wireframe",
 * "level_bounds", "hidden", "stats"; values "true"/"false" or a number). */
void lantern_editor_set_space(LanternEditor* editor, const char* name);
const char* lantern_editor_space(LanternEditor* editor);
void lantern_editor_set_pivot_mode(LanternEditor* editor, const char* name);
const char* lantern_editor_pivot_mode(LanternEditor* editor);
bool lantern_editor_set_snap_step(LanternEditor* editor, const char* name, float value);
float lantern_editor_snap_step(LanternEditor* editor, const char* name);
bool lantern_editor_set_view_option(LanternEditor* editor, const char* name, const char* value);
const char* lantern_editor_view_option(LanternEditor* editor, const char* name);
const char* lantern_editor_hovered_handle(LanternEditor* editor);

/* Camera: presets ("perspective", "front", "back", "left", "right", "top", "bottom"),
 * orthographic projection, bookmarks in slots 0..8, and alignment with the selection. */
bool lantern_editor_camera_preset(LanternEditor* editor, const char* name);
void lantern_editor_set_orthographic(LanternEditor* editor, bool orthographic);
bool lantern_editor_is_orthographic(LanternEditor* editor);
const char* lantern_editor_camera_description(LanternEditor* editor);
void lantern_editor_frame_all(LanternEditor* editor);
bool lantern_editor_set_bookmark(LanternEditor* editor, uint32_t slot);
bool lantern_editor_go_to_bookmark(LanternEditor* editor, uint32_t slot);
bool lantern_editor_has_bookmark(LanternEditor* editor, uint32_t slot);
bool lantern_editor_align_camera_to_selection(LanternEditor* editor);
bool lantern_editor_align_selection_to_camera(LanternEditor* editor);

/* Outline details, visibility, locking and more ways to select. */
uint32_t lantern_editor_entity_count(LanternEditor* editor);
int64_t lantern_editor_outline_parent(LanternEditor* editor, uint32_t row);
bool lantern_editor_outline_has_children(LanternEditor* editor, uint32_t row);
bool lantern_editor_is_hidden(LanternEditor* editor, int64_t entity);
void lantern_editor_set_hidden(LanternEditor* editor, int64_t entity, bool hidden);
bool lantern_editor_is_locked(LanternEditor* editor, int64_t entity);
void lantern_editor_set_locked(LanternEditor* editor, int64_t entity, bool locked);
void lantern_editor_hide_selection(LanternEditor* editor);
void lantern_editor_unhide_all(LanternEditor* editor);
void lantern_editor_lock_selection(LanternEditor* editor, bool locked);
void lantern_editor_select_all(LanternEditor* editor);
void lantern_editor_invert_selection(LanternEditor* editor);
void lantern_editor_select_kind(LanternEditor* editor, const char* kind);
void lantern_editor_select_children(LanternEditor* editor);
bool lantern_editor_set_parent(LanternEditor* editor, int64_t child, int64_t parent);
bool lantern_editor_move_before(LanternEditor* editor, int64_t child, int64_t sibling);

/* Editing the selection as a whole. Axes are "x", "y" or "z". */
bool lantern_editor_set_selection_field(LanternEditor* editor, const char* field, const char* text);
bool lantern_editor_rename_property(LanternEditor* editor, const char* old_name, const char* new_name);
int64_t lantern_editor_add_entity_at_cursor(LanternEditor* editor, const char* template_id);
bool lantern_editor_duplicate_along(LanternEditor* editor, const char* axis);
bool lantern_editor_mirror_selection(LanternEditor* editor, const char* axis);
bool lantern_editor_snap_selection_to_ground(LanternEditor* editor);
bool lantern_editor_group_selection(LanternEditor* editor);
bool lantern_editor_align_selection(LanternEditor* editor, const char* axis);
bool lantern_editor_distribute_selection(LanternEditor* editor, const char* axis);
bool lantern_editor_revert_to_template(LanternEditor* editor);
bool lantern_editor_apply_to_template(LanternEditor* editor);

/* Undo history as a list, and the console log. Levels are "info", "warning", "error". */
uint32_t lantern_editor_history_count(LanternEditor* editor);
const char* lantern_editor_history_label(LanternEditor* editor, uint32_t index);
uint32_t lantern_editor_history_position(LanternEditor* editor);
bool lantern_editor_jump_history(LanternEditor* editor, uint32_t position);
uint32_t lantern_editor_log_count(LanternEditor* editor);
const char* lantern_editor_log_text(LanternEditor* editor, uint32_t index);
const char* lantern_editor_log_level(LanternEditor* editor, uint32_t index);
void lantern_editor_clear_log(LanternEditor* editor);

/* Play in editor, statistics ("render_ms", "fps", "draws", "culled", "triangles", "lines",
 * "entities", "selected", "play_seconds") and template previews. Preview and thumbnail pixels
 * are RGBA8, top row first, valid until the next preview or thumbnail call. */
bool lantern_editor_play(LanternEditor* editor);
bool lantern_editor_stop_play(LanternEditor* editor);
bool lantern_editor_is_playing(LanternEditor* editor);
const char* lantern_editor_stat(LanternEditor* editor, const char* name);
bool lantern_editor_render_preview(LanternEditor* editor, const char* template_id, uint32_t width, uint32_t height,
                                   float spin_radians);
const uint8_t* lantern_editor_preview_pixels(LanternEditor* editor, uint32_t* width, uint32_t* height);
const uint8_t* lantern_editor_thumbnail(LanternEditor* editor, const char* template_id, uint32_t size,
                                        uint32_t* actual_size);

#ifdef __cplusplus
}
#endif

#endif
