#pragma once

#include "Umbra/IViewportRenderer.h"

#include <cstdint>
#include <optional>
#include <string>
#include <string_view>

struct LanternEditor;

namespace WorldEditor {

// Each renderer draws through the Rust backend's C API and hands the pixels back to the
// `<viewport renderer="...">` element that shows it. All editing logic stays in Rust; these
// classes only translate Umbra's viewport contract into lantern_editor_* calls.

std::uint32_t ToModifierBits(const Umbra::ViewportModifiers& Modifiers);

// The level view: the scene, the gizmo and the camera, with pointer, wheel and key input.
// Registered as "level".
class LevelViewportRenderer final : public Umbra::IViewportRenderer {
public:
    explicit LevelViewportRenderer(LanternEditor* Editor) : Editor_(Editor) {}

    std::optional<Umbra::ViewportFrame> Render(std::uint32_t WidthPixels, std::uint32_t HeightPixels) override;
    void                                PointerMoved(float X, float Y) override;
    void PointerButton(Umbra::ViewportPointerButton Button, bool Down, const Umbra::ViewportModifiers& Modifiers) override;
    void Wheel(float Notches, float NotchesX) override;
    bool Key(std::string_view KeyName, bool Down, const Umbra::ViewportModifiers& Modifiers) override;
    void HoverChanged(bool PointerIsOver) override;
    void Tick(float DeltaSeconds) override;

private:
    LanternEditor* Editor_;
};

// A turntable of one palette template, for the asset browser's preview pane. The template is
// chosen from Nyx (`editorSetPreviewTemplate`). Registered as "preview".
class TemplatePreviewRenderer final : public Umbra::IViewportRenderer {
public:
    explicit TemplatePreviewRenderer(LanternEditor* Editor) : Editor_(Editor) {}

    void               SetTemplate(std::string TemplateId) { TemplateId_ = std::move(TemplateId); }
    const std::string& Template() const { return TemplateId_; }

    std::optional<Umbra::ViewportFrame> Render(std::uint32_t WidthPixels, std::uint32_t HeightPixels) override;
    void                                Tick(float DeltaSeconds) override;

private:
    static constexpr float TurnsPerSecondRadians = 0.6f;

    LanternEditor* Editor_;
    std::string    TemplateId_;
    float          SpinRadians_{0.0f};
};

// A still thumbnail of one template, drawn once per palette revision by the backend and kept.
// Registered as "thumbnail:<template id>".
class TemplateThumbnailRenderer final : public Umbra::IViewportRenderer {
public:
    TemplateThumbnailRenderer(LanternEditor* Editor, std::string TemplateId)
        : Editor_(Editor), TemplateId_(std::move(TemplateId)) {}

    std::optional<Umbra::ViewportFrame> Render(std::uint32_t WidthPixels, std::uint32_t HeightPixels) override;

private:
    LanternEditor* Editor_;
    std::string    TemplateId_;
};

} // namespace WorldEditor
