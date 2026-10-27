#include "EditorHost.h"

#include <cstdio>

int main(int argc, char** argv) {
    WorldEditor::LanternEditorHandle Editor =
        WorldEditor::OpenEditorBackend(LANTERN_ASSETS_DIR, argc > 1 ? argv[1] : "");
    if (!Editor) {
        std::fprintf(stderr, "The editor backend failed to start.\n");
        return 1;
    }
    PenumbraUiBackend::ChaosApplicationHost Host(WorldEditor::EditorHostOptions());
    WorldEditor::RegisterEditorHostFunctions(Host, Editor.get());
    return Host.Run();
}
