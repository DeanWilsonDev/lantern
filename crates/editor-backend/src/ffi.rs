//! The C API: `include/lantern_editor.h` declares exactly these functions.
//!
//! Rules every function follows:
//!
//! - The editor handle comes from `lantern_editor_create` and is used from one thread (the
//!   UI thread). A null handle is ignored and returns the type's default.
//! - Strings passed in are NUL-terminated UTF-8, borrowed for the call only.
//! - Strings returned are owned by the editor and stay valid until the next call on the same
//!   handle that returns a string. Copy them straight away (the C++ shell turns them into
//!   `std::string` immediately).
//! - Entity ids are the world file's ids as `int64_t`; 0 means "none".
//! - No panic crosses the boundary: a panic is caught, logged, put in the status line, and the
//!   function returns its default.

use std::ffi::{CStr, CString, c_char};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::Path;

use lantern_editor_core::{EditorKey, GizmoTool, Modifiers, PointerButton};
use lantern_world_format::EntityKey;

use crate::EditorBackend;

/// The opaque handle C sees.
pub struct LanternEditor {
    pub(crate) backend: EditorBackend,
    /// Backing storage for the most recently returned string.
    returned_string: CString,
    /// Status text set by the C layer itself (a caught panic), shown in preference to the session's.
    pub(crate) api_status: Option<String>,
    /// The outline and inspector as of a UI revision, so per-row calls don't rebuild them.
    outline_cache: Option<(u64, Vec<lantern_editor_core::OutlineRow>)>,
    inspector_cache: Option<(u64, Vec<lantern_editor_core::InspectorField>)>,
}

impl LanternEditor {
    pub(crate) fn give_string(&mut self, text: &str) -> *const c_char {
        // Interior NULs cannot appear in a C string; replace them rather than fail.
        self.returned_string = CString::new(text.replace('\0', " ")).expect("NULs replaced");
        self.returned_string.as_ptr()
    }

    pub(crate) fn outline(&mut self) -> &[lantern_editor_core::OutlineRow] {
        let revision = self.backend.session.ui_revision();
        if self.outline_cache.as_ref().is_none_or(|(cached, _)| *cached != revision) {
            self.outline_cache = Some((revision, self.backend.session.outline()));
        }
        &self.outline_cache.as_ref().expect("filled above").1
    }

    pub(crate) fn inspector(&mut self) -> &[lantern_editor_core::InspectorField] {
        let revision = self.backend.session.ui_revision();
        if self.inspector_cache.as_ref().is_none_or(|(cached, _)| *cached != revision) {
            self.inspector_cache = Some((revision, self.backend.session.inspector()));
        }
        &self.inspector_cache.as_ref().expect("filled above").1
    }
}

/// Reads a borrowed C string. Null or invalid UTF-8 reads as empty.
///
/// # Safety
/// `text` is null or points to a NUL-terminated string valid for the call.
pub(crate) unsafe fn read_string<'call>(text: *const c_char) -> &'call str {
    if text.is_null() {
        return "";
    }
    // SAFETY: the caller guarantees a NUL-terminated string that outlives this call.
    unsafe { CStr::from_ptr(text) }.to_str().unwrap_or("")
}

/// Runs `body` on the editor behind `handle`, catching panics.
///
/// # Safety
/// `handle` is null or a live pointer from `lantern_editor_create`, not used concurrently.
pub(crate) unsafe fn with_editor<T: Default>(
    handle: *mut LanternEditor,
    body: impl FnOnce(&mut LanternEditor) -> T,
) -> T {
    if handle.is_null() {
        return T::default();
    }
    // SAFETY: the caller guarantees a live, exclusively used handle.
    let editor = unsafe { &mut *handle };
    match catch_unwind(AssertUnwindSafe(|| body(editor))) {
        Ok(value) => value,
        Err(panic) => {
            let message = panic
                .downcast_ref::<&str>()
                .map(|text| text.to_string())
                .or_else(|| panic.downcast_ref::<String>().cloned())
                .unwrap_or_else(|| "unknown panic".to_string());
            log::error!(target: "editor", "internal error: {message}");
            // SAFETY: as above; the panicking borrow has ended.
            unsafe { &mut *handle }.api_status = Some(format!("Internal error: {message}"));
            T::default()
        }
    }
}

/// Wrapper so string-returning functions can use `with_editor` (pointers have no `Default`).
pub(crate) struct ReturnedString(pub(crate) *const c_char);

impl Default for ReturnedString {
    fn default() -> Self {
        ReturnedString(c"".as_ptr())
    }
}

pub(crate) fn key(id: i64) -> EntityKey {
    EntityKey(id.max(0) as u64)
}

pub(crate) fn id(key: EntityKey) -> i64 {
    key.0 as i64
}

// ----- Lifetime --------------------------------------------------------------------------------

/// Creates an editor. `assets_directory` holds `editor/palette.json`, `worlds/`, `textures/`
/// and `models/`. `world_path` may be null to open `<assets>/worlds/course.world.json`.
///
/// # Safety
/// Both arguments are null or NUL-terminated strings.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lantern_editor_create(
    assets_directory: *const c_char,
    world_path: *const c_char,
) -> *mut LanternEditor {
    lantern_core::logging::init(log::LevelFilter::Info);
    // SAFETY: forwarded from the caller.
    let (assets, world) = unsafe { (read_string(assets_directory), read_string(world_path)) };
    let result = catch_unwind(|| {
        let world = (!world.is_empty()).then(|| Path::new(world));
        EditorBackend::new(Path::new(assets), world)
    });
    match result {
        Ok(backend) => Box::into_raw(Box::new(LanternEditor {
            backend,
            returned_string: CString::default(),
            api_status: None,
            outline_cache: None,
            inspector_cache: None,
        })),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Destroys an editor. Unsaved changes are lost; ask with `lantern_editor_is_dirty` first.
///
/// # Safety
/// `handle` is null or from `lantern_editor_create`, and is not used afterwards.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lantern_editor_destroy(handle: *mut LanternEditor) {
    if !handle.is_null() {
        // SAFETY: ownership returns to Rust exactly once.
        drop(unsafe { Box::from_raw(handle) });
    }
}

// ----- Document --------------------------------------------------------------------------------

/// Changes whenever anything the panels show changes; poll it once per frame.
///
/// # Safety
/// See the module rules.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lantern_editor_revision(handle: *mut LanternEditor) -> u64 {
    // SAFETY: see the module rules.
    unsafe {
        with_editor(handle, |editor| editor.backend.session.ui_revision() + u64::from(editor.api_status.is_some()))
    }
}

/// The status line: the result of the last action or the last error.
///
/// # Safety
/// See the module rules.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lantern_editor_status(handle: *mut LanternEditor) -> *const c_char {
    // SAFETY: see the module rules.
    unsafe {
        with_editor(handle, |editor| {
            let status = editor.api_status.clone().unwrap_or_else(|| editor.backend.session.status().to_string());
            ReturnedString(editor.give_string(&status))
        })
    }
    .0
}

/// The file name plus ` *` when there are unsaved changes.
///
/// # Safety
/// See the module rules.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lantern_editor_title(handle: *mut LanternEditor) -> *const c_char {
    // SAFETY: see the module rules.
    unsafe {
        with_editor(handle, |editor| {
            let title = editor.backend.session.title();
            ReturnedString(editor.give_string(&title))
        })
    }
    .0
}

/// The open file's path, or "" for a world never saved.
///
/// # Safety
/// See the module rules.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lantern_editor_document_path(handle: *mut LanternEditor) -> *const c_char {
    // SAFETY: see the module rules.
    unsafe {
        with_editor(handle, |editor| {
            let path =
                editor.backend.session.document().path().map(|path| path.display().to_string()).unwrap_or_default();
            ReturnedString(editor.give_string(&path))
        })
    }
    .0
}

/// # Safety
/// See the module rules.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lantern_editor_is_dirty(handle: *mut LanternEditor) -> bool {
    // SAFETY: see the module rules.
    unsafe { with_editor(handle, |editor| editor.backend.session.document().is_dirty()) }
}

/// Opens a world file, replacing the current one (check `is_dirty` first).
///
/// # Safety
/// See the module rules.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lantern_editor_open(handle: *mut LanternEditor, path: *const c_char) -> bool {
    // SAFETY: see the module rules.
    let path = unsafe { read_string(path) }.to_string();
    // SAFETY: see the module rules.
    unsafe {
        with_editor(handle, |editor| {
            editor.api_status = None;
            editor.backend.session.load(Path::new(&path))
        })
    }
}

/// Saves to the world's own file.
///
/// # Safety
/// See the module rules.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lantern_editor_save(handle: *mut LanternEditor) -> bool {
    // SAFETY: see the module rules.
    unsafe {
        with_editor(handle, |editor| {
            editor.api_status = None;
            editor.backend.session.save()
        })
    }
}

/// Saves to `path`, which becomes the world's file.
///
/// # Safety
/// See the module rules.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lantern_editor_save_as(handle: *mut LanternEditor, path: *const c_char) -> bool {
    // SAFETY: see the module rules.
    let path = unsafe { read_string(path) }.to_string();
    // SAFETY: see the module rules.
    unsafe {
        with_editor(handle, |editor| {
            editor.api_status = None;
            if path.trim().is_empty() {
                return false;
            }
            editor.backend.session.save_as(Path::new(&path))
        })
    }
}

/// Starts an empty, unsaved world.
///
/// # Safety
/// See the module rules.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lantern_editor_new_world(handle: *mut LanternEditor, name: *const c_char) {
    // SAFETY: see the module rules.
    let name = unsafe { read_string(name) }.to_string();
    // SAFETY: see the module rules.
    unsafe {
        with_editor(handle, |editor| {
            editor.api_status = None;
            editor.backend.session.new_world(if name.is_empty() { "untitled" } else { &name });
        })
    }
}

// ----- Viewport --------------------------------------------------------------------------------

/// Renders the viewport at `width` x `height` physical pixels. Returns false when the GPU is
/// unavailable (the status line says why); the panels keep working.
///
/// # Safety
/// See the module rules.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lantern_editor_render(handle: *mut LanternEditor, width: u32, height: u32) -> bool {
    // SAFETY: see the module rules.
    unsafe {
        with_editor(handle, |editor| match editor.backend.render(width, height) {
            Ok(()) => true,
            Err(error) => {
                if editor.api_status.as_deref() != Some(error.as_str()) {
                    editor.api_status = Some(error);
                }
                false
            }
        })
    }
}

/// The last rendered viewport: RGBA8 (sRGB), top row first, `width * height * 4` bytes, valid
/// until the next `lantern_editor_render`. Null before the first successful render.
///
/// # Safety
/// See the module rules; `width` and `height` are null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lantern_editor_pixels(
    handle: *mut LanternEditor,
    width: *mut u32,
    height: *mut u32,
) -> *const u8 {
    struct Pixels(*const u8, u32, u32);
    impl Default for Pixels {
        fn default() -> Self {
            Pixels(std::ptr::null(), 0, 0)
        }
    }
    // SAFETY: see the module rules.
    let pixels = unsafe {
        with_editor(handle, |editor| {
            let (bytes, pixel_width, pixel_height) = editor.backend.pixels();
            if bytes.is_empty() { Pixels::default() } else { Pixels(bytes.as_ptr(), pixel_width, pixel_height) }
        })
    };
    // SAFETY: the caller passes null or writable pointers.
    unsafe {
        if !width.is_null() {
            *width = pixels.1;
        }
        if !height.is_null() {
            *height = pixels.2;
        }
    }
    pixels.0
}

/// Pointer position in viewport pixels (the same pixels passed to `render`), origin top-left.
///
/// # Safety
/// See the module rules.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lantern_editor_pointer_move(handle: *mut LanternEditor, x: f32, y: f32) {
    // SAFETY: see the module rules.
    unsafe { with_editor(handle, |editor| editor.backend.session.pointer_moved(glam::Vec2::new(x, y))) }
}

/// `button`: 0 left, 1 middle, 2 right. `modifiers`: `LANTERN_MODIFIER_*` bits.
///
/// # Safety
/// See the module rules.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lantern_editor_pointer_button(
    handle: *mut LanternEditor,
    button: u32,
    down: bool,
    modifiers: u32,
) {
    let Some(button) = PointerButton::from_index(button) else { return };
    // SAFETY: see the module rules.
    unsafe {
        with_editor(handle, |editor| {
            editor.api_status = None;
            editor.backend.session.pointer_button(button, down, Modifiers::from_bits(modifiers));
        })
    }
}

/// Wheel notches; positive zooms in.
///
/// # Safety
/// See the module rules.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lantern_editor_wheel(handle: *mut LanternEditor, notches: f32) {
    // SAFETY: see the module rules.
    unsafe { with_editor(handle, |editor| editor.backend.session.wheel(notches)) }
}

/// A key: lowercase ASCII letters, `LANTERN_KEY_DELETE` or `LANTERN_KEY_ESCAPE`. Returns
/// true when the viewport used it.
///
/// # Safety
/// See the module rules.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lantern_editor_key(
    handle: *mut LanternEditor,
    key_name: *const c_char,
    down: bool,
    modifiers: u32,
) -> bool {
    // SAFETY: see the module rules.
    let Some(editor_key) = EditorKey::from_name(unsafe { read_string(key_name) }) else { return false };
    // SAFETY: see the module rules.
    unsafe {
        with_editor(handle, |editor| editor.backend.session.key(editor_key, down, Modifiers::from_bits(modifiers)))
    }
}

/// Advances time-based input (flying). Call once per frame.
///
/// # Safety
/// See the module rules.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lantern_editor_update(handle: *mut LanternEditor, seconds: f32) {
    // SAFETY: see the module rules.
    unsafe {
        with_editor(handle, |editor| {
            editor.backend.session.update(seconds.clamp(0.0, 0.25));
            editor.backend.poll_play();
        })
    }
}

/// Frames the selection, or the whole world when nothing is selected.
///
/// # Safety
/// See the module rules.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lantern_editor_frame_selection(handle: *mut LanternEditor) {
    // SAFETY: see the module rules.
    unsafe { with_editor(handle, |editor| editor.backend.session.frame_selection()) }
}

/// "move", "rotate" or "scale".
///
/// # Safety
/// See the module rules.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lantern_editor_set_tool(handle: *mut LanternEditor, name: *const c_char) {
    // SAFETY: see the module rules.
    let name = unsafe { read_string(name) }.to_string();
    let Some(tool) = GizmoTool::from_name(&name) else { return };
    // SAFETY: see the module rules.
    unsafe { with_editor(handle, |editor| editor.backend.session.set_tool(tool)) }
}

/// # Safety
/// See the module rules.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lantern_editor_tool(handle: *mut LanternEditor) -> *const c_char {
    // SAFETY: see the module rules.
    unsafe {
        with_editor(handle, |editor| {
            let name = editor.backend.session.tool().name();
            ReturnedString(editor.give_string(name))
        })
    }
    .0
}

/// # Safety
/// See the module rules.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lantern_editor_set_snap(handle: *mut LanternEditor, enabled: bool) {
    // SAFETY: see the module rules.
    unsafe { with_editor(handle, |editor| editor.backend.session.set_snap_enabled(enabled)) }
}

/// # Safety
/// See the module rules.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lantern_editor_snap(handle: *mut LanternEditor) -> bool {
    // SAFETY: see the module rules.
    unsafe { with_editor(handle, |editor| editor.backend.session.snap.enabled) }
}

// ----- Outline and selection -------------------------------------------------------------------

/// Rows of the entity tree, parents before children.
///
/// # Safety
/// See the module rules.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lantern_editor_outline_count(handle: *mut LanternEditor) -> u32 {
    // SAFETY: see the module rules.
    unsafe { with_editor(handle, |editor| editor.backend.session.world().entities.len() as u32) }
}

/// Which column of an outline row to read.
pub(crate) fn outline_row(editor: &mut LanternEditor, row: u32) -> Option<lantern_editor_core::OutlineRow> {
    editor.outline().get(row as usize).cloned()
}

/// # Safety
/// See the module rules.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lantern_editor_outline_id(handle: *mut LanternEditor, row: u32) -> i64 {
    // SAFETY: see the module rules.
    unsafe { with_editor(handle, |editor| outline_row(editor, row).map_or(0, |row| id(row.key))) }
}

/// # Safety
/// See the module rules.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lantern_editor_outline_depth(handle: *mut LanternEditor, row: u32) -> u32 {
    // SAFETY: see the module rules.
    unsafe { with_editor(handle, |editor| outline_row(editor, row).map_or(0, |row| row.depth)) }
}

/// # Safety
/// See the module rules.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lantern_editor_entity_name(handle: *mut LanternEditor, entity: i64) -> *const c_char {
    // SAFETY: see the module rules.
    unsafe {
        with_editor(handle, |editor| {
            let name = editor
                .backend
                .session
                .world()
                .entity(key(entity))
                .map(|entity| entity.name.clone())
                .unwrap_or_default();
            ReturnedString(editor.give_string(&name))
        })
    }
    .0
}

/// # Safety
/// See the module rules.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lantern_editor_entity_kind(handle: *mut LanternEditor, entity: i64) -> *const c_char {
    // SAFETY: see the module rules.
    unsafe {
        with_editor(handle, |editor| {
            let kind = editor
                .backend
                .session
                .world()
                .entity(key(entity))
                .map(|entity| entity.kind.clone())
                .unwrap_or_default();
            ReturnedString(editor.give_string(&kind))
        })
    }
    .0
}

/// # Safety
/// See the module rules.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lantern_editor_is_selected(handle: *mut LanternEditor, entity: i64) -> bool {
    // SAFETY: see the module rules.
    unsafe { with_editor(handle, |editor| editor.backend.session.selection().contains(&key(entity))) }
}

/// Selects `entity`; with `additive`, toggles it in the selection instead.
///
/// # Safety
/// See the module rules.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lantern_editor_select(handle: *mut LanternEditor, entity: i64, additive: bool) {
    // SAFETY: see the module rules.
    unsafe {
        with_editor(handle, |editor| {
            if additive {
                editor.backend.session.toggle_selected(key(entity));
            } else {
                editor.backend.session.select_only(key(entity));
            }
        })
    }
}

/// # Safety
/// See the module rules.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lantern_editor_clear_selection(handle: *mut LanternEditor) {
    // SAFETY: see the module rules.
    unsafe { with_editor(handle, |editor| editor.backend.session.clear_selection()) }
}

/// The entity the inspector shows, or 0.
///
/// # Safety
/// See the module rules.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lantern_editor_primary_selection(handle: *mut LanternEditor) -> i64 {
    // SAFETY: see the module rules.
    unsafe { with_editor(handle, |editor| editor.backend.session.primary_selection().map_or(0, id)) }
}

/// # Safety
/// See the module rules.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lantern_editor_selection_count(handle: *mut LanternEditor) -> u32 {
    // SAFETY: see the module rules.
    unsafe { with_editor(handle, |editor| editor.backend.session.selection().len() as u32) }
}

// ----- Inspector -------------------------------------------------------------------------------

pub(crate) fn inspector_field(editor: &mut LanternEditor, index: u32) -> Option<lantern_editor_core::InspectorField> {
    editor.inspector().get(index as usize).cloned()
}

/// Fields of the primary selection, in display order. 0 when nothing is selected.
///
/// # Safety
/// See the module rules.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lantern_editor_field_count(handle: *mut LanternEditor) -> u32 {
    // SAFETY: see the module rules.
    unsafe { with_editor(handle, |editor| editor.backend.session.inspector().len() as u32) }
}

/// One part of field `index`: `LANTERN_FIELD_KEY`, `_LABEL`, `_GROUP`, `_VALUE` or `_KIND`
/// ("text", "number", "choice" or "readonly").
///
/// # Safety
/// See the module rules.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lantern_editor_field(handle: *mut LanternEditor, index: u32, part: u32) -> *const c_char {
    // SAFETY: see the module rules.
    unsafe {
        with_editor(handle, |editor| {
            let text = inspector_field(editor, index).map_or_else(String::new, |field| match part {
                0 => field.key,
                1 => field.label,
                2 => field.group.to_string(),
                3 => field.value,
                4 => field.kind.name().to_string(),
                5 => field.mixed.to_string(),
                _ => String::new(),
            });
            ReturnedString(editor.give_string(&text))
        })
    }
    .0
}

/// How many choices a "choice" field offers.
///
/// # Safety
/// See the module rules.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lantern_editor_field_choice_count(handle: *mut LanternEditor, index: u32) -> u32 {
    // SAFETY: see the module rules.
    unsafe {
        with_editor(handle, |editor| {
            inspector_field(editor, index).map_or(0, |field| field.kind.choices().len() as u32)
        })
    }
}

/// # Safety
/// See the module rules.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lantern_editor_field_choice(
    handle: *mut LanternEditor,
    index: u32,
    choice: u32,
) -> *const c_char {
    // SAFETY: see the module rules.
    unsafe {
        with_editor(handle, |editor| {
            let text = inspector_field(editor, index)
                .and_then(|field| field.kind.choices().get(choice as usize).cloned())
                .unwrap_or_default();
            ReturnedString(editor.give_string(&text))
        })
    }
    .0
}

/// Sets a field from text. Returns false for bad input (the status line says why).
/// Repeated calls for the same entity and field are one undo step until
/// `lantern_editor_commit_field`.
///
/// # Safety
/// See the module rules.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lantern_editor_set_field(
    handle: *mut LanternEditor,
    entity: i64,
    field: *const c_char,
    text: *const c_char,
) -> bool {
    // SAFETY: see the module rules.
    let (field, text) = unsafe { (read_string(field).to_string(), read_string(text).to_string()) };
    // SAFETY: see the module rules.
    unsafe {
        with_editor(handle, |editor| {
            editor.api_status = None;
            editor.backend.session.set_field(key(entity), &field, &text)
        })
    }
}

/// Ends a run of typing into one field (focus left it).
///
/// # Safety
/// See the module rules.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lantern_editor_commit_field(handle: *mut LanternEditor) {
    // SAFETY: see the module rules.
    unsafe { with_editor(handle, |editor| editor.backend.session.commit_field()) }
}

// ----- Editing ---------------------------------------------------------------------------------

/// Templates for the Add menu, from the palette.
///
/// # Safety
/// See the module rules.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lantern_editor_template_count(handle: *mut LanternEditor) -> u32 {
    // SAFETY: see the module rules.
    unsafe { with_editor(handle, |editor| editor.backend.session.palette().templates.len() as u32) }
}

/// `part`: 0 the template id (pass it to `add_entity`), 1 its label.
///
/// # Safety
/// See the module rules.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lantern_editor_template(handle: *mut LanternEditor, index: u32, part: u32) -> *const c_char {
    // SAFETY: see the module rules.
    unsafe {
        with_editor(handle, |editor| {
            let text = editor
                .backend
                .session
                .palette()
                .templates
                .get(index as usize)
                .map(|template| match part {
                    0 => template.id.clone(),
                    1 => template.label.clone(),
                    2 => template.kind.clone(),
                    _ => String::new(),
                })
                .unwrap_or_default();
            ReturnedString(editor.give_string(&text))
        })
    }
    .0
}

/// Adds an entity from a template where the viewport's centre looks, selects it and returns
/// its id (0 on failure).
///
/// # Safety
/// See the module rules.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lantern_editor_add_entity(handle: *mut LanternEditor, template: *const c_char) -> i64 {
    // SAFETY: see the module rules.
    let template = unsafe { read_string(template) }.to_string();
    // SAFETY: see the module rules.
    unsafe {
        with_editor(handle, |editor| {
            editor.api_status = None;
            editor.backend.session.add_entity(&template, false).map_or(0, id)
        })
    }
}

/// # Safety
/// See the module rules.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lantern_editor_delete_selection(handle: *mut LanternEditor) -> bool {
    // SAFETY: see the module rules.
    unsafe { with_editor(handle, |editor| editor.backend.session.delete_selection()) }
}

/// # Safety
/// See the module rules.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lantern_editor_duplicate_selection(handle: *mut LanternEditor) -> bool {
    // SAFETY: see the module rules.
    unsafe { with_editor(handle, |editor| editor.backend.session.duplicate_selection()) }
}

/// # Safety
/// See the module rules.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lantern_editor_undo(handle: *mut LanternEditor) -> bool {
    // SAFETY: see the module rules.
    unsafe {
        with_editor(handle, |editor| {
            editor.api_status = None;
            editor.backend.session.undo()
        })
    }
}

/// # Safety
/// See the module rules.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lantern_editor_redo(handle: *mut LanternEditor) -> bool {
    // SAFETY: see the module rules.
    unsafe {
        with_editor(handle, |editor| {
            editor.api_status = None;
            editor.backend.session.redo()
        })
    }
}

/// # Safety
/// See the module rules.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lantern_editor_can_undo(handle: *mut LanternEditor) -> bool {
    // SAFETY: see the module rules.
    unsafe { with_editor(handle, |editor| editor.backend.session.document().can_undo()) }
}

/// # Safety
/// See the module rules.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lantern_editor_can_redo(handle: *mut LanternEditor) -> bool {
    // SAFETY: see the module rules.
    unsafe { with_editor(handle, |editor| editor.backend.session.document().can_redo()) }
}
