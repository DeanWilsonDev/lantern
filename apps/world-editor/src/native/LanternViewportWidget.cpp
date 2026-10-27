#include "native/LanternViewportWidget.h"

#include <lantern_editor.h>

#include <cmath>

namespace WorldEditor {

namespace {

std::uint32_t ModifierBitsFrom(const Penumbra::Platform::Modifiers& Modifiers) {
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

// The editor's key code for a physical key, or 0 for keys the viewport doesn't use. Letters go
// by the key's position (WASD flies the same way on every layout).
std::uint32_t EditorKeyCodeFor(SDL_Scancode Scancode) {
    switch (Scancode) {
        case SDL_SCANCODE_DELETE:
            return LANTERN_KEY_DELETE;
        case SDL_SCANCODE_BACKSPACE:
            return LANTERN_KEY_BACKSPACE;
        case SDL_SCANCODE_ESCAPE:
            return LANTERN_KEY_ESCAPE;
        default:
            break;
    }
    if (Scancode >= SDL_SCANCODE_A && Scancode <= SDL_SCANCODE_Z) {
        return static_cast<std::uint32_t>('a' + (Scancode - SDL_SCANCODE_A));
    }
    return 0;
}

} // namespace

LanternViewportWidget::LanternViewportWidget(LanternEditor* Editor) : Editor(Editor) {
    Style.FlexGrow = 1.0f;
}

LanternViewportWidget::~LanternViewportWidget() {
    if (ViewportTexture) {
        SDL_DestroyTexture(ViewportTexture);
    }
}

Penumbra::Point LanternViewportWidget::Measure(Penumbra::Point AvailableSizeLogical) {
    return AvailableSizeLogical;
}

void LanternViewportWidget::Arrange(Penumbra::Rect FinalRectLogical) {
    Box::Arrange(FinalRectLogical);
    const Penumbra::Rect Content = ContentRectFrom(FinalRectLogical);
    ContentSizeLogical           = {Content.W, Content.H};
}

bool LanternViewportWidget::UpdateInteractionState(const Penumbra::Platform::InputState& Input) {
    lantern_editor_update(Editor, Input.DeltaTimeSeconds);

    const Penumbra::Rect Content         = ContentRectFrom(ArrangedRect);
    const bool           PointerIsOver   = Input.IsPointerOver(Content);
    bool                 DragIsUnderway  = false;
    for (const bool Held : PointerButtonHeldByViewport) {
        DragIsUnderway = DragIsUnderway || Held;
    }
    const std::uint32_t ModifierBits = ModifierBitsFrom(Input.ModifierState);

    if (PointerIsOver || DragIsUnderway) {
        ForwardPointer(Input, Content);
    }
    ForwardKeys(PointerIsOver || DragIsUnderway, ModifierBits);
    return PointerIsOver || DragIsUnderway;
}

void LanternViewportWidget::ForwardPointer(const Penumbra::Platform::InputState& Input, Penumbra::Rect ContentRect) {
    const float PointerXPhysical = (Input.MousePosition.X - ContentRect.X) * DpiScaleFactor;
    const float PointerYPhysical = (Input.MousePosition.Y - ContentRect.Y) * DpiScaleFactor;
    lantern_editor_pointer_move(Editor, PointerXPhysical, PointerYPhysical);

    const std::uint32_t ModifierBits  = ModifierBitsFrom(Input.ModifierState);
    const bool          PointerIsOver = Input.IsPointerOver(ContentRect);
    for (int Button = 0; Button < PointerButtonCount; ++Button) {
        if (Input.MouseButtonPressedThisFrame[Button] && PointerIsOver && !PointerButtonHeldByViewport[Button]) {
            lantern_editor_commit_field(Editor);
            lantern_editor_pointer_button(Editor, static_cast<std::uint32_t>(Button), true, ModifierBits);
            PointerButtonHeldByViewport[Button] = true;
        }
        if (Input.MouseButtonReleasedThisFrame[Button] && PointerButtonHeldByViewport[Button]) {
            lantern_editor_pointer_button(Editor, static_cast<std::uint32_t>(Button), false, ModifierBits);
            PointerButtonHeldByViewport[Button] = false;
        }
    }

    if (PointerIsOver && Input.MouseWheelDelta != 0.0f) {
        lantern_editor_wheel(Editor, Input.MouseWheelDelta);
    }
}

void LanternViewportWidget::ForwardKeys(bool PointerIsOverViewport, std::uint32_t ModifierBits) {
    int         KeyCount      = 0;
    const bool* KeyboardState = SDL_GetKeyboardState(&KeyCount);
    if (!KeyboardState) {
        return;
    }
    for (int ScancodeIndex = 0; ScancodeIndex < KeyCount && ScancodeIndex < SDL_SCANCODE_COUNT; ++ScancodeIndex) {
        const auto          Scancode = static_cast<SDL_Scancode>(ScancodeIndex);
        const std::uint32_t KeyCode  = EditorKeyCodeFor(Scancode);
        if (KeyCode == 0) {
            continue;
        }
        const bool IsDown = KeyboardState[ScancodeIndex];
        bool&      Held   = KeyHeldByViewport[static_cast<std::size_t>(ScancodeIndex)];
        if (IsDown && !Held && PointerIsOverViewport && !KeyWasDownLastFrame[static_cast<std::size_t>(ScancodeIndex)]) {
            lantern_editor_key(Editor, KeyCode, true, ModifierBits);
            Held = true;
        } else if (!IsDown && Held) {
            lantern_editor_key(Editor, KeyCode, false, ModifierBits);
            Held = false;
        }
        KeyWasDownLastFrame[static_cast<std::size_t>(ScancodeIndex)] = IsDown;
    }
}

bool LanternViewportWidget::UploadPixels(SDL_Renderer* SdlRenderer) {
    std::uint32_t        PixelWidth  = 0;
    std::uint32_t        PixelHeight = 0;
    const std::uint8_t*  Pixels      = lantern_editor_pixels(Editor, &PixelWidth, &PixelHeight);
    if (!Pixels || PixelWidth == 0 || PixelHeight == 0) {
        return false;
    }
    const int Width  = static_cast<int>(PixelWidth);
    const int Height = static_cast<int>(PixelHeight);
    if (!ViewportTexture || Width != TextureWidthPhysical || Height != TextureHeightPhysical) {
        if (ViewportTexture) {
            SDL_DestroyTexture(ViewportTexture);
        }
        ViewportTexture = SDL_CreateTexture(SdlRenderer, SDL_PIXELFORMAT_RGBA32, SDL_TEXTUREACCESS_STREAMING, Width, Height);
        TextureWidthPhysical  = ViewportTexture ? Width : 0;
        TextureHeightPhysical = ViewportTexture ? Height : 0;
        if (!ViewportTexture) {
            return false;
        }
    }
    return SDL_UpdateTexture(ViewportTexture, nullptr, Pixels, Width * 4);
}

void LanternViewportWidget::DrawContent(Penumbra::Render::Renderer& Renderer, Penumbra::Rect ContentRect) {
    DpiScaleFactor           = Renderer.GetDpiScaleFactor();
    const auto WidthPhysical  = static_cast<std::uint32_t>(std::lround(ContentSizeLogical.X * DpiScaleFactor));
    const auto HeightPhysical = static_cast<std::uint32_t>(std::lround(ContentSizeLogical.Y * DpiScaleFactor));
    if (WidthPhysical == 0 || HeightPhysical == 0) {
        return;
    }
    if (!lantern_editor_render(Editor, WidthPhysical, HeightPhysical)) {
        return;
    }
    if (UploadPixels(Renderer.GetSdlRenderer())) {
        Renderer.DrawTexture(ViewportTexture, ContentRect);
    }
}

} // namespace WorldEditor
