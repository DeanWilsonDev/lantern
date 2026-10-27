#pragma once

#include "Penumbra/Widgets/Box.h"

#include <SDL3/SDL.h>

#include <array>
#include <cstdint>

struct LanternEditor;

namespace WorldEditor {

// The 3D viewport. Rust draws the scene off-screen (lantern_editor_render); this widget copies
// the pixels into a streaming texture and draws it in its content rect, and passes the
// pointer, wheel and keys back to Rust. All editing logic stays in Rust.
class LanternViewportWidget : public Penumbra::Widgets::Box {
public:
    explicit LanternViewportWidget(LanternEditor* Editor);
    ~LanternViewportWidget() override;

    LanternViewportWidget(const LanternViewportWidget&)            = delete;
    LanternViewportWidget& operator=(const LanternViewportWidget&) = delete;

    Penumbra::Point Measure(Penumbra::Point AvailableSizeLogical) override;
    void            Arrange(Penumbra::Rect FinalRectLogical) override;
    bool            UpdateInteractionState(const Penumbra::Platform::InputState& Input) override;

protected:
    void DrawContent(Penumbra::Render::Renderer& Renderer, Penumbra::Rect ContentRect) override;

private:
    static constexpr int PointerButtonCount = 3;

    void     ForwardPointer(const Penumbra::Platform::InputState& Input, Penumbra::Rect ContentRect);
    void     ForwardKeys(bool PointerIsOverViewport, std::uint32_t ModifierBits);
    bool     UploadPixels(SDL_Renderer* SdlRenderer);

    LanternEditor*  Editor;
    SDL_Texture*    ViewportTexture{nullptr};
    int             TextureWidthPhysical{0};
    int             TextureHeightPhysical{0};
    Penumbra::Point ContentSizeLogical{0.0f, 0.0f};
    float           DpiScaleFactor{1.0f};

    std::array<bool, PointerButtonCount> PointerButtonHeldByViewport{};
    std::array<bool, SDL_SCANCODE_COUNT> KeyHeldByViewport{};
    std::array<bool, SDL_SCANCODE_COUNT> KeyWasDownLastFrame{};
};

} // namespace WorldEditor
