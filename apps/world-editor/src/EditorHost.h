#pragma once

#include "PenumbraUiBackend/ChaosApplicationHost.h"
#include "native/EditorViewportRenderers.h"

#include <memory>
#include <string>
#include <vector>

struct LanternEditor;

namespace WorldEditor {

struct LanternEditorDeleter {
    void operator()(LanternEditor* Editor) const;
};

using LanternEditorHandle = std::unique_ptr<LanternEditor, LanternEditorDeleter>;

PenumbraUiBackend::ChaosApplicationHostOptions EditorHostOptions();

// Opens the Rust backend on Lantern's assets and the given world (empty: the demo course).
LanternEditorHandle OpenEditorBackend(const std::string& AssetsDirectory, const std::string& WorldPath);

// What the UI reaches the backend through: the Nyx functions the UI calls (`editorRevision()`,
// `editorSelect(id, additive)`...), each a thin wrapper over one lantern_editor_* call, and the
// viewport renderers behind `<viewport renderer="level">`, `"preview"` and
// `"thumbnail:<template>"`. Register the functions before the host loads the application and
// the renderers after, on the loaded ChaosApplication. The renderers outlive the widgets that
// show them, so this object must outlive the host.
class EditorHost {
public:
    explicit EditorHost(LanternEditor* Editor);

    void RegisterHostFunctions(PenumbraUiBackend::ChaosApplicationHost& Host);
    void RegisterViewportRenderers(PenumbraUiBackend::ChaosApplication& Application);

private:
    LanternEditor*                                          Editor_;
    LevelViewportRenderer                                   LevelRenderer_;
    TemplatePreviewRenderer                                 PreviewRenderer_;
    std::vector<std::unique_ptr<TemplateThumbnailRenderer>> ThumbnailRenderers_;
};

} // namespace WorldEditor
