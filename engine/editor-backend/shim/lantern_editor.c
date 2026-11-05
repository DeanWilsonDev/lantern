#if !defined(_WIN32) && !defined(_POSIX_C_SOURCE)
#define _POSIX_C_SOURCE 200809L
#endif

#include "lantern_editor.h"

#include <stdlib.h>
#include <string.h>

#include "liblantern_editor_nox.h"

typedef struct PixelBuffer {
    uint8_t* bytes;
    size_t capacity;
} PixelBuffer;

struct LanternEditor {
    struct nox_lantern__editorapi__Editor* backend;
    char* returnedString;
    PixelBuffer viewportPixels;
    PixelBuffer previewPixels;
    PixelBuffer thumbnailPixels;
};

static struct nox_string NoxText(const char* text) {
    struct nox_string converted;
    converted.data = text ? text : "";
    converted.length = text ? (uint64_t)strlen(text) : 0;
    return converted;
}

static const char* GiveString(LanternEditor* editor, struct nox_string owned) {
    char* copy = (char*)malloc((size_t)owned.length + 1);
    if (!copy) {
        free((void*)owned.data);
        return "";
    }
    for (uint64_t index = 0; index < owned.length; ++index) {
        char character = owned.data[index];
        copy[index] = character == '\0' ? ' ' : character;
    }
    copy[owned.length] = '\0';
    free((void*)owned.data);
    free(editor->returnedString);
    editor->returnedString = copy;
    return copy;
}

static uint8_t* ReserveBytes(PixelBuffer* buffer, size_t byteCount) {
    if (byteCount > buffer->capacity) {
        uint8_t* grown = (uint8_t*)realloc(buffer->bytes, byteCount);
        if (!grown) return NULL;
        buffer->bytes = grown;
        buffer->capacity = byteCount;
    }
    return buffer->bytes;
}

static struct nox_interop_view_u8 ViewOf(uint8_t* bytes, size_t byteCount) {
    struct nox_interop_view_u8 view;
    view.data = bytes;
    view.length = (uint64_t)byteCount;
    return view;
}

static void WriteSize(uint32_t* target, uint32_t value) {
    if (target) *target = value;
}

static void UseBuiltShaderDirectory(void) {
#ifdef LANTERN_EDITOR_SHADER_DIRECTORY
    const char* current = getenv("LANTERN_SHADER_DIRECTORY");
    if (current && current[0]) return;
#ifdef _WIN32
    _putenv_s("LANTERN_SHADER_DIRECTORY", LANTERN_EDITOR_SHADER_DIRECTORY);
#else
    setenv("LANTERN_SHADER_DIRECTORY", LANTERN_EDITOR_SHADER_DIRECTORY, 0);
#endif
#endif
}

LanternEditor* lantern_editor_create(const char* assets_directory, const char* world_path) {
    UseBuiltShaderDirectory();
    LanternEditor* editor = (LanternEditor*)calloc(1, sizeof(LanternEditor));
    if (!editor) return NULL;
    editor->backend = nox_lantern__editorapi__Create(NoxText(assets_directory), NoxText(world_path));
    if (!editor->backend) {
        free(editor);
        return NULL;
    }
    return editor;
}

void lantern_editor_destroy(LanternEditor* editor) {
    if (!editor) return;
    nox_lantern__editorapi__Destroy(editor->backend);
    free(editor->returnedString);
    free(editor->viewportPixels.bytes);
    free(editor->previewPixels.bytes);
    free(editor->thumbnailPixels.bytes);
    free(editor);
}

const uint8_t* lantern_editor_pixels(LanternEditor* editor, uint32_t* width, uint32_t* height) {
    WriteSize(width, 0);
    WriteSize(height, 0);
    if (!editor) return NULL;
    uint32_t pixelWidth = nox_lantern__editorapi__PixelWidth(editor->backend);
    uint32_t pixelHeight = nox_lantern__editorapi__PixelHeight(editor->backend);
    size_t byteCount = (size_t)pixelWidth * pixelHeight * 4;
    if (byteCount == 0) return NULL;
    uint8_t* bytes = ReserveBytes(&editor->viewportPixels, byteCount);
    if (!bytes || !nox_lantern__editorapi__CopyPixels(editor->backend, ViewOf(bytes, byteCount))) return NULL;
    WriteSize(width, pixelWidth);
    WriteSize(height, pixelHeight);
    return bytes;
}

const uint8_t* lantern_editor_preview_pixels(LanternEditor* editor, uint32_t* width, uint32_t* height) {
    WriteSize(width, 0);
    WriteSize(height, 0);
    if (!editor) return NULL;
    uint32_t pixelWidth = nox_lantern__editorapi__PreviewWidth(editor->backend);
    uint32_t pixelHeight = nox_lantern__editorapi__PreviewHeight(editor->backend);
    size_t byteCount = (size_t)pixelWidth * pixelHeight * 4;
    if (byteCount == 0) return NULL;
    uint8_t* bytes = ReserveBytes(&editor->previewPixels, byteCount);
    if (!bytes || !nox_lantern__editorapi__CopyPreviewPixels(editor->backend, ViewOf(bytes, byteCount))) return NULL;
    WriteSize(width, pixelWidth);
    WriteSize(height, pixelHeight);
    return bytes;
}

const uint8_t* lantern_editor_thumbnail(LanternEditor* editor, const char* template_id, uint32_t size,
                                        uint32_t* actual_size) {
    WriteSize(actual_size, 0);
    if (!editor) return NULL;
    uint32_t side = nox_lantern__editorapi__Thumbnail(editor->backend, NoxText(template_id), size);
    size_t byteCount = (size_t)side * side * 4;
    if (byteCount == 0) return NULL;
    uint8_t* bytes = ReserveBytes(&editor->thumbnailPixels, byteCount);
    if (!bytes ||
        !nox_lantern__editorapi__CopyThumbnailPixels(editor->backend, NoxText(template_id), ViewOf(bytes, byteCount))) {
        return NULL;
    }
    WriteSize(actual_size, side);
    return bytes;
}

uint64_t lantern_editor_revision(LanternEditor* editor) {
    if (!editor) return 0;
    return nox_lantern__editorapi__Revision(editor->backend);
}

const char* lantern_editor_status(LanternEditor* editor) {
    if (!editor) return "";
    return GiveString(editor, nox_lantern__editorapi__Status(editor->backend));
}

const char* lantern_editor_title(LanternEditor* editor) {
    if (!editor) return "";
    return GiveString(editor, nox_lantern__editorapi__Title(editor->backend));
}

const char* lantern_editor_document_path(LanternEditor* editor) {
    if (!editor) return "";
    return GiveString(editor, nox_lantern__editorapi__DocumentPath(editor->backend));
}

bool lantern_editor_is_dirty(LanternEditor* editor) {
    if (!editor) return false;
    return nox_lantern__editorapi__IsDirty(editor->backend);
}

bool lantern_editor_open(LanternEditor* editor, const char* path) {
    if (!editor) return false;
    return nox_lantern__editorapi__Open(editor->backend, NoxText(path));
}

bool lantern_editor_save(LanternEditor* editor) {
    if (!editor) return false;
    return nox_lantern__editorapi__Save(editor->backend);
}

bool lantern_editor_save_as(LanternEditor* editor, const char* path) {
    if (!editor) return false;
    return nox_lantern__editorapi__SaveAs(editor->backend, NoxText(path));
}

void lantern_editor_new_world(LanternEditor* editor, const char* name) {
    if (!editor) return;
    nox_lantern__editorapi__NewWorld(editor->backend, NoxText(name));
}

bool lantern_editor_render(LanternEditor* editor, uint32_t width, uint32_t height) {
    if (!editor) return false;
    return nox_lantern__editorapi__Render(editor->backend, width, height);
}

void lantern_editor_pointer_move(LanternEditor* editor, float x, float y) {
    if (!editor) return;
    nox_lantern__editorapi__PointerMove(editor->backend, x, y);
}

void lantern_editor_pointer_button(LanternEditor* editor, uint32_t button, bool down, uint32_t modifiers) {
    if (!editor) return;
    nox_lantern__editorapi__PointerButton(editor->backend, button, down, modifiers);
}

void lantern_editor_wheel(LanternEditor* editor, float notches) {
    if (!editor) return;
    nox_lantern__editorapi__Wheel(editor->backend, notches);
}

bool lantern_editor_key(LanternEditor* editor, const char* key_name, bool down, uint32_t modifiers) {
    if (!editor) return false;
    return nox_lantern__editorapi__Key(editor->backend, NoxText(key_name), down, modifiers);
}

void lantern_editor_release_keys(LanternEditor* editor) {
    if (!editor) return;
    nox_lantern__editorapi__ReleaseKeys(editor->backend);
}

void lantern_editor_update(LanternEditor* editor, float seconds) {
    if (!editor) return;
    nox_lantern__editorapi__Update(editor->backend, seconds);
}

void lantern_editor_frame_selection(LanternEditor* editor) {
    if (!editor) return;
    nox_lantern__editorapi__FrameSelection(editor->backend);
}

void lantern_editor_set_tool(LanternEditor* editor, const char* name) {
    if (!editor) return;
    nox_lantern__editorapi__SetTool(editor->backend, NoxText(name));
}

const char* lantern_editor_tool(LanternEditor* editor) {
    if (!editor) return "";
    return GiveString(editor, nox_lantern__editorapi__Tool(editor->backend));
}

void lantern_editor_set_snap(LanternEditor* editor, bool enabled) {
    if (!editor) return;
    nox_lantern__editorapi__SetSnap(editor->backend, enabled);
}

bool lantern_editor_snap(LanternEditor* editor) {
    if (!editor) return false;
    return nox_lantern__editorapi__Snap(editor->backend);
}

uint32_t lantern_editor_outline_count(LanternEditor* editor) {
    if (!editor) return 0;
    return nox_lantern__editorapi__OutlineCount(editor->backend);
}

int64_t lantern_editor_outline_id(LanternEditor* editor, uint32_t row) {
    if (!editor) return 0;
    return nox_lantern__editorapi__OutlineId(editor->backend, row);
}

uint32_t lantern_editor_outline_depth(LanternEditor* editor, uint32_t row) {
    if (!editor) return 0;
    return nox_lantern__editorapi__OutlineDepth(editor->backend, row);
}

const char* lantern_editor_entity_name(LanternEditor* editor, int64_t entity) {
    if (!editor) return "";
    return GiveString(editor, nox_lantern__editorapi__EntityName(editor->backend, entity));
}

const char* lantern_editor_entity_kind(LanternEditor* editor, int64_t entity) {
    if (!editor) return "";
    return GiveString(editor, nox_lantern__editorapi__EntityKind(editor->backend, entity));
}

bool lantern_editor_is_selected(LanternEditor* editor, int64_t entity) {
    if (!editor) return false;
    return nox_lantern__editorapi__IsSelected(editor->backend, entity);
}

void lantern_editor_select(LanternEditor* editor, int64_t entity, bool additive) {
    if (!editor) return;
    nox_lantern__editorapi__Select(editor->backend, entity, additive);
}

void lantern_editor_clear_selection(LanternEditor* editor) {
    if (!editor) return;
    nox_lantern__editorapi__ClearSelection(editor->backend);
}

int64_t lantern_editor_primary_selection(LanternEditor* editor) {
    if (!editor) return 0;
    return nox_lantern__editorapi__PrimarySelection(editor->backend);
}

uint32_t lantern_editor_selection_count(LanternEditor* editor) {
    if (!editor) return 0;
    return nox_lantern__editorapi__SelectionCount(editor->backend);
}

uint32_t lantern_editor_field_count(LanternEditor* editor) {
    if (!editor) return 0;
    return nox_lantern__editorapi__FieldCount(editor->backend);
}

const char* lantern_editor_field(LanternEditor* editor, uint32_t index, uint32_t part) {
    if (!editor) return "";
    return GiveString(editor, nox_lantern__editorapi__Field(editor->backend, index, part));
}

uint32_t lantern_editor_field_choice_count(LanternEditor* editor, uint32_t index) {
    if (!editor) return 0;
    return nox_lantern__editorapi__FieldChoiceCount(editor->backend, index);
}

const char* lantern_editor_field_choice(LanternEditor* editor, uint32_t index, uint32_t choice) {
    if (!editor) return "";
    return GiveString(editor, nox_lantern__editorapi__FieldChoice(editor->backend, index, choice));
}

bool lantern_editor_set_field(LanternEditor* editor, int64_t entity, const char* field, const char* text) {
    if (!editor) return false;
    return nox_lantern__editorapi__SetField(editor->backend, entity, NoxText(field), NoxText(text));
}

void lantern_editor_commit_field(LanternEditor* editor) {
    if (!editor) return;
    nox_lantern__editorapi__CommitField(editor->backend);
}

uint32_t lantern_editor_template_count(LanternEditor* editor) {
    if (!editor) return 0;
    return nox_lantern__editorapi__TemplateCount(editor->backend);
}

const char* lantern_editor_template(LanternEditor* editor, uint32_t index, uint32_t part) {
    if (!editor) return "";
    return GiveString(editor, nox_lantern__editorapi__Template(editor->backend, index, part));
}

int64_t lantern_editor_add_entity(LanternEditor* editor, const char* template_id) {
    if (!editor) return 0;
    return nox_lantern__editorapi__AddEntity(editor->backend, NoxText(template_id));
}

bool lantern_editor_delete_selection(LanternEditor* editor) {
    if (!editor) return false;
    return nox_lantern__editorapi__DeleteSelection(editor->backend);
}

bool lantern_editor_duplicate_selection(LanternEditor* editor) {
    if (!editor) return false;
    return nox_lantern__editorapi__DuplicateSelection(editor->backend);
}

bool lantern_editor_undo(LanternEditor* editor) {
    if (!editor) return false;
    return nox_lantern__editorapi__Undo(editor->backend);
}

bool lantern_editor_redo(LanternEditor* editor) {
    if (!editor) return false;
    return nox_lantern__editorapi__Redo(editor->backend);
}

bool lantern_editor_can_undo(LanternEditor* editor) {
    if (!editor) return false;
    return nox_lantern__editorapi__CanUndo(editor->backend);
}

bool lantern_editor_can_redo(LanternEditor* editor) {
    if (!editor) return false;
    return nox_lantern__editorapi__CanRedo(editor->backend);
}

uint32_t lantern_editor_recent_file_count(LanternEditor* editor) {
    if (!editor) return 0;
    return nox_lantern__editorapi__RecentFileCount(editor->backend);
}

const char* lantern_editor_recent_file(LanternEditor* editor, uint32_t index) {
    if (!editor) return "";
    return GiveString(editor, nox_lantern__editorapi__RecentFile(editor->backend, index));
}

const char* lantern_editor_preference(LanternEditor* editor, const char* name) {
    if (!editor) return "";
    return GiveString(editor, nox_lantern__editorapi__Preference(editor->backend, NoxText(name)));
}

bool lantern_editor_set_preference(LanternEditor* editor, const char* name, const char* value) {
    if (!editor) return false;
    return nox_lantern__editorapi__SetPreference(editor->backend, NoxText(name), NoxText(value));
}

bool lantern_editor_save_preferences(LanternEditor* editor) {
    if (!editor) return false;
    return nox_lantern__editorapi__SavePreferences(editor->backend);
}

const char* lantern_editor_assets_directory(LanternEditor* editor) {
    if (!editor) return "";
    return GiveString(editor, nox_lantern__editorapi__AssetsDirectory(editor->backend));
}

void lantern_editor_set_space(LanternEditor* editor, const char* name) {
    if (!editor) return;
    nox_lantern__editorapi__SetSpace(editor->backend, NoxText(name));
}

const char* lantern_editor_space(LanternEditor* editor) {
    if (!editor) return "";
    return GiveString(editor, nox_lantern__editorapi__Space(editor->backend));
}

void lantern_editor_set_pivot_mode(LanternEditor* editor, const char* name) {
    if (!editor) return;
    nox_lantern__editorapi__SetPivotMode(editor->backend, NoxText(name));
}

const char* lantern_editor_pivot_mode(LanternEditor* editor) {
    if (!editor) return "";
    return GiveString(editor, nox_lantern__editorapi__PivotMode(editor->backend));
}

bool lantern_editor_set_snap_step(LanternEditor* editor, const char* name, float value) {
    if (!editor) return false;
    return nox_lantern__editorapi__SetSnapStep(editor->backend, NoxText(name), value);
}

float lantern_editor_snap_step(LanternEditor* editor, const char* name) {
    if (!editor) return 0.0f;
    return nox_lantern__editorapi__SnapStep(editor->backend, NoxText(name));
}

bool lantern_editor_set_view_option(LanternEditor* editor, const char* name, const char* value) {
    if (!editor) return false;
    return nox_lantern__editorapi__SetViewOption(editor->backend, NoxText(name), NoxText(value));
}

const char* lantern_editor_view_option(LanternEditor* editor, const char* name) {
    if (!editor) return "";
    return GiveString(editor, nox_lantern__editorapi__ViewOption(editor->backend, NoxText(name)));
}

const char* lantern_editor_hovered_handle(LanternEditor* editor) {
    if (!editor) return "";
    return GiveString(editor, nox_lantern__editorapi__HoveredHandle(editor->backend));
}

bool lantern_editor_camera_preset(LanternEditor* editor, const char* name) {
    if (!editor) return false;
    return nox_lantern__editorapi__CameraPreset(editor->backend, NoxText(name));
}

void lantern_editor_set_orthographic(LanternEditor* editor, bool orthographic) {
    if (!editor) return;
    nox_lantern__editorapi__SetOrthographic(editor->backend, orthographic);
}

bool lantern_editor_is_orthographic(LanternEditor* editor) {
    if (!editor) return false;
    return nox_lantern__editorapi__IsOrthographic(editor->backend);
}

const char* lantern_editor_camera_description(LanternEditor* editor) {
    if (!editor) return "";
    return GiveString(editor, nox_lantern__editorapi__CameraDescription(editor->backend));
}

void lantern_editor_frame_all(LanternEditor* editor) {
    if (!editor) return;
    nox_lantern__editorapi__FrameAll(editor->backend);
}

bool lantern_editor_set_bookmark(LanternEditor* editor, uint32_t slot) {
    if (!editor) return false;
    return nox_lantern__editorapi__SetBookmark(editor->backend, slot);
}

bool lantern_editor_go_to_bookmark(LanternEditor* editor, uint32_t slot) {
    if (!editor) return false;
    return nox_lantern__editorapi__GoToBookmark(editor->backend, slot);
}

bool lantern_editor_has_bookmark(LanternEditor* editor, uint32_t slot) {
    if (!editor) return false;
    return nox_lantern__editorapi__HasBookmark(editor->backend, slot);
}

bool lantern_editor_align_camera_to_selection(LanternEditor* editor) {
    if (!editor) return false;
    return nox_lantern__editorapi__AlignCameraToSelection(editor->backend);
}

bool lantern_editor_align_selection_to_camera(LanternEditor* editor) {
    if (!editor) return false;
    return nox_lantern__editorapi__AlignSelectionToCamera(editor->backend);
}

uint32_t lantern_editor_entity_count(LanternEditor* editor) {
    if (!editor) return 0;
    return nox_lantern__editorapi__EntityCount(editor->backend);
}

int64_t lantern_editor_outline_parent(LanternEditor* editor, uint32_t row) {
    if (!editor) return 0;
    return nox_lantern__editorapi__OutlineParent(editor->backend, row);
}

bool lantern_editor_outline_has_children(LanternEditor* editor, uint32_t row) {
    if (!editor) return false;
    return nox_lantern__editorapi__OutlineHasChildren(editor->backend, row);
}

bool lantern_editor_is_hidden(LanternEditor* editor, int64_t entity) {
    if (!editor) return false;
    return nox_lantern__editorapi__IsHidden(editor->backend, entity);
}

void lantern_editor_set_hidden(LanternEditor* editor, int64_t entity, bool hidden) {
    if (!editor) return;
    nox_lantern__editorapi__SetHidden(editor->backend, entity, hidden);
}

bool lantern_editor_is_locked(LanternEditor* editor, int64_t entity) {
    if (!editor) return false;
    return nox_lantern__editorapi__IsLocked(editor->backend, entity);
}

void lantern_editor_set_locked(LanternEditor* editor, int64_t entity, bool locked) {
    if (!editor) return;
    nox_lantern__editorapi__SetLocked(editor->backend, entity, locked);
}

void lantern_editor_hide_selection(LanternEditor* editor) {
    if (!editor) return;
    nox_lantern__editorapi__HideSelection(editor->backend);
}

void lantern_editor_unhide_all(LanternEditor* editor) {
    if (!editor) return;
    nox_lantern__editorapi__UnhideAll(editor->backend);
}

void lantern_editor_lock_selection(LanternEditor* editor, bool locked) {
    if (!editor) return;
    nox_lantern__editorapi__LockSelection(editor->backend, locked);
}

void lantern_editor_select_all(LanternEditor* editor) {
    if (!editor) return;
    nox_lantern__editorapi__SelectAll(editor->backend);
}

void lantern_editor_invert_selection(LanternEditor* editor) {
    if (!editor) return;
    nox_lantern__editorapi__InvertSelection(editor->backend);
}

void lantern_editor_select_kind(LanternEditor* editor, const char* kind) {
    if (!editor) return;
    nox_lantern__editorapi__SelectKind(editor->backend, NoxText(kind));
}

void lantern_editor_select_children(LanternEditor* editor) {
    if (!editor) return;
    nox_lantern__editorapi__SelectChildren(editor->backend);
}

bool lantern_editor_set_parent(LanternEditor* editor, int64_t child, int64_t parent) {
    if (!editor) return false;
    return nox_lantern__editorapi__SetParent(editor->backend, child, parent);
}

bool lantern_editor_set_selection_field(LanternEditor* editor, const char* field, const char* text) {
    if (!editor) return false;
    return nox_lantern__editorapi__SetSelectionField(editor->backend, NoxText(field), NoxText(text));
}

bool lantern_editor_rename_property(LanternEditor* editor, const char* old_name, const char* new_name) {
    if (!editor) return false;
    return nox_lantern__editorapi__RenameProperty(editor->backend, NoxText(old_name), NoxText(new_name));
}

int64_t lantern_editor_add_entity_at_cursor(LanternEditor* editor, const char* template_id) {
    if (!editor) return 0;
    return nox_lantern__editorapi__AddEntityAtCursor(editor->backend, NoxText(template_id));
}

bool lantern_editor_duplicate_along(LanternEditor* editor, const char* axis) {
    if (!editor) return false;
    return nox_lantern__editorapi__DuplicateAlong(editor->backend, NoxText(axis));
}

bool lantern_editor_mirror_selection(LanternEditor* editor, const char* axis) {
    if (!editor) return false;
    return nox_lantern__editorapi__MirrorSelection(editor->backend, NoxText(axis));
}

bool lantern_editor_snap_selection_to_ground(LanternEditor* editor) {
    if (!editor) return false;
    return nox_lantern__editorapi__SnapSelectionToGround(editor->backend);
}

bool lantern_editor_group_selection(LanternEditor* editor) {
    if (!editor) return false;
    return nox_lantern__editorapi__GroupSelection(editor->backend);
}

bool lantern_editor_align_selection(LanternEditor* editor, const char* axis) {
    if (!editor) return false;
    return nox_lantern__editorapi__AlignSelection(editor->backend, NoxText(axis));
}

bool lantern_editor_distribute_selection(LanternEditor* editor, const char* axis) {
    if (!editor) return false;
    return nox_lantern__editorapi__DistributeSelection(editor->backend, NoxText(axis));
}

bool lantern_editor_revert_to_template(LanternEditor* editor) {
    if (!editor) return false;
    return nox_lantern__editorapi__RevertToTemplate(editor->backend);
}

bool lantern_editor_apply_to_template(LanternEditor* editor) {
    if (!editor) return false;
    return nox_lantern__editorapi__ApplyToTemplate(editor->backend);
}

uint32_t lantern_editor_history_count(LanternEditor* editor) {
    if (!editor) return 0;
    return nox_lantern__editorapi__HistoryCount(editor->backend);
}

const char* lantern_editor_history_label(LanternEditor* editor, uint32_t index) {
    if (!editor) return "";
    return GiveString(editor, nox_lantern__editorapi__HistoryLabel(editor->backend, index));
}

uint32_t lantern_editor_history_position(LanternEditor* editor) {
    if (!editor) return 0;
    return nox_lantern__editorapi__HistoryPosition(editor->backend);
}

bool lantern_editor_jump_history(LanternEditor* editor, uint32_t position) {
    if (!editor) return false;
    return nox_lantern__editorapi__JumpHistory(editor->backend, position);
}

uint32_t lantern_editor_log_count(LanternEditor* editor) {
    if (!editor) return 0;
    return nox_lantern__editorapi__LogCount(editor->backend);
}

const char* lantern_editor_log_text(LanternEditor* editor, uint32_t index) {
    if (!editor) return "";
    return GiveString(editor, nox_lantern__editorapi__LogText(editor->backend, index));
}

const char* lantern_editor_log_level(LanternEditor* editor, uint32_t index) {
    if (!editor) return "";
    return GiveString(editor, nox_lantern__editorapi__LogLevel(editor->backend, index));
}

void lantern_editor_clear_log(LanternEditor* editor) {
    if (!editor) return;
    nox_lantern__editorapi__ClearLog(editor->backend);
}

bool lantern_editor_play(LanternEditor* editor) {
    if (!editor) return false;
    return nox_lantern__editorapi__Play(editor->backend);
}

bool lantern_editor_stop_play(LanternEditor* editor) {
    if (!editor) return false;
    return nox_lantern__editorapi__StopPlay(editor->backend);
}

bool lantern_editor_is_playing(LanternEditor* editor) {
    if (!editor) return false;
    return nox_lantern__editorapi__IsPlaying(editor->backend);
}

const char* lantern_editor_stat(LanternEditor* editor, const char* name) {
    if (!editor) return "";
    return GiveString(editor, nox_lantern__editorapi__Stat(editor->backend, NoxText(name)));
}

bool lantern_editor_render_preview(LanternEditor* editor, const char* template_id, uint32_t width, uint32_t height, float spin_radians) {
    if (!editor) return false;
    return nox_lantern__editorapi__RenderPreview(editor->backend, NoxText(template_id), width, height, spin_radians);
}
