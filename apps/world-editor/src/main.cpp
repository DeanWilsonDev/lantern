#include "EditorHost.h"

#include <cstdio>

int main(int argc, char** argv) {
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
