#include "EditorHost.h"

#include "PenumbraUiBackend/ChaosApplication.h"

#include <host/host-surface-manifest.hpp>

#include <cstdio>
#include <fstream>
#include <memory>
#include <string_view>

namespace {

constexpr const char* kHostSurfaceApplicationSource = "EditorHostSurfaceApplication :: class : Application {\n}\n";

int WriteEditorHostSurface(const char* OutputPath) {
    WorldEditor::LanternEditorHandle Editor = WorldEditor::OpenEditorBackend(LANTERN_ASSETS_DIR, "");
    if (!Editor) {
        std::fprintf(stderr, "The editor backend failed to start.\n");
        return 1;
    }
    WorldEditor::EditorHost                 EditorHost(Editor.get());
    PenumbraUiBackend::ChaosApplicationHost Host(WorldEditor::EditorHostOptions());
    EditorHost.RegisterHostFunctions(Host);
    const std::unique_ptr<PenumbraUiBackend::ChaosApplication> Application(Host.Bridge().LoadApplication(
        kHostSurfaceApplicationSource, "editor-host-surface.nyx", "EditorHostSurfaceApplication"));
    if (!Application) {
        std::fprintf(stderr, "Failed to describe the editor's host surface:\n%s", Host.LastDiagnostics().c_str());
        return 1;
    }
    std::ofstream Output(OutputPath, std::ios::binary | std::ios::trunc);
    Output << nyx::host::WriteHostSurfaceManifest(Host.Runtime());
    if (!Output) {
        std::fprintf(stderr, "Can't write %s\n", OutputPath);
        return 1;
    }
    return 0;
}

} // namespace

int main(int argc, char** argv) {
    if (argc == 3 && std::string_view(argv[1]) == "--host-surface") {
        return WriteEditorHostSurface(argv[2]);
    }
    WorldEditor::LanternEditorHandle Editor =
        WorldEditor::OpenEditorBackend(LANTERN_ASSETS_DIR, argc > 1 ? argv[1] : "");
    if (!Editor) {
        std::fprintf(stderr, "The editor backend failed to start.\n");
        return 1;
    }
    WorldEditor::EditorHost                 EditorHost(Editor.get());
    PenumbraUiBackend::ChaosApplicationHost Host(WorldEditor::EditorHostOptions());
    EditorHost.RegisterHostFunctions(Host);
    PenumbraUiBackend::ChaosApplication* Application = Host.Load();
    if (Application == nullptr) {
        std::fprintf(stderr, "Failed to load the editor UI:\n%s", Host.LastDiagnostics().c_str());
        return 1;
    }
    EditorHost.RegisterViewportRenderers(*Application);
    return Host.Run();
}
