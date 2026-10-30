#ifndef LANTERN_CHAOS_UI_H
#define LANTERN_CHAOS_UI_H

/* Lantern's backend for Chaos components and Cosmos stylesheets.
 *
 * The bridge runs Chaos on the game's own Nyx runtime, keeps the mounted components as plain
 * nodes that implement Umbra::IWidget (the contract Chaos reconciles against), and resolves each
 * node's Cosmos style. It does no layout and no drawing: the engine reads a snapshot of the nodes
 * with their resolved styles (lantern_chaos_ui_snapshot), lays them out and draws them, and sends
 * events back by node id (lantern_chaos_ui_dispatch).
 *
 * Everything here runs on the thread that owns the Nyx runtime. */

#include <stddef.h>
#include <stdint.h>

#include "nyx/nyx.h"

#ifdef __cplusplus
extern "C" {
#endif

typedef struct lantern_chaos_ui lantern_chaos_ui;

typedef enum lantern_chaos_ui_status {
    LANTERN_CHAOS_UI_OK = 0,
    /* Bad arguments, an unknown mount or node, or a mount that reported diagnostics. The
     * reason is in the diagnostics (lantern_chaos_ui_diagnostic). */
    LANTERN_CHAOS_UI_ERROR = 1
} lantern_chaos_ui_status;

typedef enum lantern_chaos_ui_kind {
    LANTERN_CHAOS_UI_FRAME = 0, /* <frame>: a box that stacks its children */
    LANTERN_CHAOS_UI_TEXT = 1,  /* <text>: one run of text, no children */
    /* Any other Core primitive. Lantern doesn't draw these yet; they lay out as empty frames
     * so the rest of the tree still works. */
    LANTERN_CHAOS_UI_UNSUPPORTED = 2
} lantern_chaos_ui_kind;

/* Which event props a node has, as bits of lantern_chaos_ui_node.events. */
typedef enum lantern_chaos_ui_event_bits {
    LANTERN_CHAOS_UI_HAS_PRESS = 1u << 0,
    LANTERN_CHAOS_UI_HAS_RELEASE = 1u << 1,
    LANTERN_CHAOS_UI_HAS_FOCUS = 1u << 2,
    LANTERN_CHAOS_UI_HAS_HOVER = 1u << 3,
    LANTERN_CHAOS_UI_HAS_HOVER_END = 1u << 4
} lantern_chaos_ui_event_bits;

typedef enum lantern_chaos_ui_event {
    LANTERN_CHAOS_UI_PRESS = 0,     /* onPress */
    LANTERN_CHAOS_UI_RELEASE = 1,   /* onRelease: a button was chosen */
    LANTERN_CHAOS_UI_FOCUS = 2,     /* onFocus: the node became the focused one */
    LANTERN_CHAOS_UI_HOVER = 3,     /* onHover */
    LANTERN_CHAOS_UI_HOVER_END = 4  /* onHoverEnd */
} lantern_chaos_ui_event;

typedef struct lantern_chaos_ui_color {
    uint8_t r, g, b, a;
} lantern_chaos_ui_color;

typedef enum lantern_chaos_ui_unit {
    LANTERN_CHAOS_UI_PX = 0,      /* logical pixels */
    LANTERN_CHAOS_UI_PERCENT = 1, /* of the containing box */
    LANTERN_CHAOS_UI_VW = 2,      /* of the window's width */
    LANTERN_CHAOS_UI_VH = 3,      /* of the window's height */
    LANTERN_CHAOS_UI_AUTO = 4     /* an offset written as `auto` */
} lantern_chaos_ui_unit;

typedef struct lantern_chaos_ui_length {
    float value;
    int32_t unit; /* lantern_chaos_ui_unit */
} lantern_chaos_ui_length;

/* Which fields of lantern_chaos_ui_style the stylesheets set, as bits of `set`. A field whose
 * bit is clear holds zero and means "not set". */
typedef enum lantern_chaos_ui_style_bits {
    LANTERN_CHAOS_UI_SET_BACKGROUND = 1ull << 0,
    LANTERN_CHAOS_UI_SET_GRADIENT = 1ull << 1, /* gradient_top and gradient_bottom */
    LANTERN_CHAOS_UI_SET_BORDER_COLOR = 1ull << 2,
    LANTERN_CHAOS_UI_SET_BORDER_WIDTH = 1ull << 3,
    LANTERN_CHAOS_UI_SET_BORDER_RADIUS = 1ull << 4,
    LANTERN_CHAOS_UI_SET_PADDING = 1ull << 5,
    LANTERN_CHAOS_UI_SET_MARGIN = 1ull << 6,
    LANTERN_CHAOS_UI_SET_TEXT_COLOR = 1ull << 7,
    LANTERN_CHAOS_UI_SET_FONT_SIZE = 1ull << 8,
    LANTERN_CHAOS_UI_SET_DISPLAY = 1ull << 9,
    LANTERN_CHAOS_UI_SET_DIRECTION = 1ull << 10,
    LANTERN_CHAOS_UI_SET_GAP = 1ull << 11,
    LANTERN_CHAOS_UI_SET_ALIGN_ITEMS = 1ull << 12,
    LANTERN_CHAOS_UI_SET_JUSTIFY_CONTENT = 1ull << 13,
    LANTERN_CHAOS_UI_SET_WIDTH = 1ull << 14,
    LANTERN_CHAOS_UI_SET_HEIGHT = 1ull << 15,
    LANTERN_CHAOS_UI_SET_MIN_WIDTH = 1ull << 16,
    LANTERN_CHAOS_UI_SET_MAX_WIDTH = 1ull << 17,
    LANTERN_CHAOS_UI_SET_FLEX_GROW = 1ull << 18,
    LANTERN_CHAOS_UI_SET_OPACITY = 1ull << 19,
    LANTERN_CHAOS_UI_SET_POSITION = 1ull << 20,
    LANTERN_CHAOS_UI_SET_TOP = 1ull << 21,
    LANTERN_CHAOS_UI_SET_RIGHT = 1ull << 22,
    LANTERN_CHAOS_UI_SET_BOTTOM = 1ull << 23,
    LANTERN_CHAOS_UI_SET_LEFT = 1ull << 24,
    LANTERN_CHAOS_UI_SET_Z_INDEX = 1ull << 25,
    LANTERN_CHAOS_UI_SET_SHADOW = 1ull << 26 /* shadow_color and shadow_blur (box-shadow) */
} lantern_chaos_ui_style_bits;

/* The values below are Cosmos's own enums, in Cosmos's order. */
typedef enum lantern_chaos_ui_display { LANTERN_CHAOS_UI_DISPLAY_STACK = 0, LANTERN_CHAOS_UI_DISPLAY_INLINE = 1 } lantern_chaos_ui_display;
typedef enum lantern_chaos_ui_direction { LANTERN_CHAOS_UI_ROW = 0, LANTERN_CHAOS_UI_COLUMN = 1 } lantern_chaos_ui_direction;
typedef enum lantern_chaos_ui_align {
    LANTERN_CHAOS_UI_ALIGN_START = 0,
    LANTERN_CHAOS_UI_ALIGN_CENTER = 1,
    LANTERN_CHAOS_UI_ALIGN_END = 2,
    LANTERN_CHAOS_UI_ALIGN_STRETCH = 3
} lantern_chaos_ui_align;
typedef enum lantern_chaos_ui_justify {
    LANTERN_CHAOS_UI_JUSTIFY_START = 0,
    LANTERN_CHAOS_UI_JUSTIFY_CENTER = 1,
    LANTERN_CHAOS_UI_JUSTIFY_END = 2,
    LANTERN_CHAOS_UI_JUSTIFY_SPACE_BETWEEN = 3
} lantern_chaos_ui_justify;
typedef enum lantern_chaos_ui_position {
    LANTERN_CHAOS_UI_STATIC = 0,
    LANTERN_CHAOS_UI_RELATIVE = 1,
    LANTERN_CHAOS_UI_ABSOLUTE = 2,
    LANTERN_CHAOS_UI_FIXED = 3,
    LANTERN_CHAOS_UI_STICKY = 4
} lantern_chaos_ui_position;

/* One element's resolved Cosmos style. Lengths are logical pixels unless a length says
 * otherwise; padding and margin are left, top, right, bottom. */
typedef struct lantern_chaos_ui_style {
    uint64_t set;
    lantern_chaos_ui_color background;
    lantern_chaos_ui_color gradient_top;
    lantern_chaos_ui_color gradient_bottom;
    lantern_chaos_ui_color border_color;
    lantern_chaos_ui_color text_color;
    lantern_chaos_ui_color shadow_color;
    float border_width;
    float border_radius;
    float font_size;
    float gap;
    float opacity;
    float flex_grow;
    float shadow_blur;
    float padding[4];
    float margin[4];
    int32_t display;
    int32_t direction;
    int32_t align_items;
    int32_t justify_content;
    int32_t position;
    int32_t z_index;
    lantern_chaos_ui_length width;
    lantern_chaos_ui_length height;
    lantern_chaos_ui_length min_width;
    lantern_chaos_ui_length max_width;
    lantern_chaos_ui_length top;
    lantern_chaos_ui_length right;
    lantern_chaos_ui_length bottom;
    lantern_chaos_ui_length left;
} lantern_chaos_ui_style;

typedef struct lantern_chaos_ui_node {
    /* Stable while the node lives: the reconciler updates nodes in place. */
    uint64_t id;
    /* The parent's index in the same snapshot, or -1 for a mount's root. */
    int32_t parent;
    int32_t kind; /* lantern_chaos_ui_kind */
    /* <text>'s text, UTF-8, not NUL-terminated. Empty for other kinds. */
    const char* text;
    size_t text_length;
    uint32_t events; /* lantern_chaos_ui_event_bits */
    /* `:hover` and `:active` give full styles (the base with the pseudo-class block on top)
     * when the stylesheets have such a block for this node, flagged here. */
    int32_t has_hover;
    int32_t has_active;
    lantern_chaos_ui_style style;
    lantern_chaos_ui_style hover;
    lantern_chaos_ui_style active;
} lantern_chaos_ui_node;

/* Starts Chaos on `runtime`: registers the `Component` base class, the `@signal` decorator,
 * the `.chaos` file kind and the element type `Ui::Element`, and loads every `.cosmos`
 * file directly in `ui_directory` (`global.cosmos` as the global sheet, the rest as component
 * sheets). `project_root` is where `.chaos` imports resolve from.
 *
 * Call it after registering the engine API and before mounting the game, so the game's own
 * classes can use `@signal`. Destroy it before the runtime. */
lantern_chaos_ui_status lantern_chaos_ui_create(nyx_runtime* runtime, const char* project_root,
                                                const char* ui_directory, lantern_chaos_ui** out_ui);
void lantern_chaos_ui_destroy(lantern_chaos_ui* ui);

/* Mounts `component` from `file` (a path under project_root, or absolute) with `args` as its
 * arguments, and gives back a mount number for lantern_chaos_ui_unmount. */
lantern_chaos_ui_status lantern_chaos_ui_mount(lantern_chaos_ui* ui, const char* file, const char* component,
                                               nyx_value* const* args, size_t count, uint32_t* out_mount);
lantern_chaos_ui_status lantern_chaos_ui_unmount(lantern_chaos_ui* ui, uint32_t mount);

/* Re-renders every slot whose signals changed since the last tick. */
void lantern_chaos_ui_tick(lantern_chaos_ui* ui);

/* Every mounted node, mount by mount in mount order, each tree in depth-first order (a parent
 * before its children), with styles resolved now. The array stays valid until the next call
 * on `ui`. */
lantern_chaos_ui_status lantern_chaos_ui_snapshot(lantern_chaos_ui* ui, const lantern_chaos_ui_node** out_nodes,
                                                  size_t* out_count);

/* Calls the node's handler for `event` (a lantern_chaos_ui_event). Nothing happens when the
 * node has no such handler. */
lantern_chaos_ui_status lantern_chaos_ui_dispatch(lantern_chaos_ui* ui, uint64_t node_id, int32_t event);

/* Everything Chaos and Cosmos reported so far (compile errors, script errors in handlers,
 * stylesheet errors), oldest first, rendered for a log. */
size_t lantern_chaos_ui_diagnostic_count(const lantern_chaos_ui* ui);
lantern_chaos_ui_status lantern_chaos_ui_diagnostic(const lantern_chaos_ui* ui, size_t index, const char** out_text,
                                                    size_t* out_length);

#ifdef __cplusplus
}
#endif

#endif
