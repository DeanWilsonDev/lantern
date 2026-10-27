#include "EditorHost.h"

#include "native/LanternViewportWidget.h"

#include "PenumbraUiBackend/PenumbraWidgetAdapter.h"

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
}

void RegisterViewportFunctions(nyx::host::NyxRuntime& Runtime, LanternEditor* Editor) {
    Runtime.RegisterFunction("editorSetTool",
                             [Editor](std::string Name) -> void { lantern_editor_set_tool(Editor, Name.c_str()); });
    Runtime.RegisterFunction("editorTool", [Editor]() -> std::string { return CopyString(lantern_editor_tool(Editor)); });
    Runtime.RegisterFunction("editorSetSnap", [Editor](bool Enabled) -> void { lantern_editor_set_snap(Editor, Enabled); });
    Runtime.RegisterFunction("editorSnap", [Editor]() -> bool { return lantern_editor_snap(Editor); });
    Runtime.RegisterFunction("editorFrameSelection", [Editor]() -> void { lantern_editor_frame_selection(Editor); });
}

void RegisterOutlineFunctions(nyx::host::NyxRuntime& Runtime, LanternEditor* Editor) {
    Runtime.RegisterFunction("editorOutlineCount", [Editor]() -> std::int32_t {
        return static_cast<std::int32_t>(lantern_editor_outline_count(Editor));
    });
    Runtime.RegisterFunction("editorOutlineId", [Editor](std::int32_t Row) -> std::int64_t {
        return lantern_editor_outline_id(Editor, ToIndex(Row));
    });
    Runtime.RegisterFunction("editorOutlineDepth", [Editor](std::int32_t Row) -> std::int32_t {
        return static_cast<std::int32_t>(lantern_editor_outline_depth(Editor, ToIndex(Row)));
    });
    Runtime.RegisterFunction("editorEntityName", [Editor](std::int64_t Entity) -> std::string {
        return CopyString(lantern_editor_entity_name(Editor, Entity));
    });
    Runtime.RegisterFunction("editorEntityKind", [Editor](std::int64_t Entity) -> std::string {
        return CopyString(lantern_editor_entity_kind(Editor, Entity));
    });
    Runtime.RegisterFunction("editorIsSelected",
                             [Editor](std::int64_t Entity) -> bool { return lantern_editor_is_selected(Editor, Entity); });
    Runtime.RegisterFunction("editorSelect", [Editor](std::int64_t Entity, bool Additive) -> void {
        lantern_editor_commit_field(Editor);
        lantern_editor_select(Editor, Entity, Additive);
    });
    Runtime.RegisterFunction("editorClearSelection", [Editor]() -> void { lantern_editor_clear_selection(Editor); });
    Runtime.RegisterFunction("editorPrimarySelection",
                             [Editor]() -> std::int64_t { return lantern_editor_primary_selection(Editor); });
    Runtime.RegisterFunction("editorSelectionCount", [Editor]() -> std::int32_t {
        return static_cast<std::int32_t>(lantern_editor_selection_count(Editor));
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
    Runtime.RegisterFunction("editorCommitField", [Editor]() -> void { lantern_editor_commit_field(Editor); });
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
    Runtime.RegisterFunction("editorDeleteSelection", [Editor]() -> bool { return lantern_editor_delete_selection(Editor); });
    Runtime.RegisterFunction("editorDuplicateSelection",
                             [Editor]() -> bool { return lantern_editor_duplicate_selection(Editor); });
    Runtime.RegisterFunction("editorUndo", [Editor]() -> bool {
        lantern_editor_commit_field(Editor);
        return lantern_editor_undo(Editor);
    });
    Runtime.RegisterFunction("editorRedo", [Editor]() -> bool { return lantern_editor_redo(Editor); });
    Runtime.RegisterFunction("editorCanUndo", [Editor]() -> bool { return lantern_editor_can_undo(Editor); });
    Runtime.RegisterFunction("editorCanRedo", [Editor]() -> bool { return lantern_editor_can_redo(Editor); });
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

void RegisterEditorHostFunctions(PenumbraUiBackend::ChaosApplicationHost& Host, LanternEditor* Editor) {
    nyx::host::NyxRuntime& Runtime = Host.Runtime();
    RegisterDocumentFunctions(Runtime, Editor);
    RegisterViewportFunctions(Runtime, Editor);
    RegisterOutlineFunctions(Runtime, Editor);
    RegisterInspectorFunctions(Runtime, Editor);
    RegisterEditingFunctions(Runtime, Editor);

    Host.Bridge().ChaosDriver().RegisterNativeBuilder("LanternViewport", [Editor]() -> std::unique_ptr<Umbra::IWidget> {
        return std::make_unique<PenumbraUiBackend::PenumbraWidget>(std::make_unique<LanternViewportWidget>(Editor));
    });
}

} // namespace WorldEditor
