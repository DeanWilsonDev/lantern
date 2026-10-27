#include "native/EditorViewportRenderers.h"

#include <lantern_editor.h>

#include <algorithm>
#include <cmath>
#include <numbers>

namespace WorldEditor {

std::uint32_t ToModifierBits(const Umbra::ViewportModifiers& Modifiers) {
    std::uint32_t Bits = 0;
    if (Modifiers.Shift) {
        Bits |= LANTERN_MODIFIER_SHIFT;
    }
    if (Modifiers.Shortcut) {
        Bits |= LANTERN_MODIFIER_COMMAND;
    }
    if (Modifiers.Alt) {
        Bits |= LANTERN_MODIFIER_ALT;
    }
    return Bits;
}

std::optional<Umbra::ViewportFrame> LevelViewportRenderer::Render(std::uint32_t WidthPixels,
                                                                  std::uint32_t HeightPixels) {
    if (!lantern_editor_render(Editor_, WidthPixels, HeightPixels)) {
        return std::nullopt;
    }
    Umbra::ViewportFrame Frame;
    Frame.Pixels = lantern_editor_pixels(Editor_, &Frame.Width, &Frame.Height);
    return Frame;
}

void LevelViewportRenderer::PointerMoved(float X, float Y) { lantern_editor_pointer_move(Editor_, X, Y); }

void LevelViewportRenderer::PointerButton(Umbra::ViewportPointerButton Button, bool Down,
                                          const Umbra::ViewportModifiers& Modifiers) {
    lantern_editor_pointer_button(Editor_, static_cast<std::uint32_t>(Button), Down, ToModifierBits(Modifiers));
}

void LevelViewportRenderer::Wheel(float Notches, float) { lantern_editor_wheel(Editor_, Notches); }

bool LevelViewportRenderer::Key(std::string_view KeyName, bool Down, const Umbra::ViewportModifiers& Modifiers) {
    const std::string Name(KeyName);
    return lantern_editor_key(Editor_, Name.c_str(), Down, ToModifierBits(Modifiers));
}

void LevelViewportRenderer::HoverChanged(bool PointerIsOver) {
    if (!PointerIsOver) {
        lantern_editor_release_keys(Editor_);
    }
}

void LevelViewportRenderer::Tick(float DeltaSeconds) { lantern_editor_update(Editor_, DeltaSeconds); }

std::optional<Umbra::ViewportFrame> TemplatePreviewRenderer::Render(std::uint32_t WidthPixels,
                                                                    std::uint32_t HeightPixels) {
    if (TemplateId_.empty()) {
        return std::nullopt;
    }
    if (!lantern_editor_render_preview(Editor_, TemplateId_.c_str(), WidthPixels, HeightPixels, SpinRadians_)) {
        return std::nullopt;
    }
    Umbra::ViewportFrame Frame;
    Frame.Pixels = lantern_editor_preview_pixels(Editor_, &Frame.Width, &Frame.Height);
    return Frame;
}

void TemplatePreviewRenderer::Tick(float DeltaSeconds) {
    constexpr float FullTurn = 2.0f * std::numbers::pi_v<float>;
    SpinRadians_             = std::fmod(SpinRadians_ + DeltaSeconds * TurnsPerSecondRadians, FullTurn);
}

std::optional<Umbra::ViewportFrame> TemplateThumbnailRenderer::Render(std::uint32_t WidthPixels,
                                                                      std::uint32_t HeightPixels) {
    const std::uint32_t Size = std::min(WidthPixels, HeightPixels);
    if (Size == 0) {
        return std::nullopt;
    }
    Umbra::ViewportFrame Frame;
    std::uint32_t        ActualSize = 0;
    Frame.Pixels                    = lantern_editor_thumbnail(Editor_, TemplateId_.c_str(), Size, &ActualSize);
    if (Frame.Pixels == nullptr || ActualSize == 0) {
        return std::nullopt;
    }
    Frame.Width  = ActualSize;
    Frame.Height = ActualSize;
    return Frame;
}

} // namespace WorldEditor
