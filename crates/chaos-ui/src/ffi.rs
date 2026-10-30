//! The bridge's C API (`native/include/lantern_chaos_ui.h`), by hand: the header is small and
//! its structs are plain data. Keep the two in step.

#![allow(non_camel_case_types)]

use std::os::raw::c_char;

use nyx_sys::{nyx_runtime, nyx_value};

#[repr(C)]
pub struct lantern_chaos_ui {
    _private: [u8; 0],
}

pub const LANTERN_CHAOS_UI_OK: i32 = 0;

pub const KIND_FRAME: i32 = 0;
pub const KIND_TEXT: i32 = 1;

pub const HAS_PRESS: u32 = 1 << 0;
pub const HAS_RELEASE: u32 = 1 << 1;

pub const EVENT_PRESS: i32 = 0;
pub const EVENT_RELEASE: i32 = 1;
pub const EVENT_FOCUS: i32 = 2;

pub const SET_BACKGROUND: u64 = 1 << 0;
pub const SET_GRADIENT: u64 = 1 << 1;
pub const SET_BORDER_COLOR: u64 = 1 << 2;
pub const SET_BORDER_WIDTH: u64 = 1 << 3;
pub const SET_BORDER_RADIUS: u64 = 1 << 4;
pub const SET_PADDING: u64 = 1 << 5;
pub const SET_MARGIN: u64 = 1 << 6;
pub const SET_TEXT_COLOR: u64 = 1 << 7;
pub const SET_FONT_SIZE: u64 = 1 << 8;
pub const SET_DISPLAY: u64 = 1 << 9;
pub const SET_DIRECTION: u64 = 1 << 10;
pub const SET_GAP: u64 = 1 << 11;
pub const SET_ALIGN_ITEMS: u64 = 1 << 12;
pub const SET_JUSTIFY_CONTENT: u64 = 1 << 13;
pub const SET_WIDTH: u64 = 1 << 14;
pub const SET_HEIGHT: u64 = 1 << 15;
pub const SET_MIN_WIDTH: u64 = 1 << 16;
pub const SET_MAX_WIDTH: u64 = 1 << 17;
pub const SET_FLEX_GROW: u64 = 1 << 18;
pub const SET_OPACITY: u64 = 1 << 19;
pub const SET_POSITION: u64 = 1 << 20;
pub const SET_TOP: u64 = 1 << 21;
pub const SET_RIGHT: u64 = 1 << 22;
pub const SET_BOTTOM: u64 = 1 << 23;
pub const SET_LEFT: u64 = 1 << 24;
pub const SET_Z_INDEX: u64 = 1 << 25;
pub const SET_SHADOW: u64 = 1 << 26;

pub const UNIT_PX: i32 = 0;
pub const UNIT_PERCENT: i32 = 1;
pub const UNIT_VW: i32 = 2;
pub const UNIT_VH: i32 = 3;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct lantern_chaos_ui_color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct lantern_chaos_ui_length {
    pub value: f32,
    pub unit: i32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct lantern_chaos_ui_style {
    pub set: u64,
    pub background: lantern_chaos_ui_color,
    pub gradient_top: lantern_chaos_ui_color,
    pub gradient_bottom: lantern_chaos_ui_color,
    pub border_color: lantern_chaos_ui_color,
    pub text_color: lantern_chaos_ui_color,
    pub shadow_color: lantern_chaos_ui_color,
    pub border_width: f32,
    pub border_radius: f32,
    pub font_size: f32,
    pub gap: f32,
    pub opacity: f32,
    pub flex_grow: f32,
    pub shadow_blur: f32,
    pub padding: [f32; 4],
    pub margin: [f32; 4],
    pub display: i32,
    pub direction: i32,
    pub align_items: i32,
    pub justify_content: i32,
    pub position: i32,
    pub z_index: i32,
    pub width: lantern_chaos_ui_length,
    pub height: lantern_chaos_ui_length,
    pub min_width: lantern_chaos_ui_length,
    pub max_width: lantern_chaos_ui_length,
    pub top: lantern_chaos_ui_length,
    pub right: lantern_chaos_ui_length,
    pub bottom: lantern_chaos_ui_length,
    pub left: lantern_chaos_ui_length,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct lantern_chaos_ui_node {
    pub id: u64,
    pub parent: i32,
    pub kind: i32,
    pub text: *const c_char,
    pub text_length: usize,
    pub events: u32,
    pub has_hover: i32,
    pub has_active: i32,
    pub style: lantern_chaos_ui_style,
    pub hover: lantern_chaos_ui_style,
    pub active: lantern_chaos_ui_style,
}

unsafe extern "C" {
    pub fn lantern_chaos_ui_create(
        runtime: *mut nyx_runtime,
        project_root: *const c_char,
        ui_directory: *const c_char,
        out_ui: *mut *mut lantern_chaos_ui,
    ) -> i32;
    pub fn lantern_chaos_ui_destroy(ui: *mut lantern_chaos_ui);
    pub fn lantern_chaos_ui_mount(
        ui: *mut lantern_chaos_ui,
        file: *const c_char,
        component: *const c_char,
        args: *const *mut nyx_value,
        count: usize,
        out_mount: *mut u32,
    ) -> i32;
    pub fn lantern_chaos_ui_unmount(ui: *mut lantern_chaos_ui, mount: u32) -> i32;
    pub fn lantern_chaos_ui_tick(ui: *mut lantern_chaos_ui);
    pub fn lantern_chaos_ui_snapshot(
        ui: *mut lantern_chaos_ui,
        out_nodes: *mut *const lantern_chaos_ui_node,
        out_count: *mut usize,
    ) -> i32;
    pub fn lantern_chaos_ui_dispatch(ui: *mut lantern_chaos_ui, node_id: u64, event: i32) -> i32;
    pub fn lantern_chaos_ui_diagnostic_count(ui: *const lantern_chaos_ui) -> usize;
    pub fn lantern_chaos_ui_diagnostic(
        ui: *const lantern_chaos_ui,
        index: usize,
        out_text: *mut *const c_char,
        out_length: *mut usize,
    ) -> i32;
}
