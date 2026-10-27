#pragma once

#include "PenumbraUiBackend/ChaosApplicationHost.h"

#include <memory>
#include <string>

struct LanternEditor;

namespace WorldEditor {

struct LanternEditorDeleter {
    void operator()(LanternEditor* Editor) const;
};

using LanternEditorHandle = std::unique_ptr<LanternEditor, LanternEditorDeleter>;

PenumbraUiBackend::ChaosApplicationHostOptions EditorHostOptions();

// Opens the Rust backend on Lantern's assets and the given world (empty: the demo course).
LanternEditorHandle OpenEditorBackend(const std::string& AssetsDirectory, const std::string& WorldPath);

// The Nyx functions the UI calls (`editorRevision()`, `editorSelect(id, additive)`...), each a
// thin wrapper over one lantern_editor_* call, and the `LanternViewport` native builder.
void RegisterEditorHostFunctions(PenumbraUiBackend::ChaosApplicationHost& Host, LanternEditor* Editor);

} // namespace WorldEditor
