// Lantern's backend for Chaos and Cosmos. See include/lantern_chaos_ui.h for what it does and
// docs/decisions.md D-043 for why it is shaped this way.
//
// Chaos reconciles against Umbra::IWidget, so a node here is a plain IWidget that remembers
// what the component said about it (tag, class, text, handlers, number props). It is also a
// Cosmos::IStyleTarget, so Cosmos can resolve its style by walking its ancestors. Layout and
// drawing happen in Rust (crates/chaos-ui/src), from the snapshot this file builds.

#include "lantern_chaos_ui.h"

#include "Chaos/ChaosNyxDriver.h"
#include "Chaos/Component.h"
#include "Chaos/Diagnostics/RenderText.h"
#include "Chaos/SlotResolution.h"
#include "Chaos/SlotRuntime.h"
#include "Cosmos/Parser.h"
#include "Cosmos/ResolvedStyle.h"
#include "Cosmos/Resolver.h"
#include "Umbra/IWidget.h"
#include "nyx/nyx-host-interop.hpp"

#include <algorithm>
#include <cstdint>
#include <exception>
#include <filesystem>
#include <fstream>
#include <functional>
#include <map>
#include <memory>
#include <optional>
#include <set>
#include <sstream>
#include <string>
#include <unordered_map>
#include <utility>
#include <variant>
#include <vector>

namespace {

template <typename CosmosEnum> constexpr int Value(CosmosEnum Entry) { return static_cast<int>(Entry); }

static_assert(Value(Cosmos::LengthUnit::Px) == LANTERN_CHAOS_UI_PX);
static_assert(Value(Cosmos::LengthUnit::Percent) == LANTERN_CHAOS_UI_PERCENT);
static_assert(Value(Cosmos::LengthUnit::Vw) == LANTERN_CHAOS_UI_VW);
static_assert(Value(Cosmos::LengthUnit::Vh) == LANTERN_CHAOS_UI_VH);
static_assert(Value(Cosmos::Display::Stack) == LANTERN_CHAOS_UI_DISPLAY_STACK);
static_assert(Value(Cosmos::Display::Inline) == LANTERN_CHAOS_UI_DISPLAY_INLINE);
static_assert(Value(Cosmos::FlexDirection::Row) == LANTERN_CHAOS_UI_ROW);
static_assert(Value(Cosmos::FlexDirection::Column) == LANTERN_CHAOS_UI_COLUMN);
static_assert(Value(Cosmos::Align::Start) == LANTERN_CHAOS_UI_ALIGN_START);
static_assert(Value(Cosmos::Align::Center) == LANTERN_CHAOS_UI_ALIGN_CENTER);
static_assert(Value(Cosmos::Align::End) == LANTERN_CHAOS_UI_ALIGN_END);
static_assert(Value(Cosmos::Align::Stretch) == LANTERN_CHAOS_UI_ALIGN_STRETCH);
static_assert(Value(Cosmos::Justify::Start) == LANTERN_CHAOS_UI_JUSTIFY_START);
static_assert(Value(Cosmos::Justify::Center) == LANTERN_CHAOS_UI_JUSTIFY_CENTER);
static_assert(Value(Cosmos::Justify::End) == LANTERN_CHAOS_UI_JUSTIFY_END);
static_assert(Value(Cosmos::Justify::SpaceBetween) == LANTERN_CHAOS_UI_JUSTIFY_SPACE_BETWEEN);
static_assert(Value(Cosmos::Position::Static) == LANTERN_CHAOS_UI_STATIC);
static_assert(Value(Cosmos::Position::Relative) == LANTERN_CHAOS_UI_RELATIVE);
static_assert(Value(Cosmos::Position::Absolute) == LANTERN_CHAOS_UI_ABSOLUTE);
static_assert(Value(Cosmos::Position::Fixed) == LANTERN_CHAOS_UI_FIXED);
static_assert(Value(Cosmos::Position::Sticky) == LANTERN_CHAOS_UI_STICKY);

// The Nyx type a component's element tree has: `Name :: () -> lantern:::ui:::Element`.
constexpr const char* ElementTypeName = "lantern::ui::Element";

class UiNode;

// Every live node by id, so the engine can send an event to a node it saw in a snapshot.
class NodeRegistry {
public:
    std::uint64_t Add(UiNode* Node) {
        const std::uint64_t Id = NextId_++;
        Nodes_[Id] = Node;
        return Id;
    }
    void     Remove(std::uint64_t Id) { Nodes_.erase(Id); }
    UiNode*  Find(std::uint64_t Id) const {
        const auto Found = Nodes_.find(Id);
        return Found == Nodes_.end() ? nullptr : Found->second;
    }

private:
    std::uint64_t                               NextId_{1};
    std::unordered_map<std::uint64_t, UiNode*> Nodes_;
};

std::string CosmosTagFor(Chaos::ChaosElementTag Tag) {
    switch (Tag) {
        case Chaos::ChaosElementTag::Frame: return "Frame";
        case Chaos::ChaosElementTag::Inline: return "Inline";
        case Chaos::ChaosElementTag::Grid: return "Grid";
        case Chaos::ChaosElementTag::Image: return "Image";
        case Chaos::ChaosElementTag::Icon: return "Icon";
        case Chaos::ChaosElementTag::Text: return "Text";
        case Chaos::ChaosElementTag::Scroll: return "Scroll";
        case Chaos::ChaosElementTag::Input: return "Input";
        case Chaos::ChaosElementTag::TextArea: return "TextArea";
        default: return "";
    }
}

class UiNode final : public Umbra::IWidget, public Cosmos::IStyleTarget {
public:
    UiNode(NodeRegistry& Registry, Chaos::ChaosElementTag Tag, bool ComponentRoot)
        : Registry_(Registry), Id_(Registry.Add(this)), Tag_(Tag), ComponentRoot_(ComponentRoot) {}

    ~UiNode() override { Registry_.Remove(Id_); }

    UiNode(const UiNode&)            = delete;
    UiNode& operator=(const UiNode&) = delete;

    // -- Umbra::IWidget: what the reconciler changes --

    void ApplyPropDiff(const Umbra::ChaosPropDiff& Diff) override {
        if (Diff.ClassName) ClassName_ = *Diff.ClassName;
        if (Diff.Text) Text_ = *Diff.Text;
        if (Diff.OnPress) OnPress = *Diff.OnPress;
        if (Diff.OnRelease) OnRelease = *Diff.OnRelease;
        if (Diff.OnFocus) OnFocus = *Diff.OnFocus;
        if (Diff.OnHover) OnHover = *Diff.OnHover;
        if (Diff.OnHoverEnd) OnHoverEnd = *Diff.OnHoverEnd;
        for (const auto& [Name, Value] : Diff.Numbers) Numbers[Name] = Value;
    }

    std::size_t     GetChildCount() const override { return Children_.size(); }
    Umbra::IWidget* GetChildAt(std::size_t Index) const override {
        return Index < Children_.size() ? Children_[Index].get() : nullptr;
    }

    void InsertChildAt(std::size_t Index, std::unique_ptr<Umbra::IWidget> Child) override {
        if (!Child) return;
        if (auto* AsNode = dynamic_cast<UiNode*>(Child.get())) AsNode->Parent_ = this;
        Index = std::min(Index, Children_.size());
        Children_.insert(Children_.begin() + static_cast<std::ptrdiff_t>(Index), std::move(Child));
    }

    std::unique_ptr<Umbra::IWidget> RemoveChildAt(std::size_t Index) override {
        if (Index >= Children_.size()) return nullptr;
        std::unique_ptr<Umbra::IWidget> Removed = std::move(Children_[Index]);
        Children_.erase(Children_.begin() + static_cast<std::ptrdiff_t>(Index));
        if (auto* AsNode = dynamic_cast<UiNode*>(Removed.get())) AsNode->Parent_ = nullptr;
        return Removed;
    }

    // -- Cosmos::IStyleTarget: what the cascade reads --

    std::string                  ClassName() const override { return ClassName_; }
    std::string                  PrimitiveTag() const override { return CosmosTagFor(Tag_); }
    bool                         IsComponentRoot() const override { return ComponentRoot_; }
    const Cosmos::IStyleTarget*  Parent() const override { return Parent_; }

    // -- Read by the snapshot and by events --

    std::uint64_t            Id() const { return Id_; }
    Chaos::ChaosElementTag   Tag() const { return Tag_; }
    const std::string&       Text() const { return Text_; }
    void                     SetClassName(std::string Name) { ClassName_ = std::move(Name); }
    void                     SetText(std::string Text) { Text_ = std::move(Text); }

    std::function<void()> OnPress;
    std::function<void()> OnRelease;
    std::function<void()> OnFocus;
    std::function<void()> OnHover;
    std::function<void()> OnHoverEnd;
    std::unordered_map<std::string, float> Numbers;

private:
    NodeRegistry&                                Registry_;
    std::uint64_t                                Id_;
    Chaos::ChaosElementTag                       Tag_;
    bool                                         ComponentRoot_;
    UiNode*                                      Parent_{nullptr};
    std::string                                  ClassName_;
    std::string                                  Text_;
    std::vector<std::unique_ptr<Umbra::IWidget>> Children_;
};

template <typename T>
const T* PropAs(const Chaos::ChaosProps& Props, const char* Name) {
    const auto Found = Props.find(Name);
    return Found == Props.end() ? nullptr : std::get_if<T>(&Found->second);
}

// Builds a node tree for one Component. `<slot>`s and `None` give nothing here: Chaos's
// ResolveSlots mounts slot contents afterwards, through this same function.
std::unique_ptr<UiNode> BuildNode(NodeRegistry& Registry, const Chaos::Component& Node, bool IsMountRoot) {
    if (Node.Tag == Chaos::ChaosElementTag::None || Node.Tag == Chaos::ChaosElementTag::Slot) return nullptr;
    // A component's own root bounds its stylesheet's descendant selectors (Cosmos spec §1.2).
    const bool ComponentRoot = IsMountRoot || Node.Instance != nullptr;
    auto Built = std::make_unique<UiNode>(Registry, Node.Tag, ComponentRoot);
    if (const auto* Class = PropAs<std::string>(Node.Props, "class")) Built->SetClassName(*Class);
    if (const auto* Text = PropAs<std::string>(Node.Props, "text")) Built->SetText(*Text);
    if (const auto* Handler = PropAs<std::function<void()>>(Node.Props, "onPress")) Built->OnPress = *Handler;
    if (const auto* Handler = PropAs<std::function<void()>>(Node.Props, "onRelease")) Built->OnRelease = *Handler;
    if (const auto* Handler = PropAs<std::function<void()>>(Node.Props, "onFocus")) Built->OnFocus = *Handler;
    if (const auto* Handler = PropAs<std::function<void()>>(Node.Props, "onHover")) Built->OnHover = *Handler;
    if (const auto* Handler = PropAs<std::function<void()>>(Node.Props, "onHoverEnd")) Built->OnHoverEnd = *Handler;
    for (const auto& [Name, Value] : Node.Props) {
        if (const auto* AsFloat = std::get_if<float>(&Value)) Built->Numbers[Name] = *AsFloat;
        if (const auto* AsInt = std::get_if<int>(&Value)) Built->Numbers[Name] = static_cast<float>(*AsInt);
    }
    for (const Chaos::Component& Child : Node.Children) {
        if (std::unique_ptr<UiNode> BuiltChild = BuildNode(Registry, Child, false)) {
            Built->InsertChildAt(Built->GetChildCount(), std::move(BuiltChild));
        }
    }
    return Built;
}

// -- Cosmos style → lantern_chaos_ui_style --

lantern_chaos_ui_color ToColor(const Cosmos::Color& Color) { return {Color.R, Color.G, Color.B, Color.A}; }

lantern_chaos_ui_length ToLength(const Cosmos::Length& Length) {
    return {Length.Value, static_cast<int32_t>(Length.Unit)};
}

lantern_chaos_ui_length ToOffset(const Cosmos::Offset& Offset) {
    if (Offset.Auto) return {0.0F, LANTERN_CHAOS_UI_AUTO};
    return ToLength(Offset.Value);
}

void ToInsets(const Cosmos::EdgeInsets& Insets, float (&Out)[4]) {
    Out[0] = Insets.Left;
    Out[1] = Insets.Top;
    Out[2] = Insets.Right;
    Out[3] = Insets.Bottom;
}

// `attr(name)` single-number bindings (Cosmos spec §2): the property follows the element's own
// number prop, in logical pixels for a length.
void BindNumberAttributes(Cosmos::ResolvedStyle& Style, const std::unordered_map<std::string, float>& Numbers) {
    for (const Cosmos::AttrBinding& Binding : Style.AttrBindings) {
        const auto Found = Numbers.find(Binding.Attribute);
        if (Found == Numbers.end()) continue;
        const float           Value  = Found->second;
        const Cosmos::Length  Pixels{Value, Cosmos::LengthUnit::Px};
        const Cosmos::Offset  Offset{.Auto = false, .Value = Pixels};
        const std::string&    Property = Binding.Property;
        if (Property == "width") Style.Width = Pixels;
        else if (Property == "height") Style.Height = Pixels;
        else if (Property == "min-width") Style.MinWidth = Pixels;
        else if (Property == "max-width") Style.MaxWidth = Pixels;
        else if (Property == "flex-grow") Style.FlexGrow = std::max(0.0F, Value);
        else if (Property == "opacity") Style.Opacity = std::clamp(Value, 0.0F, 1.0F);
        else if (Property == "gap") Style.Gap = Value;
        else if (Property == "border-width") Style.BorderWidth = Value;
        else if (Property == "border-radius") Style.BorderRadius = Value;
        else if (Property == "top") Style.Top = Offset;
        else if (Property == "right") Style.Right = Offset;
        else if (Property == "bottom") Style.Bottom = Offset;
        else if (Property == "left") Style.Left = Offset;
    }
}

lantern_chaos_ui_style ExportStyle(const Cosmos::ResolvedStyle& Style) {
    lantern_chaos_ui_style Out{};
    auto Mark = [&Out](lantern_chaos_ui_style_bits Bit) { Out.set |= static_cast<std::uint64_t>(Bit); };
    if (Style.BackgroundColor) {
        Out.background = ToColor(*Style.BackgroundColor);
        Mark(LANTERN_CHAOS_UI_SET_BACKGROUND);
    }
    if (Style.BackgroundGradientStart && Style.BackgroundGradientEnd) {
        Out.gradient_top    = ToColor(*Style.BackgroundGradientStart);
        Out.gradient_bottom = ToColor(*Style.BackgroundGradientEnd);
        Mark(LANTERN_CHAOS_UI_SET_GRADIENT);
    }
    if (Style.BorderColor) {
        Out.border_color = ToColor(*Style.BorderColor);
        Mark(LANTERN_CHAOS_UI_SET_BORDER_COLOR);
    }
    if (Style.BorderWidth) {
        Out.border_width = *Style.BorderWidth;
        Mark(LANTERN_CHAOS_UI_SET_BORDER_WIDTH);
    }
    if (Style.BorderRadius) {
        Out.border_radius = *Style.BorderRadius;
        Mark(LANTERN_CHAOS_UI_SET_BORDER_RADIUS);
    }
    if (Style.Padding) {
        ToInsets(*Style.Padding, Out.padding);
        Mark(LANTERN_CHAOS_UI_SET_PADDING);
    }
    if (Style.Margin) {
        ToInsets(*Style.Margin, Out.margin);
        Mark(LANTERN_CHAOS_UI_SET_MARGIN);
    }
    if (Style.TextColor) {
        Out.text_color = ToColor(*Style.TextColor);
        Mark(LANTERN_CHAOS_UI_SET_TEXT_COLOR);
    }
    // `font-size` alone, or the size of a `font-family` + `font-size` request: Lantern draws
    // with its own bitmap font, so only the size is used.
    if (Style.FontSizeLogical) {
        Out.font_size = *Style.FontSizeLogical;
        Mark(LANTERN_CHAOS_UI_SET_FONT_SIZE);
    } else if (Style.Font) {
        Out.font_size = Style.Font->SizeLogical;
        Mark(LANTERN_CHAOS_UI_SET_FONT_SIZE);
    }
    if (Style.DisplayMode) {
        Out.display = static_cast<int32_t>(*Style.DisplayMode);
        Mark(LANTERN_CHAOS_UI_SET_DISPLAY);
    }
    if (Style.FlexDirectionMode) {
        Out.direction = static_cast<int32_t>(*Style.FlexDirectionMode);
        Mark(LANTERN_CHAOS_UI_SET_DIRECTION);
    }
    if (Style.Gap) {
        Out.gap = *Style.Gap;
        Mark(LANTERN_CHAOS_UI_SET_GAP);
    }
    if (Style.AlignItems) {
        Out.align_items = static_cast<int32_t>(*Style.AlignItems);
        Mark(LANTERN_CHAOS_UI_SET_ALIGN_ITEMS);
    }
    if (Style.JustifyContent) {
        Out.justify_content = static_cast<int32_t>(*Style.JustifyContent);
        Mark(LANTERN_CHAOS_UI_SET_JUSTIFY_CONTENT);
    }
    if (Style.Width) {
        Out.width = ToLength(*Style.Width);
        Mark(LANTERN_CHAOS_UI_SET_WIDTH);
    }
    if (Style.Height) {
        Out.height = ToLength(*Style.Height);
        Mark(LANTERN_CHAOS_UI_SET_HEIGHT);
    }
    if (Style.MinWidth) {
        Out.min_width = ToLength(*Style.MinWidth);
        Mark(LANTERN_CHAOS_UI_SET_MIN_WIDTH);
    }
    if (Style.MaxWidth) {
        Out.max_width = ToLength(*Style.MaxWidth);
        Mark(LANTERN_CHAOS_UI_SET_MAX_WIDTH);
    }
    if (Style.FlexGrow) {
        Out.flex_grow = *Style.FlexGrow;
        Mark(LANTERN_CHAOS_UI_SET_FLEX_GROW);
    }
    if (Style.Opacity) {
        Out.opacity = *Style.Opacity;
        Mark(LANTERN_CHAOS_UI_SET_OPACITY);
    }
    if (Style.PositionMode) {
        Out.position = static_cast<int32_t>(*Style.PositionMode);
        Mark(LANTERN_CHAOS_UI_SET_POSITION);
    }
    if (Style.Top) {
        Out.top = ToOffset(*Style.Top);
        Mark(LANTERN_CHAOS_UI_SET_TOP);
    }
    if (Style.Right) {
        Out.right = ToOffset(*Style.Right);
        Mark(LANTERN_CHAOS_UI_SET_RIGHT);
    }
    if (Style.Bottom) {
        Out.bottom = ToOffset(*Style.Bottom);
        Mark(LANTERN_CHAOS_UI_SET_BOTTOM);
    }
    if (Style.Left) {
        Out.left = ToOffset(*Style.Left);
        Mark(LANTERN_CHAOS_UI_SET_LEFT);
    }
    if (Style.ZIndex && !Style.ZIndex->Auto) {
        Out.z_index = Style.ZIndex->Level;
        Mark(LANTERN_CHAOS_UI_SET_Z_INDEX);
    }
    if (Style.ShadowColor) {
        Out.shadow_color = ToColor(*Style.ShadowColor);
        Out.shadow_blur  = Style.ShadowBlurRadiusLogical.value_or(0.0F);
        Mark(LANTERN_CHAOS_UI_SET_SHADOW);
    }
    return Out;
}

// `Over`'s set fields on top of `Base`: a `:hover` or `:active` block over the element's own
// style.
lantern_chaos_ui_style MergeStyle(lantern_chaos_ui_style Base, const lantern_chaos_ui_style& Over) {
    auto Has = [&Over](lantern_chaos_ui_style_bits Bit) { return (Over.set & static_cast<std::uint64_t>(Bit)) != 0; };
    if (Has(LANTERN_CHAOS_UI_SET_BACKGROUND)) Base.background = Over.background;
    if (Has(LANTERN_CHAOS_UI_SET_GRADIENT)) {
        Base.gradient_top    = Over.gradient_top;
        Base.gradient_bottom = Over.gradient_bottom;
    }
    if (Has(LANTERN_CHAOS_UI_SET_BORDER_COLOR)) Base.border_color = Over.border_color;
    if (Has(LANTERN_CHAOS_UI_SET_BORDER_WIDTH)) Base.border_width = Over.border_width;
    if (Has(LANTERN_CHAOS_UI_SET_BORDER_RADIUS)) Base.border_radius = Over.border_radius;
    if (Has(LANTERN_CHAOS_UI_SET_PADDING)) std::copy(std::begin(Over.padding), std::end(Over.padding), Base.padding);
    if (Has(LANTERN_CHAOS_UI_SET_MARGIN)) std::copy(std::begin(Over.margin), std::end(Over.margin), Base.margin);
    if (Has(LANTERN_CHAOS_UI_SET_TEXT_COLOR)) Base.text_color = Over.text_color;
    if (Has(LANTERN_CHAOS_UI_SET_FONT_SIZE)) Base.font_size = Over.font_size;
    if (Has(LANTERN_CHAOS_UI_SET_DISPLAY)) Base.display = Over.display;
    if (Has(LANTERN_CHAOS_UI_SET_DIRECTION)) Base.direction = Over.direction;
    if (Has(LANTERN_CHAOS_UI_SET_GAP)) Base.gap = Over.gap;
    if (Has(LANTERN_CHAOS_UI_SET_ALIGN_ITEMS)) Base.align_items = Over.align_items;
    if (Has(LANTERN_CHAOS_UI_SET_JUSTIFY_CONTENT)) Base.justify_content = Over.justify_content;
    if (Has(LANTERN_CHAOS_UI_SET_WIDTH)) Base.width = Over.width;
    if (Has(LANTERN_CHAOS_UI_SET_HEIGHT)) Base.height = Over.height;
    if (Has(LANTERN_CHAOS_UI_SET_MIN_WIDTH)) Base.min_width = Over.min_width;
    if (Has(LANTERN_CHAOS_UI_SET_MAX_WIDTH)) Base.max_width = Over.max_width;
    if (Has(LANTERN_CHAOS_UI_SET_FLEX_GROW)) Base.flex_grow = Over.flex_grow;
    if (Has(LANTERN_CHAOS_UI_SET_OPACITY)) Base.opacity = Over.opacity;
    if (Has(LANTERN_CHAOS_UI_SET_POSITION)) Base.position = Over.position;
    if (Has(LANTERN_CHAOS_UI_SET_TOP)) Base.top = Over.top;
    if (Has(LANTERN_CHAOS_UI_SET_RIGHT)) Base.right = Over.right;
    if (Has(LANTERN_CHAOS_UI_SET_BOTTOM)) Base.bottom = Over.bottom;
    if (Has(LANTERN_CHAOS_UI_SET_LEFT)) Base.left = Over.left;
    if (Has(LANTERN_CHAOS_UI_SET_Z_INDEX)) Base.z_index = Over.z_index;
    if (Has(LANTERN_CHAOS_UI_SET_SHADOW)) {
        Base.shadow_color = Over.shadow_color;
        Base.shadow_blur  = Over.shadow_blur;
    }
    Base.set |= Over.set;
    return Base;
}

std::optional<std::string> ReadFile(const std::filesystem::path& Path) {
    std::ifstream File(Path);
    if (!File) return std::nullopt;
    std::ostringstream Buffer;
    Buffer << File.rdbuf();
    return Buffer.str();
}

struct MountedTree {
    // Destroyed bottom to top: slots first, then the nodes, then the component tree whose
    // closures the nodes' handlers point into.
    std::shared_ptr<Chaos::Component>              Root;
    std::unique_ptr<Umbra::IWidget>                 Tree;
    std::vector<std::unique_ptr<chaos::SlotState>>  Slots;
};

} // namespace

struct lantern_chaos_ui {
    lantern_chaos_ui(nyx::host::NyxRuntime& Runtime, std::string ProjectRoot, Chaos::ChaosConfig Config)
        : Driver(std::move(Config), std::move(ProjectRoot), Runtime) {}

    // Declared first so it is destroyed last: every node removes itself from it.
    NodeRegistry Registry;

    // The C handle of the runtime the driver runs on, to check mount arguments against.
    nyx_runtime* RuntimeHandle = nullptr;

    Chaos::ChaosNyxDriver Driver;
    Cosmos::Stylesheet    GlobalSheet;
    Cosmos::Stylesheet    ComponentSheet;
    Cosmos::StylesheetSet Sheets{&GlobalSheet, &ComponentSheet};

    std::map<std::uint32_t, MountedTree> Mounts;
    std::uint32_t                        NextMount{1};

    std::vector<lantern_chaos_ui_node> Snapshot;

    std::vector<std::string> Messages;
    std::set<std::string>    ReportedStyleMessages;
    std::size_t              ReportedDriverDiagnostics{0};

    chaos::MountFn MountFunction() {
        return [this](const Chaos::Component& Node) -> std::unique_ptr<Umbra::IWidget> {
            return BuildNode(Registry, Node, true);
        };
    }

    // Moves new Chaos diagnostics into Messages; true when there were any.
    bool CollectDriverDiagnostics() {
        const std::vector<Chaos::Diagnostic>& All = Driver.Diagnostics();
        const bool                             Any = All.size() > ReportedDriverDiagnostics;
        for (; ReportedDriverDiagnostics < All.size(); ++ReportedDriverDiagnostics) {
            Messages.push_back(Chaos::RenderText(All[ReportedDriverDiagnostics], Driver.Sources()));
        }
        return Any;
    }

    void ReportStyleMessage(std::string Message) {
        if (ReportedStyleMessages.insert(Message).second) Messages.push_back(std::move(Message));
    }

    void LoadStylesheets(const std::filesystem::path& Directory) {
        std::error_code                     Ignored;
        std::vector<std::filesystem::path> Files;
        for (const auto& Entry : std::filesystem::directory_iterator(Directory, Ignored)) {
            if (Entry.path().extension() == ".cosmos") Files.push_back(Entry.path());
        }
        // Sorted, so rules from different files cascade in the same order on every machine.
        std::sort(Files.begin(), Files.end());
        for (const std::filesystem::path& File : Files) {
            const std::optional<std::string> Source = ReadFile(File);
            if (!Source) {
                ReportStyleMessage("cosmos: could not read " + File.string());
                continue;
            }
            Cosmos::Parser      Parser(*Source, File.string());
            Cosmos::ParseResult Parsed = Parser.Parse();
            for (const Cosmos::ParseError& Error : Parsed.Errors) {
                ReportStyleMessage("cosmos: " + File.string() + ": " + Error.Message);
            }
            if (!Parsed.Sheet) continue;
            Cosmos::Stylesheet& Target = File.filename() == "global.cosmos" ? GlobalSheet : ComponentSheet;
            if (Parsed.Sheet->Root) {
                if (!Target.Root) Target.Root.emplace();
                for (Cosmos::VariableDeclaration& Variable : Parsed.Sheet->Root->Variables) {
                    Target.Root->Variables.push_back(std::move(Variable));
                }
            }
            for (Cosmos::RulePtr& Rule : Parsed.Sheet->Rules) Target.Rules.push_back(std::move(Rule));
        }
    }

    void AppendSnapshot(const Umbra::IWidget& Widget, int32_t ParentIndex) {
        const auto* Node = dynamic_cast<const UiNode*>(&Widget);
        if (Node == nullptr) return;
        std::vector<Cosmos::ResolveDiagnostic> StyleDiagnostics;
        Cosmos::ResolvedStyle                  Resolved = Cosmos::ResolveStyle(*Node, Sheets, StyleDiagnostics);
        for (Cosmos::ResolveDiagnostic& Diagnostic : StyleDiagnostics) {
            ReportStyleMessage("cosmos: " + Diagnostic.Message);
        }
        BindNumberAttributes(Resolved, Node->Numbers);

        lantern_chaos_ui_node Out{};
        Out.id     = Node->Id();
        Out.parent = ParentIndex;
        switch (Node->Tag()) {
            case Chaos::ChaosElementTag::Frame: Out.kind = LANTERN_CHAOS_UI_FRAME; break;
            case Chaos::ChaosElementTag::Text: Out.kind = LANTERN_CHAOS_UI_TEXT; break;
            default: Out.kind = LANTERN_CHAOS_UI_UNSUPPORTED; break;
        }
        if (Out.kind == LANTERN_CHAOS_UI_TEXT) {
            Out.text        = Node->Text().data();
            Out.text_length = Node->Text().size();
        }
        if (Node->OnPress) Out.events |= LANTERN_CHAOS_UI_HAS_PRESS;
        if (Node->OnRelease) Out.events |= LANTERN_CHAOS_UI_HAS_RELEASE;
        if (Node->OnFocus) Out.events |= LANTERN_CHAOS_UI_HAS_FOCUS;
        if (Node->OnHover) Out.events |= LANTERN_CHAOS_UI_HAS_HOVER;
        if (Node->OnHoverEnd) Out.events |= LANTERN_CHAOS_UI_HAS_HOVER_END;
        Out.style = ExportStyle(Resolved);
        if (Resolved.Hover) {
            Cosmos::ResolvedStyle Hover = *Resolved.Hover;
            BindNumberAttributes(Hover, Node->Numbers);
            Out.has_hover = 1;
            Out.hover     = MergeStyle(Out.style, ExportStyle(Hover));
        }
        if (Resolved.Active) {
            Cosmos::ResolvedStyle Active = *Resolved.Active;
            BindNumberAttributes(Active, Node->Numbers);
            Out.has_active = 1;
            Out.active     = MergeStyle(Out.has_hover ? Out.hover : Out.style, ExportStyle(Active));
        }

        const auto ThisIndex = static_cast<int32_t>(Snapshot.size());
        Snapshot.push_back(Out);
        for (std::size_t Index = 0; Index < Node->GetChildCount(); ++Index) {
            if (const Umbra::IWidget* Child = Node->GetChildAt(Index)) AppendSnapshot(*Child, ThisIndex);
        }
    }
};

extern "C" {

lantern_chaos_ui_status lantern_chaos_ui_create(nyx_runtime* runtime, const char* project_root,
                                                const char* ui_directory, lantern_chaos_ui** out_ui) {
    if (out_ui == nullptr) return LANTERN_CHAOS_UI_ERROR;
    *out_ui = nullptr;
    nyx::host::NyxRuntime* Runtime = nyx::capi::HostRuntime(runtime);
    if (Runtime == nullptr || project_root == nullptr || ui_directory == nullptr) return LANTERN_CHAOS_UI_ERROR;
    try {
        Chaos::ChaosConfig Config;
        Config.Target = Chaos::ChaosBuildTarget::UmbraEngine;
        // Component imports (`MainMenu :: import MainMenu;`) are looked up under the search
        // paths, relative to the project root: the whole game directory, as for Nyx imports.
        Config.SearchPaths = {"."};
        auto Ui       = std::make_unique<lantern_chaos_ui>(*Runtime, project_root, std::move(Config));
        Ui->RuntimeHandle = runtime;
        Ui->Driver.RegisterElementType(ElementTypeName);
        Ui->LoadStylesheets(ui_directory);
        *out_ui = Ui.release();
        return LANTERN_CHAOS_UI_OK;
    } catch (...) {
        return LANTERN_CHAOS_UI_ERROR;
    }
}

void lantern_chaos_ui_destroy(lantern_chaos_ui* ui) { delete ui; }

lantern_chaos_ui_status lantern_chaos_ui_mount(lantern_chaos_ui* ui, const char* file, const char* component,
                                               nyx_value* const* args, size_t count, uint32_t* out_mount) {
    if (ui == nullptr || file == nullptr || component == nullptr || out_mount == nullptr) return LANTERN_CHAOS_UI_ERROR;
    *out_mount = 0;
    try {
        std::vector<nyx::runtime::Value> Arguments;
        for (size_t Index = 0; Index < count; ++Index) {
            const nyx::runtime::Value* Argument = nyx::capi::HostValue(ui->RuntimeHandle, args[Index]);
            if (Argument == nullptr) {
                ui->Messages.push_back(std::string("chaos-ui: argument ") + std::to_string(Index + 1) + " for " +
                                       component + " is missing or belongs to another Nyx runtime");
                return LANTERN_CHAOS_UI_ERROR;
            }
            Arguments.push_back(*Argument);
        }
        std::filesystem::path Path(file);
        if (Path.is_relative()) Path = std::filesystem::path(ui->Driver.ProjectRoot()) / Path;

        MountedTree Mounted;
        Mounted.Root = std::make_shared<Chaos::Component>(
            ui->Driver.MountRoot(Path.lexically_normal().string(), component, std::move(Arguments)));
        if (ui->CollectDriverDiagnostics() || Mounted.Root->Tag == Chaos::ChaosElementTag::None) {
            ui->Messages.push_back(std::string("chaos-ui: mounting ") + component + " from " + file + " failed");
            return LANTERN_CHAOS_UI_ERROR;
        }
        Mounted.Tree = BuildNode(ui->Registry, *Mounted.Root, true);
        if (!Mounted.Tree) {
            ui->Messages.push_back(std::string("chaos-ui: ") + component + " from " + file +
                                   " has no element at its root to draw (a root must be an element, not a bare <slot>)");
            return LANTERN_CHAOS_UI_ERROR;
        }
        Mounted.Slots = chaos::ResolveSlots(*Mounted.Tree, *Mounted.Root, ui->MountFunction());
        for (std::unique_ptr<chaos::SlotState>& Slot : Mounted.Slots) Slot->Reconcile();
        ui->CollectDriverDiagnostics();

        const std::uint32_t Number = ui->NextMount++;
        ui->Mounts.emplace(Number, std::move(Mounted));
        *out_mount = Number;
        return LANTERN_CHAOS_UI_OK;
    } catch (const std::exception& Error) {
        ui->Messages.push_back(std::string("chaos-ui: mounting ") + component + " failed: " + Error.what());
        return LANTERN_CHAOS_UI_ERROR;
    }
}

lantern_chaos_ui_status lantern_chaos_ui_unmount(lantern_chaos_ui* ui, uint32_t mount) {
    if (ui == nullptr) return LANTERN_CHAOS_UI_ERROR;
    return ui->Mounts.erase(mount) == 1 ? LANTERN_CHAOS_UI_OK : LANTERN_CHAOS_UI_ERROR;
}

void lantern_chaos_ui_tick(lantern_chaos_ui* ui) {
    if (ui == nullptr) return;
    try {
        chaos::Tick();
    } catch (const std::exception& Error) {
        ui->Messages.push_back(std::string("chaos-ui: re-rendering failed: ") + Error.what());
    }
    ui->CollectDriverDiagnostics();
}

lantern_chaos_ui_status lantern_chaos_ui_snapshot(lantern_chaos_ui* ui, const lantern_chaos_ui_node** out_nodes,
                                                  size_t* out_count) {
    if (ui == nullptr || out_nodes == nullptr || out_count == nullptr) return LANTERN_CHAOS_UI_ERROR;
    ui->Snapshot.clear();
    try {
        for (const auto& [Number, Mounted] : ui->Mounts) {
            if (Mounted.Tree) ui->AppendSnapshot(*Mounted.Tree, -1);
        }
    } catch (const std::exception& Error) {
        ui->Messages.push_back(std::string("chaos-ui: resolving styles failed: ") + Error.what());
    }
    *out_nodes = ui->Snapshot.data();
    *out_count = ui->Snapshot.size();
    return LANTERN_CHAOS_UI_OK;
}

lantern_chaos_ui_status lantern_chaos_ui_dispatch(lantern_chaos_ui* ui, uint64_t node_id, int32_t event) {
    if (ui == nullptr) return LANTERN_CHAOS_UI_ERROR;
    UiNode* Node = ui->Registry.Find(node_id);
    if (Node == nullptr) return LANTERN_CHAOS_UI_ERROR;
    // Copied: the handler may change what the node holds, or unmount the node.
    std::function<void()> Handler;
    switch (event) {
        case LANTERN_CHAOS_UI_PRESS: Handler = Node->OnPress; break;
        case LANTERN_CHAOS_UI_RELEASE: Handler = Node->OnRelease; break;
        case LANTERN_CHAOS_UI_FOCUS: Handler = Node->OnFocus; break;
        case LANTERN_CHAOS_UI_HOVER: Handler = Node->OnHover; break;
        case LANTERN_CHAOS_UI_HOVER_END: Handler = Node->OnHoverEnd; break;
        default: return LANTERN_CHAOS_UI_ERROR;
    }
    if (!Handler) return LANTERN_CHAOS_UI_OK;
    try {
        // Signal writes in the handler re-render once, at the end of the batch.
        chaos::ScopedEventBatch Batch;
        Handler();
    } catch (const std::exception& Error) {
        ui->Messages.push_back(std::string("chaos-ui: an event handler failed: ") + Error.what());
    }
    ui->CollectDriverDiagnostics();
    return LANTERN_CHAOS_UI_OK;
}

size_t lantern_chaos_ui_diagnostic_count(const lantern_chaos_ui* ui) { return ui == nullptr ? 0 : ui->Messages.size(); }

lantern_chaos_ui_status lantern_chaos_ui_diagnostic(const lantern_chaos_ui* ui, size_t index, const char** out_text,
                                                    size_t* out_length) {
    if (ui == nullptr || out_text == nullptr || out_length == nullptr || index >= ui->Messages.size()) {
        return LANTERN_CHAOS_UI_ERROR;
    }
    *out_text   = ui->Messages[index].data();
    *out_length = ui->Messages[index].size();
    return LANTERN_CHAOS_UI_OK;
}

} // extern "C"
