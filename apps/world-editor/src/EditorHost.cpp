#include "EditorHost.h"

#include "PenumbraUiBackend/ChaosApplication.h"

#include <lantern_editor.h>

#include <cstdint>
#include <memory>
#include <string>

namespace WorldEditor {

namespace {

std::string CopyString(const char* Text) { return Text ? std::string(Text) : std::string(); }

std::uint32_t ToIndex(std::int32_t Value) { return Value < 0 ? 0u : static_cast<std::uint32_t>(Value); }

void RegisterDocumentFunctions(nyx::host::NyxRuntime& Runtime, LanternEditor* Editor) {
    Runtime.RegisterFunction("editorRevision",
                             [Editor]() -> std::int64_t { return static_cast<std::int64_t>(lantern_editor_revision(Editor)); });
    Runtime.RegisterFunction("editorStatus", [Editor]() -> std::string { return CopyString(lantern_editor_status(Editor)); });
    Runtime.RegisterFunction("editorTitle", [Editor]() -> std::string { return CopyString(lantern_editor_title(Editor)); });
    Runtime.RegisterFunction("editorDocumentPath",
                             [Editor]() -> std::string { return CopyString(lantern_editor_document_path(Editor)); });
    Runtime.RegisterFunction("editorIsDirty", [Editor]() -> bool { return lantern_editor_is_dirty(Editor); });
    Runtime.RegisterFunction("editorOpen",
                             [Editor](std::string Path) -> bool { return lantern_editor_open(Editor, Path.c_str()); });
    Runtime.RegisterFunction("editorSave", [Editor]() -> bool { return lantern_editor_save(Editor); });
    Runtime.RegisterFunction("editorSaveAs",
                             [Editor](std::string Path) -> bool { return lantern_editor_save_as(Editor, Path.c_str()); });
    Runtime.RegisterFunction("editorNewWorld",
                             [Editor](std::string Name) -> void { lantern_editor_new_world(Editor, Name.c_str()); });
    Runtime.RegisterFunction("editorRecentFileCount", [Editor]() -> std::int32_t {
        return static_cast<std::int32_t>(lantern_editor_recent_file_count(Editor));
    });
    Runtime.RegisterFunction("editorRecentFile", [Editor](std::int32_t Index) -> std::string {
        return CopyString(lantern_editor_recent_file(Editor, ToIndex(Index)));
    });
    Runtime.RegisterFunction("editorAssetsDirectory",
                             [Editor]() -> std::string { return CopyString(lantern_editor_assets_directory(Editor)); });
}

void RegisterPreferenceFunctions(nyx::host::NyxRuntime& Runtime, LanternEditor* Editor) {
    Runtime.RegisterFunction("editorPreference", [Editor](std::string Name) -> std::string {
        return CopyString(lantern_editor_preference(Editor, Name.c_str()));
    });
    Runtime.RegisterFunction("editorSetPreference", [Editor](std::string Name, std::string Value) -> bool {
        return lantern_editor_set_preference(Editor, Name.c_str(), Value.c_str());
    });
    Runtime.RegisterFunction("editorSavePreferences", [Editor]() -> bool { return lantern_editor_save_preferences(Editor); });
}

void RegisterViewportFunctions(nyx::host::NyxRuntime& Runtime, LanternEditor* Editor) {
    Runtime.RegisterFunction("editorSetTool",
                             [Editor](std::string Name) -> void { lantern_editor_set_tool(Editor, Name.c_str()); });
    Runtime.RegisterFunction("editorTool", [Editor]() -> std::string { return CopyString(lantern_editor_tool(Editor)); });
    Runtime.RegisterFunction("editorSetSnap", [Editor](bool Enabled) -> void { lantern_editor_set_snap(Editor, Enabled); });
    Runtime.RegisterFunction("editorSnap", [Editor]() -> bool { return lantern_editor_snap(Editor); });
    Runtime.RegisterFunction("editorSetSnapStep", [Editor](std::string Name, float Value) -> bool {
        return lantern_editor_set_snap_step(Editor, Name.c_str(), Value);
    });
    Runtime.RegisterFunction("editorSnapStep",
                             [Editor](std::string Name) -> float { return lantern_editor_snap_step(Editor, Name.c_str()); });
    Runtime.RegisterFunction("editorSetSpace",
                             [Editor](std::string Name) -> void { lantern_editor_set_space(Editor, Name.c_str()); });
    Runtime.RegisterFunction("editorSpace", [Editor]() -> std::string { return CopyString(lantern_editor_space(Editor)); });
    Runtime.RegisterFunction("editorSetPivotMode",
                             [Editor](std::string Name) -> void { lantern_editor_set_pivot_mode(Editor, Name.c_str()); });
    Runtime.RegisterFunction("editorPivotMode",
                             [Editor]() -> std::string { return CopyString(lantern_editor_pivot_mode(Editor)); });
    Runtime.RegisterFunction("editorSetViewOption", [Editor](std::string Name, std::string Value) -> bool {
        return lantern_editor_set_view_option(Editor, Name.c_str(), Value.c_str());
    });
    Runtime.RegisterFunction("editorViewOption", [Editor](std::string Name) -> std::string {
        return CopyString(lantern_editor_view_option(Editor, Name.c_str()));
    });
    Runtime.RegisterFunction("editorHoveredHandle",
                             [Editor]() -> std::string { return CopyString(lantern_editor_hovered_handle(Editor)); });
    Runtime.RegisterFunction("editorKey", [Editor](std::string KeyName, bool Down, std::int32_t Modifiers) -> bool {
        return lantern_editor_key(Editor, KeyName.c_str(), Down, ToIndex(Modifiers));
    });
}

void RegisterCameraFunctions(nyx::host::NyxRuntime& Runtime, LanternEditor* Editor) {
    Runtime.RegisterFunction("editorFrameSelection", [Editor]() -> void { lantern_editor_frame_selection(Editor); });
    Runtime.RegisterFunction("editorFrameAll", [Editor]() -> void { lantern_editor_frame_all(Editor); });
    Runtime.RegisterFunction("editorCameraPreset",
                             [Editor](std::string Name) -> bool { return lantern_editor_camera_preset(Editor, Name.c_str()); });
    Runtime.RegisterFunction("editorSetOrthographic",
                             [Editor](bool Orthographic) -> void { lantern_editor_set_orthographic(Editor, Orthographic); });
    Runtime.RegisterFunction("editorIsOrthographic", [Editor]() -> bool { return lantern_editor_is_orthographic(Editor); });
    Runtime.RegisterFunction("editorCameraDescription",
                             [Editor]() -> std::string { return CopyString(lantern_editor_camera_description(Editor)); });
    Runtime.RegisterFunction("editorSetBookmark",
                             [Editor](std::int32_t Slot) -> bool { return lantern_editor_set_bookmark(Editor, ToIndex(Slot)); });
    Runtime.RegisterFunction("editorGoToBookmark", [Editor](std::int32_t Slot) -> bool {
        return lantern_editor_go_to_bookmark(Editor, ToIndex(Slot));
    });
    Runtime.RegisterFunction("editorHasBookmark",
                             [Editor](std::int32_t Slot) -> bool { return lantern_editor_has_bookmark(Editor, ToIndex(Slot)); });
    Runtime.RegisterFunction("editorAlignCameraToSelection",
                             [Editor]() -> bool { return lantern_editor_align_camera_to_selection(Editor); });
    Runtime.RegisterFunction("editorAlignSelectionToCamera",
                             [Editor]() -> bool { return lantern_editor_align_selection_to_camera(Editor); });
}

void RegisterOutlineFunctions(nyx::host::NyxRuntime& Runtime, LanternEditor* Editor) {
    Runtime.RegisterFunction("editorOutlineCount", [Editor]() -> std::int32_t {
        return static_cast<std::int32_t>(lantern_editor_outline_count(Editor));
    });
    Runtime.RegisterFunction("editorEntityCount", [Editor]() -> std::int32_t {
        return static_cast<std::int32_t>(lantern_editor_entity_count(Editor));
    });
    Runtime.RegisterFunction("editorOutlineId", [Editor](std::int32_t Row) -> std::int64_t {
        return lantern_editor_outline_id(Editor, ToIndex(Row));
    });
    Runtime.RegisterFunction("editorOutlineDepth", [Editor](std::int32_t Row) -> std::int32_t {
        return static_cast<std::int32_t>(lantern_editor_outline_depth(Editor, ToIndex(Row)));
    });
    Runtime.RegisterFunction("editorOutlineParent", [Editor](std::int32_t Row) -> std::int64_t {
        return lantern_editor_outline_parent(Editor, ToIndex(Row));
    });
    Runtime.RegisterFunction("editorOutlineHasChildren", [Editor](std::int32_t Row) -> bool {
        return lantern_editor_outline_has_children(Editor, ToIndex(Row));
    });
    Runtime.RegisterFunction("editorEntityName", [Editor](std::int64_t Entity) -> std::string {
        return CopyString(lantern_editor_entity_name(Editor, Entity));
    });
    Runtime.RegisterFunction("editorEntityKind", [Editor](std::int64_t Entity) -> std::string {
        return CopyString(lantern_editor_entity_kind(Editor, Entity));
    });
    Runtime.RegisterFunction("editorIsSelected",
                             [Editor](std::int64_t Entity) -> bool { return lantern_editor_is_selected(Editor, Entity); });
    Runtime.RegisterFunction("editorIsHidden",
                             [Editor](std::int64_t Entity) -> bool { return lantern_editor_is_hidden(Editor, Entity); });
    Runtime.RegisterFunction("editorSetHidden", [Editor](std::int64_t Entity, bool Hidden) -> void {
        lantern_editor_set_hidden(Editor, Entity, Hidden);
    });
    Runtime.RegisterFunction("editorIsLocked",
                             [Editor](std::int64_t Entity) -> bool { return lantern_editor_is_locked(Editor, Entity); });
    Runtime.RegisterFunction("editorSetLocked", [Editor](std::int64_t Entity, bool Locked) -> void {
        lantern_editor_set_locked(Editor, Entity, Locked);
    });
    Runtime.RegisterFunction("editorHideSelection", [Editor]() -> void { lantern_editor_hide_selection(Editor); });
    Runtime.RegisterFunction("editorUnhideAll", [Editor]() -> void { lantern_editor_unhide_all(Editor); });
    Runtime.RegisterFunction("editorLockSelection",
                             [Editor](bool Locked) -> void { lantern_editor_lock_selection(Editor, Locked); });
    Runtime.RegisterFunction("editorSelect", [Editor](std::int64_t Entity, bool Additive) -> void {
        lantern_editor_commit_field(Editor);
        lantern_editor_select(Editor, Entity, Additive);
    });
    Runtime.RegisterFunction("editorClearSelection", [Editor]() -> void { lantern_editor_clear_selection(Editor); });
    Runtime.RegisterFunction("editorSelectAll", [Editor]() -> void { lantern_editor_select_all(Editor); });
    Runtime.RegisterFunction("editorInvertSelection", [Editor]() -> void { lantern_editor_invert_selection(Editor); });
    Runtime.RegisterFunction("editorSelectKind",
                             [Editor](std::string Kind) -> void { lantern_editor_select_kind(Editor, Kind.c_str()); });
    Runtime.RegisterFunction("editorSelectChildren", [Editor]() -> void { lantern_editor_select_children(Editor); });
    Runtime.RegisterFunction("editorPrimarySelection",
                             [Editor]() -> std::int64_t { return lantern_editor_primary_selection(Editor); });
    Runtime.RegisterFunction("editorSelectionCount", [Editor]() -> std::int32_t {
        return static_cast<std::int32_t>(lantern_editor_selection_count(Editor));
    });
    Runtime.RegisterFunction("editorSetParent", [Editor](std::int64_t Child, std::int64_t Parent) -> bool {
        return lantern_editor_set_parent(Editor, Child, Parent);
    });
}

void RegisterInspectorFunctions(nyx::host::NyxRuntime& Runtime, LanternEditor* Editor) {
    Runtime.RegisterFunction("editorFieldCount", [Editor]() -> std::int32_t {
        return static_cast<std::int32_t>(lantern_editor_field_count(Editor));
    });
    Runtime.RegisterFunction("editorField", [Editor](std::int32_t Index, std::int32_t Part) -> std::string {
        return CopyString(lantern_editor_field(Editor, ToIndex(Index), ToIndex(Part)));
    });
    Runtime.RegisterFunction("editorFieldChoiceCount", [Editor](std::int32_t Index) -> std::int32_t {
        return static_cast<std::int32_t>(lantern_editor_field_choice_count(Editor, ToIndex(Index)));
    });
    Runtime.RegisterFunction("editorFieldChoice", [Editor](std::int32_t Index, std::int32_t Choice) -> std::string {
        return CopyString(lantern_editor_field_choice(Editor, ToIndex(Index), ToIndex(Choice)));
    });
    Runtime.RegisterFunction("editorSetField", [Editor](std::int64_t Entity, std::string Field, std::string Text) -> bool {
        return lantern_editor_set_field(Editor, Entity, Field.c_str(), Text.c_str());
    });
    Runtime.RegisterFunction("editorSetSelectionField", [Editor](std::string Field, std::string Text) -> bool {
        return lantern_editor_set_selection_field(Editor, Field.c_str(), Text.c_str());
    });
    Runtime.RegisterFunction("editorCommitField", [Editor]() -> void { lantern_editor_commit_field(Editor); });
    Runtime.RegisterFunction("editorRenameProperty", [Editor](std::string OldName, std::string NewName) -> bool {
        return lantern_editor_rename_property(Editor, OldName.c_str(), NewName.c_str());
    });
}

void RegisterEditingFunctions(nyx::host::NyxRuntime& Runtime, LanternEditor* Editor) {
    Runtime.RegisterFunction("editorTemplateCount", [Editor]() -> std::int32_t {
        return static_cast<std::int32_t>(lantern_editor_template_count(Editor));
    });
    Runtime.RegisterFunction("editorTemplate", [Editor](std::int32_t Index, std::int32_t Part) -> std::string {
        return CopyString(lantern_editor_template(Editor, ToIndex(Index), ToIndex(Part)));
    });
    Runtime.RegisterFunction("editorAddEntity", [Editor](std::string TemplateId) -> std::int64_t {
        lantern_editor_commit_field(Editor);
        return lantern_editor_add_entity(Editor, TemplateId.c_str());
    });
    Runtime.RegisterFunction("editorAddEntityAtCursor", [Editor](std::string TemplateId) -> std::int64_t {
        lantern_editor_commit_field(Editor);
        return lantern_editor_add_entity_at_cursor(Editor, TemplateId.c_str());
    });
    Runtime.RegisterFunction("editorDeleteSelection", [Editor]() -> bool { return lantern_editor_delete_selection(Editor); });
    Runtime.RegisterFunction("editorDuplicateSelection",
                             [Editor]() -> bool { return lantern_editor_duplicate_selection(Editor); });
    Runtime.RegisterFunction("editorDuplicateAlong",
                             [Editor](std::string Axis) -> bool { return lantern_editor_duplicate_along(Editor, Axis.c_str()); });
    Runtime.RegisterFunction("editorMirrorSelection",
                             [Editor](std::string Axis) -> bool { return lantern_editor_mirror_selection(Editor, Axis.c_str()); });
    Runtime.RegisterFunction("editorSnapSelectionToGround",
                             [Editor]() -> bool { return lantern_editor_snap_selection_to_ground(Editor); });
    Runtime.RegisterFunction("editorGroupSelection", [Editor]() -> bool { return lantern_editor_group_selection(Editor); });
    Runtime.RegisterFunction("editorAlignSelection",
                             [Editor](std::string Axis) -> bool { return lantern_editor_align_selection(Editor, Axis.c_str()); });
    Runtime.RegisterFunction("editorDistributeSelection", [Editor](std::string Axis) -> bool {
        return lantern_editor_distribute_selection(Editor, Axis.c_str());
    });
    Runtime.RegisterFunction("editorRevertToTemplate", [Editor]() -> bool { return lantern_editor_revert_to_template(Editor); });
    Runtime.RegisterFunction("editorApplyToTemplate", [Editor]() -> bool { return lantern_editor_apply_to_template(Editor); });
    Runtime.RegisterFunction("editorUndo", [Editor]() -> bool {
        lantern_editor_commit_field(Editor);
        return lantern_editor_undo(Editor);
    });
    Runtime.RegisterFunction("editorRedo", [Editor]() -> bool { return lantern_editor_redo(Editor); });
    Runtime.RegisterFunction("editorCanUndo", [Editor]() -> bool { return lantern_editor_can_undo(Editor); });
    Runtime.RegisterFunction("editorCanRedo", [Editor]() -> bool { return lantern_editor_can_redo(Editor); });
}

void RegisterHistoryAndLogFunctions(nyx::host::NyxRuntime& Runtime, LanternEditor* Editor) {
    Runtime.RegisterFunction("editorHistoryCount", [Editor]() -> std::int32_t {
        return static_cast<std::int32_t>(lantern_editor_history_count(Editor));
    });
    Runtime.RegisterFunction("editorHistoryLabel", [Editor](std::int32_t Index) -> std::string {
        return CopyString(lantern_editor_history_label(Editor, ToIndex(Index)));
    });
    Runtime.RegisterFunction("editorHistoryPosition", [Editor]() -> std::int32_t {
        return static_cast<std::int32_t>(lantern_editor_history_position(Editor));
    });
    Runtime.RegisterFunction("editorJumpHistory", [Editor](std::int32_t Position) -> bool {
        lantern_editor_commit_field(Editor);
        return lantern_editor_jump_history(Editor, ToIndex(Position));
    });
    Runtime.RegisterFunction("editorLogCount", [Editor]() -> std::int32_t {
        return static_cast<std::int32_t>(lantern_editor_log_count(Editor));
    });
    Runtime.RegisterFunction("editorLogText", [Editor](std::int32_t Index) -> std::string {
        return CopyString(lantern_editor_log_text(Editor, ToIndex(Index)));
    });
    Runtime.RegisterFunction("editorLogLevel", [Editor](std::int32_t Index) -> std::string {
        return CopyString(lantern_editor_log_level(Editor, ToIndex(Index)));
    });
    Runtime.RegisterFunction("editorClearLog", [Editor]() -> void { lantern_editor_clear_log(Editor); });
}

void RegisterPlayAndStatFunctions(nyx::host::NyxRuntime& Runtime, LanternEditor* Editor) {
    Runtime.RegisterFunction("editorPlay", [Editor]() -> bool {
        lantern_editor_commit_field(Editor);
        return lantern_editor_play(Editor);
    });
    Runtime.RegisterFunction("editorStopPlay", [Editor]() -> bool { return lantern_editor_stop_play(Editor); });
    Runtime.RegisterFunction("editorIsPlaying", [Editor]() -> bool { return lantern_editor_is_playing(Editor); });
    Runtime.RegisterFunction("editorStat", [Editor](std::string Name) -> std::string {
        return CopyString(lantern_editor_stat(Editor, Name.c_str()));
    });
}

} // namespace

void LanternEditorDeleter::operator()(LanternEditor* Editor) const { lantern_editor_destroy(Editor); }

PenumbraUiBackend::ChaosApplicationHostOptions EditorHostOptions() {
    PenumbraUiBackend::ChaosApplicationHostOptions Options;
    Options.UiDirectory          = WORLD_EDITOR_UI_DIR;
    Options.ApplicationFile      = "EditorApplication.nyx";
    Options.ApplicationClass     = "EditorApplication";
    Options.NyxSourceDirectories = {WORLD_EDITOR_UI_DIR};
    Options.FontPath             = WORLD_EDITOR_FONT_PATH;
    Options.FontSizeLogical      = 12.0f;
    return Options;
}

LanternEditorHandle OpenEditorBackend(const std::string& AssetsDirectory, const std::string& WorldPath) {
    return LanternEditorHandle(
        lantern_editor_create(AssetsDirectory.c_str(), WorldPath.empty() ? nullptr : WorldPath.c_str()));
}

EditorHost::EditorHost(LanternEditor* Editor) : Editor_(Editor), LevelRenderer_(Editor), PreviewRenderer_(Editor) {
    const std::uint32_t TemplateCount = lantern_editor_template_count(Editor);
    for (std::uint32_t Index = 0; Index < TemplateCount; ++Index) {
        ThumbnailRenderers_.push_back(std::make_unique<TemplateThumbnailRenderer>(
            Editor, CopyString(lantern_editor_template(Editor, Index, LANTERN_TEMPLATE_ID))));
    }
}

void EditorHost::RegisterHostFunctions(PenumbraUiBackend::ChaosApplicationHost& Host) {
    nyx::host::NyxRuntime& Runtime = Host.Runtime();
    RegisterDocumentFunctions(Runtime, Editor_);
    RegisterPreferenceFunctions(Runtime, Editor_);
    RegisterViewportFunctions(Runtime, Editor_);
    RegisterCameraFunctions(Runtime, Editor_);
    RegisterOutlineFunctions(Runtime, Editor_);
    RegisterInspectorFunctions(Runtime, Editor_);
    RegisterEditingFunctions(Runtime, Editor_);
    RegisterHistoryAndLogFunctions(Runtime, Editor_);
    RegisterPlayAndStatFunctions(Runtime, Editor_);

    TemplatePreviewRenderer* Preview = &PreviewRenderer_;
    Runtime.RegisterFunction("editorSetPreviewTemplate",
                             [Preview](std::string TemplateId) -> void { Preview->SetTemplate(std::move(TemplateId)); });
    Runtime.RegisterFunction("editorPreviewTemplate", [Preview]() -> std::string { return Preview->Template(); });
}

void EditorHost::RegisterViewportRenderers(PenumbraUiBackend::ChaosApplication& Application) {
    Application.RegisterViewportRenderer("level", &LevelRenderer_);
    Application.RegisterViewportRenderer("preview", &PreviewRenderer_);
    const std::uint32_t TemplateCount = lantern_editor_template_count(Editor_);
    for (std::uint32_t Index = 0; Index < TemplateCount && Index < ThumbnailRenderers_.size(); ++Index) {
        Application.RegisterViewportRenderer("thumbnail:" + CopyString(lantern_editor_template(Editor_, Index, LANTERN_TEMPLATE_ID)),
                                             ThumbnailRenderers_[Index].get());
    }
}

} // namespace WorldEditor
