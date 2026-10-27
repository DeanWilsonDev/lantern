/*
 * lantern_editor.h: the C API of Lantern's world editor backend (crates/editor-backend).
 *
 * The world editor's front end (apps/world-editor: Penumbra, Chaos, Cosmos and Nyx) drives the
 * Rust backend through these functions only. The backend owns the world document, undo history,
 * selection, gizmos and the camera, and draws the 3D viewport off-screen into RGBA pixels.
 *
 * Rules:
 * - Use one handle from one thread (the UI thread). A null handle is ignored.
 * - Strings passed in are NUL-terminated UTF-8, borrowed for the call.
 * - Strings returned stay valid until the next call on the same handle that returns a string.
 *   Copy them straight away.
 * - Entity ids are the world file's ids; 0 means none.
 * - No panic crosses the boundary: errors show in lantern_editor_status.
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

#define LANTERN_KEY_BACKSPACE 8u
#define LANTERN_KEY_ESCAPE 27u
#define LANTERN_KEY_DELETE 127u

#define LANTERN_POINTER_LEFT 0u
#define LANTERN_POINTER_MIDDLE 1u
#define LANTERN_POINTER_RIGHT 2u

#define LANTERN_FIELD_KEY 0u
#define LANTERN_FIELD_LABEL 1u
#define LANTERN_FIELD_GROUP 2u
#define LANTERN_FIELD_VALUE 3u
#define LANTERN_FIELD_KIND 4u

#define LANTERN_TEMPLATE_ID 0u
#define LANTERN_TEMPLATE_LABEL 1u

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
bool lantern_editor_key(LanternEditor* editor, uint32_t code, bool down, uint32_t modifiers);
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

#ifdef __cplusplus
}
#endif

#endif
