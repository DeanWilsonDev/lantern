//! `EditorSession`: one open world plus everything around it (selection, camera, tool, view
//! options, input state, the log). It is the editor backend's single object: the front end
//! calls its operations and reads its views; the viewport feeds it pointer and key input.

use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};

use glam::{Mat4, Quat, Vec2, Vec3};
use lantern_world_format::{EntityKey, TransformData, WorldEntity, WorldFile};

use crate::camera::{OrbitCamera, ViewportProjection};
use crate::document::{CoalesceKey, Edit, EditorDocument};
use crate::gizmo::{Axis, GizmoFrame, GizmoSpace, GizmoTool, Handle, PivotMode, SnapSettings, euler_degrees, snap};
use crate::inspector::{self, InspectorField};
use crate::palette::{EntityTemplate, Palette};
use crate::scene_geometry::SceneGeometry;

/// A press that moves less than this many pixels is a click.
const CLICK_SLOP_PIXELS: f32 = 4.0;
const BOOKMARK_COUNT: usize = 9;
const RECENT_FILE_LIMIT: usize = 10;
const LOG_LIMIT: usize = 500;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PointerButton {
    Left,
    Middle,
    Right,
}

impl PointerButton {
    pub fn from_index(index: u32) -> Option<PointerButton> {
        match index {
            0 => Some(PointerButton::Left),
            1 => Some(PointerButton::Middle),
            2 => Some(PointerButton::Right),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Modifiers {
    pub shift: bool,
    /// Ctrl, or Cmd on macOS.
    pub command: bool,
    pub alt: bool,
}

impl Modifiers {
    pub const SHIFT: u32 = 1;
    pub const COMMAND: u32 = 2;
    pub const ALT: u32 = 4;

    pub fn from_bits(bits: u32) -> Modifiers {
        Modifiers {
            shift: bits & Modifiers::SHIFT != 0,
            command: bits & Modifiers::COMMAND != 0,
            alt: bits & Modifiers::ALT != 0,
        }
    }
}

/// The keys the viewport reacts to, named as the web's `KeyboardEvent.key` names them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EditorKey {
    /// A lowercase ASCII letter.
    Letter(char),
    Digit(u8),
    Delete,
    Escape,
    Home,
    End,
}

impl EditorKey {
    pub fn from_name(name: &str) -> Option<EditorKey> {
        match name {
            "Delete" | "Backspace" => Some(EditorKey::Delete),
            "Escape" => Some(EditorKey::Escape),
            "Home" => Some(EditorKey::Home),
            "End" => Some(EditorKey::End),
            _ => {
                let mut characters = name.chars();
                let character = characters.next()?;
                if characters.next().is_some() {
                    return None;
                }
                if character.is_ascii_alphabetic() {
                    Some(EditorKey::Letter(character.to_ascii_lowercase()))
                } else {
                    character.to_digit(10).map(|digit| EditorKey::Digit(digit as u8))
                }
            }
        }
    }
}

/// One row of the outline (the entity tree), in display order.
#[derive(Clone, Debug, PartialEq)]
pub struct OutlineRow {
    pub key: EntityKey,
    pub parent: Option<EntityKey>,
    pub name: String,
    pub kind: String,
    pub depth: u32,
    pub selected: bool,
    pub hidden: bool,
    pub locked: bool,
    pub has_children: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LogLevel {
    Info,
    Warning,
    Error,
}

impl LogLevel {
    pub fn name(self) -> &'static str {
        match self {
            LogLevel::Info => "info",
            LogLevel::Warning => "warning",
            LogLevel::Error => "error",
        }
    }
}

/// One line of the editor's console: what the status line said, and why an action failed.
#[derive(Clone, Debug, PartialEq)]
pub struct LogEntry {
    pub level: LogLevel,
    pub text: String,
}

/// What the viewport draws besides the entities themselves.
#[derive(Clone, Debug, PartialEq)]
pub struct ViewOptions {
    pub show_grid: bool,
    /// World units between minor grid lines.
    pub grid_spacing: f32,
    pub show_colliders: bool,
    pub show_markers: bool,
    pub wireframe: bool,
    pub show_level_bounds: bool,
    /// Draw hidden entities faintly instead of leaving them out.
    pub show_hidden: bool,
    pub show_stats: bool,
}

impl Default for ViewOptions {
    fn default() -> Self {
        ViewOptions {
            show_grid: true,
            grid_spacing: 1.0,
            show_colliders: false,
            show_markers: true,
            wireframe: false,
            show_level_bounds: false,
            show_hidden: false,
            show_stats: false,
        }
    }
}

impl ViewOptions {
    pub const NAMES: [&'static str; 8] =
        ["grid", "grid_spacing", "colliders", "markers", "wireframe", "level_bounds", "hidden", "stats"];

    pub fn get(&self, name: &str) -> Option<String> {
        Some(match name {
            "grid" => self.show_grid.to_string(),
            "grid_spacing" => inspector::format_number(self.grid_spacing),
            "colliders" => self.show_colliders.to_string(),
            "markers" => self.show_markers.to_string(),
            "wireframe" => self.wireframe.to_string(),
            "level_bounds" => self.show_level_bounds.to_string(),
            "hidden" => self.show_hidden.to_string(),
            "stats" => self.show_stats.to_string(),
            _ => return None,
        })
    }

    pub fn set(&mut self, name: &str, value: &str) -> Result<(), String> {
        let flag = || match value.trim() {
            "true" | "on" | "1" => Ok(true),
            "false" | "off" | "0" => Ok(false),
            other => Err(format!("'{other}' is not on or off")),
        };
        match name {
            "grid" => self.show_grid = flag()?,
            "grid_spacing" => {
                let spacing: f32 = value.trim().parse().map_err(|_| format!("'{value}' is not a number"))?;
                if !(spacing > 0.0 && spacing.is_finite()) {
                    return Err("the grid spacing must be positive".to_string());
                }
                self.grid_spacing = spacing;
            }
            "colliders" => self.show_colliders = flag()?,
            "markers" => self.show_markers = flag()?,
            "wireframe" => self.wireframe = flag()?,
            "level_bounds" => self.show_level_bounds = flag()?,
            "hidden" => self.show_hidden = flag()?,
            "stats" => self.show_stats = flag()?,
            other => return Err(format!("unknown view option '{other}'")),
        }
        Ok(())
    }
}

/// An entity as it was when a drag began.
#[derive(Clone, Debug)]
struct DragOriginal {
    entity: WorldEntity,
    /// World matrix of the parent (identity at the root), to turn world deltas into local ones.
    parent_world: Mat4,
    /// World position of the entity's origin.
    world_position: Vec3,
}

#[derive(Clone, Debug)]
enum Drag {
    /// Left button down; becomes a click, a body move or a rubber band once the pointer travels.
    Pending {
        press: Vec2,
        hit: Option<EntityKey>,
        additive: bool,
    },
    Orbit,
    Pan,
    Gizmo {
        tool: GizmoTool,
        handle: Handle,
        frame: GizmoFrame,
        /// The axis parameter or ring angle where the drag began.
        start: f32,
        /// Where the drag began on the handle's plane, for plane and uniform handles.
        start_point: Vec3,
        originals: Vec<DragOriginal>,
    },
    /// Dragging the selection itself across the horizontal plane through its pivot.
    Body {
        start_point: Vec3,
        originals: Vec<DragOriginal>,
    },
    /// Dragging a selection rectangle over empty space.
    RubberBand {
        press: Vec2,
        additive: bool,
    },
}

pub struct EditorSession {
    document: EditorDocument,
    palette: Palette,
    palette_path: Option<PathBuf>,
    selection: Vec<EntityKey>,
    pub camera: OrbitCamera,
    tool: GizmoTool,
    space: GizmoSpace,
    pivot_mode: PivotMode,
    pub snap: SnapSettings,
    pub view: ViewOptions,
    hidden: HashSet<EntityKey>,
    locked: HashSet<EntityKey>,
    bookmarks: [Option<OrbitCamera>; BOOKMARK_COUNT],
    geometry: SceneGeometry,
    viewport_size: Vec2,
    pointer: Vec2,
    drag: Option<Drag>,
    hovered_handle: Option<Handle>,
    hovered_entity: Option<EntityKey>,
    buttons_down: [bool; 3],
    keys_held: HashSet<char>,
    status: String,
    log: Vec<LogEntry>,
    recent_files: Vec<PathBuf>,
    /// Bumped when a template changes, so cached thumbnails are redrawn.
    palette_revision: u64,
    /// Bumped whenever something the panels show changes outside the document (selection,
    /// tool, status).
    local_revision: u64,
}

impl EditorSession {
    pub fn new(world: WorldFile, path: Option<PathBuf>, palette: Palette) -> EditorSession {
        let document = EditorDocument::new(world, path);
        let geometry = SceneGeometry::build(document.world(), document.revision());
        let mut session = EditorSession {
            document,
            palette,
            palette_path: None,
            selection: Vec::new(),
            camera: OrbitCamera::default(),
            tool: GizmoTool::Translate,
            space: GizmoSpace::World,
            pivot_mode: PivotMode::Primary,
            snap: SnapSettings::default(),
            view: ViewOptions::default(),
            hidden: HashSet::new(),
            locked: HashSet::new(),
            bookmarks: [None; BOOKMARK_COUNT],
            geometry,
            viewport_size: Vec2::new(1280.0, 720.0),
            pointer: Vec2::ZERO,
            drag: None,
            hovered_handle: None,
            hovered_entity: None,
            buttons_down: [false; 3],
            keys_held: HashSet::new(),
            status: String::new(),
            log: Vec::new(),
            recent_files: Vec::new(),
            palette_revision: 0,
            local_revision: 0,
        };
        session.frame_all();
        session
    }

    pub fn open(path: &Path, palette: Palette) -> Result<EditorSession, String> {
        let document = EditorDocument::open(path).map_err(|error| error.to_string())?;
        let world = document.world().clone();
        let mut session = EditorSession::new(world, Some(path.to_path_buf()), palette);
        session.remember_recent_file(path);
        session.set_status(format!("Opened {}", display_name(path)));
        Ok(session)
    }

    /// Where the palette was loaded from, so "apply to template" can save it.
    pub fn set_palette_path(&mut self, path: Option<PathBuf>) {
        self.palette_path = path;
    }

    // ----- Views -------------------------------------------------------------------------

    pub fn world(&self) -> &WorldFile {
        self.document.world()
    }

    pub fn palette(&self) -> &Palette {
        &self.palette
    }

    pub fn palette_revision(&self) -> u64 {
        self.palette_revision
    }

    pub fn document(&self) -> &EditorDocument {
        &self.document
    }

    /// Changes whenever anything a panel shows changes. The front end polls it each frame.
    pub fn ui_revision(&self) -> u64 {
        self.document.revision() + self.local_revision
    }

    pub fn geometry(&mut self) -> &SceneGeometry {
        if self.geometry.revision != self.document.revision() {
            self.geometry = SceneGeometry::build(self.document.world(), self.document.revision());
        }
        &self.geometry
    }

    pub fn selection(&self) -> &[EntityKey] {
        &self.selection
    }

    /// The most recently selected entity: the one the inspector shows and the gizmo follows.
    pub fn primary_selection(&self) -> Option<EntityKey> {
        self.selection.last().copied()
    }

    pub fn tool(&self) -> GizmoTool {
        self.tool
    }

    pub fn space(&self) -> GizmoSpace {
        self.space
    }

    pub fn pivot_mode(&self) -> PivotMode {
        self.pivot_mode
    }

    pub fn status(&self) -> &str {
        &self.status
    }

    pub fn log(&self) -> &[LogEntry] {
        &self.log
    }

    pub fn recent_files(&self) -> &[PathBuf] {
        &self.recent_files
    }

    pub fn hovered_handle(&self) -> Option<Handle> {
        match &self.drag {
            Some(Drag::Gizmo { handle, .. }) => Some(*handle),
            _ => self.hovered_handle,
        }
    }

    pub fn hovered_entity(&self) -> Option<EntityKey> {
        self.hovered_entity
    }

    pub fn is_hidden(&self, key: EntityKey) -> bool {
        self.hidden.contains(&key)
    }

    pub fn is_locked(&self, key: EntityKey) -> bool {
        self.locked.contains(&key)
    }

    pub fn has_bookmark(&self, slot: usize) -> bool {
        self.bookmarks.get(slot).is_some_and(Option::is_some)
    }

    /// The selection rectangle being dragged, as two viewport corners.
    pub fn rubber_band(&self) -> Option<(Vec2, Vec2)> {
        match &self.drag {
            Some(Drag::RubberBand { press, .. }) => Some((*press, self.pointer)),
            _ => None,
        }
    }

    pub fn viewport_size(&self) -> Vec2 {
        self.viewport_size
    }

    pub fn projection(&self) -> ViewportProjection {
        ViewportProjection::new(self.camera.render_camera(), self.viewport_size)
    }

    pub fn title(&self) -> String {
        let name = self.document.path().map_or_else(|| "untitled".to_string(), display_name);
        format!("{name}{}", if self.document.is_dirty() { " *" } else { "" })
    }

    /// One line describing the camera for the status bar.
    pub fn camera_description(&self) -> String {
        let position = self.camera.position();
        format!(
            "{} at {}, {}, {}  yaw {}  pitch {}",
            if self.camera.orthographic { "ortho" } else { "persp" },
            inspector::format_number(position.x),
            inspector::format_number(position.y),
            inspector::format_number(position.z),
            inspector::format_number(self.camera.yaw.to_degrees()),
            inspector::format_number(self.camera.pitch.to_degrees()),
        )
    }

    /// The entity tree, parents before children, siblings in file order. An entity whose
    /// parent is missing is shown at the root rather than lost.
    pub fn outline(&self) -> Vec<OutlineRow> {
        let world = self.document.world();
        let parent_of = |entity: &WorldEntity| entity.parent.filter(|parent| world.entity(*parent).is_some());
        let mut rows = Vec::with_capacity(world.entities.len());
        // Depth-first with an explicit stack; children are pushed in reverse so the first
        // child comes out first.
        let mut stack: Vec<(EntityKey, u32)> = world
            .entities
            .iter()
            .rev()
            .filter(|entity| parent_of(entity).is_none())
            .map(|entity| (entity.id, 0))
            .collect();
        while let Some((key, depth)) = stack.pop() {
            let Some(entity) = world.entity(key) else { continue };
            let children: Vec<EntityKey> =
                world.entities.iter().filter(|child| parent_of(child) == Some(key)).map(|child| child.id).collect();
            rows.push(OutlineRow {
                key,
                parent: parent_of(entity),
                name: entity.name.clone(),
                kind: entity.kind.clone(),
                depth,
                selected: self.selection.contains(&key),
                hidden: self.hidden.contains(&key),
                locked: self.locked.contains(&key),
                has_children: !children.is_empty(),
            });
            // A cycle (an invalid file) would otherwise loop forever.
            if rows.len() > world.entities.len() {
                break;
            }
            for child in children.into_iter().rev() {
                stack.push((child, depth + 1));
            }
        }
        rows
    }

    /// The inspector: the primary selection's fields, with a value blanked and marked mixed
    /// where the other selected entities disagree. Empty when nothing is selected.
    pub fn inspector(&self) -> Vec<InspectorField> {
        let Some(key) = self.primary_selection() else { return Vec::new() };
        let world = self.document.world();
        let Some(primary) = world.entity(key) else { return Vec::new() };
        let others: Vec<&WorldEntity> =
            self.selection.iter().filter(|other| **other != key).filter_map(|other| world.entity(*other)).collect();
        inspector::fields_for_selection(primary, &others, world, &self.palette)
    }

    // ----- Status and log ----------------------------------------------------------------------

    fn set_status(&mut self, status: impl Into<String>) {
        self.status = status.into();
        self.log_line(LogLevel::Info, self.status.clone());
        self.local_revision += 1;
    }

    fn log_line(&mut self, level: LogLevel, text: String) {
        if text.is_empty() {
            return;
        }
        self.log.push(LogEntry { level, text });
        if self.log.len() > LOG_LIMIT {
            self.log.remove(0);
        }
    }

    /// Something another layer (the viewport GPU, the game process) wants in the console.
    pub fn note(&mut self, level: LogLevel, text: impl Into<String>) {
        let text = text.into();
        if level != LogLevel::Info {
            self.status = text.clone();
        }
        self.log_line(level, text);
        self.local_revision += 1;
    }

    pub fn clear_log(&mut self) {
        self.log.clear();
        self.local_revision += 1;
    }

    fn report<T>(&mut self, result: Result<T, String>) -> Option<T> {
        match result {
            Ok(value) => Some(value),
            Err(error) => {
                log::warn!(target: "editor", "{error}");
                self.status = error.clone();
                self.log_line(LogLevel::Error, error);
                self.local_revision += 1;
                None
            }
        }
    }

    // ----- Files ---------------------------------------------------------------------------

    pub fn save(&mut self) -> bool {
        let result = self.document.save(None);
        self.finish_save(result)
    }

    pub fn save_as(&mut self, path: &Path) -> bool {
        let result = self.document.save(Some(path));
        self.finish_save(result)
    }

    fn finish_save(&mut self, result: Result<PathBuf, String>) -> bool {
        match self.report(result) {
            Some(path) => {
                self.remember_recent_file(&path);
                self.set_status(format!("Saved {}", display_name(&path)));
                true
            }
            None => false,
        }
    }

    /// Replaces the open world. Unsaved changes are the caller's concern.
    pub fn load(&mut self, path: &Path) -> bool {
        match EditorDocument::open(path) {
            Ok(document) => {
                self.replace_document(document);
                self.remember_recent_file(path);
                self.set_status(format!("Opened {}", display_name(path)));
                true
            }
            Err(error) => {
                self.report::<()>(Err(error.to_string()));
                false
            }
        }
    }

    pub fn new_world(&mut self, name: &str) {
        self.replace_document(EditorDocument::new(WorldFile::new(name), None));
        self.set_status(format!("New world '{name}'"));
    }

    fn replace_document(&mut self, document: EditorDocument) {
        // Revisions keep counting up across documents so the front end always sees a change.
        let carried_revision = self.ui_revision() + 1;
        self.document = document;
        self.local_revision = carried_revision;
        self.selection.clear();
        self.hidden.clear();
        self.locked.clear();
        self.drag = None;
        self.geometry = SceneGeometry::build(self.document.world(), self.document.revision());
        self.frame_all();
    }

    fn remember_recent_file(&mut self, path: &Path) {
        let path = path.to_path_buf();
        self.recent_files.retain(|existing| *existing != path);
        self.recent_files.insert(0, path);
        self.recent_files.truncate(RECENT_FILE_LIMIT);
    }

    /// Restores the recent-file list from preferences.
    pub fn set_recent_files(&mut self, paths: Vec<PathBuf>) {
        self.recent_files = paths;
        self.recent_files.truncate(RECENT_FILE_LIMIT);
        self.local_revision += 1;
    }

    // ----- Selection -----------------------------------------------------------------------

    fn selection_changed(&mut self) {
        self.document.seal_history();
        self.local_revision += 1;
    }

    pub fn select_only(&mut self, key: EntityKey) {
        if self.document.world().entity(key).is_some() {
            self.selection = vec![key];
            self.selection_changed();
        }
    }

    /// Adds `key`, or removes it when already selected.
    pub fn toggle_selected(&mut self, key: EntityKey) {
        if let Some(index) = self.selection.iter().position(|selected| *selected == key) {
            self.selection.remove(index);
        } else if self.document.world().entity(key).is_some() {
            self.selection.push(key);
        }
        self.selection_changed();
    }

    pub fn clear_selection(&mut self) {
        if !self.selection.is_empty() {
            self.selection.clear();
            self.selection_changed();
        }
    }

    pub fn select_all(&mut self) {
        self.selection = self.document.world().entities.iter().map(|entity| entity.id).collect();
        self.selection_changed();
    }

    pub fn invert_selection(&mut self) {
        let current: HashSet<EntityKey> = self.selection.iter().copied().collect();
        self.selection = self
            .document
            .world()
            .entities
            .iter()
            .map(|entity| entity.id)
            .filter(|key| !current.contains(key))
            .collect();
        self.selection_changed();
    }

    /// Selects every entity of the primary selection's kind (or of `kind`).
    pub fn select_kind(&mut self, kind: &str) {
        self.selection = self.document.world().entities_of_kind(kind).map(|entity| entity.id).collect();
        self.selection_changed();
    }

    /// Adds every descendant of the selection.
    pub fn select_children(&mut self) {
        let world = self.document.world();
        let mut index = 0;
        while index < self.selection.len() {
            let parent = self.selection[index];
            for entity in &world.entities {
                if entity.parent == Some(parent) && !self.selection.contains(&entity.id) {
                    self.selection.push(entity.id);
                }
            }
            index += 1;
        }
        self.selection_changed();
    }

    fn prune_selection(&mut self) {
        let world = self.document.world();
        let before = self.selection.len();
        self.selection.retain(|key| world.entity(*key).is_some());
        self.hidden.retain(|key| world.entity(*key).is_some());
        self.locked.retain(|key| world.entity(*key).is_some());
        if self.selection.len() != before {
            self.local_revision += 1;
        }
    }

    /// The selected entities a drag may move: the unlocked ones.
    fn movable_selection(&self) -> Vec<EntityKey> {
        self.selection.iter().copied().filter(|key| !self.locked.contains(key)).collect()
    }

    // ----- Visibility and locking ------------------------------------------------------------

    pub fn set_hidden(&mut self, key: EntityKey, hidden: bool) {
        let changed = if hidden { self.hidden.insert(key) } else { self.hidden.remove(&key) };
        if changed {
            self.local_revision += 1;
        }
    }

    pub fn set_locked(&mut self, key: EntityKey, locked: bool) {
        let changed = if locked { self.locked.insert(key) } else { self.locked.remove(&key) };
        if changed {
            self.local_revision += 1;
        }
    }

    pub fn hide_selection(&mut self) {
        for key in self.selection.clone() {
            self.hidden.insert(key);
        }
        self.set_status(format!("Hid {}", count_entities(self.selection.len())));
    }

    pub fn unhide_all(&mut self) {
        let count = self.hidden.len();
        self.hidden.clear();
        self.set_status(format!("Showed {}", count_entities(count)));
    }

    pub fn lock_selection(&mut self, locked: bool) {
        for key in self.selection.clone() {
            self.set_locked(key, locked);
        }
        self.set_status(format!(
            "{} {}",
            if locked { "Locked" } else { "Unlocked" },
            count_entities(self.selection.len())
        ));
    }

    // ----- Tool state ---------------------------------------------------------------------------

    pub fn set_tool(&mut self, tool: GizmoTool) {
        if self.tool != tool {
            self.tool = tool;
            self.local_revision += 1;
        }
    }

    pub fn set_space(&mut self, space: GizmoSpace) {
        if self.space != space {
            self.space = space;
            self.local_revision += 1;
        }
    }

    pub fn set_pivot_mode(&mut self, mode: PivotMode) {
        if self.pivot_mode != mode {
            self.pivot_mode = mode;
            self.local_revision += 1;
        }
    }

    pub fn set_snap_enabled(&mut self, enabled: bool) {
        if self.snap.enabled != enabled {
            self.snap.enabled = enabled;
            self.local_revision += 1;
        }
    }

    /// Sets a snap step by name: `translate`, `rotate` (degrees) or `scale`.
    pub fn set_snap_step(&mut self, name: &str, value: f32) -> bool {
        if !(value > 0.0 && value.is_finite()) {
            self.report::<()>(Err(format!("a snap step must be positive, not {value}")));
            return false;
        }
        match name {
            "translate" => self.snap.translate_step = value,
            "rotate" => self.snap.rotate_step_degrees = value,
            "scale" => self.snap.scale_step = value,
            other => {
                self.report::<()>(Err(format!("unknown snap step '{other}'")));
                return false;
            }
        }
        self.local_revision += 1;
        true
    }

    pub fn snap_step(&self, name: &str) -> Option<f32> {
        match name {
            "translate" => Some(self.snap.translate_step),
            "rotate" => Some(self.snap.rotate_step_degrees),
            "scale" => Some(self.snap.scale_step),
            _ => None,
        }
    }

    pub fn set_view_option(&mut self, name: &str, value: &str) -> bool {
        let result = self.view.set(name, value);
        let ok = self.report(result).is_some();
        self.local_revision += 1;
        ok
    }

    // ----- History -----------------------------------------------------------------------------

    pub fn undo(&mut self) -> bool {
        self.drag = None;
        let label = self.document.undo_label().map(str::to_string);
        let undone = self.document.undo();
        if undone {
            self.prune_selection();
            self.set_status(format!("Undid {}", label.unwrap_or_else(|| "drag".to_string()).to_lowercase()));
        }
        undone
    }

    pub fn redo(&mut self) -> bool {
        let label = self.document.redo_label().map(str::to_string);
        let redone = self.document.redo();
        if redone {
            self.prune_selection();
            self.set_status(format!("Redid {}", label.unwrap_or_default().to_lowercase()));
        }
        redone
    }

    pub fn history_labels(&self) -> Vec<String> {
        self.document.history_labels()
    }

    pub fn history_position(&self) -> usize {
        self.document.history_position()
    }

    /// Undoes or redoes to the state after `position` steps (0 is the state as opened).
    pub fn jump_history(&mut self, position: usize) -> bool {
        self.drag = None;
        let moved = self.document.jump_history(position);
        if moved {
            self.prune_selection();
            self.set_status(format!("Went to history step {position}"));
        }
        moved
    }

    // ----- Field edits -------------------------------------------------------------------------

    /// Sets one inspector field on `key` from text. Typing into the same field again merges
    /// into one undo step until the selection changes or `commit_field` is called.
    pub fn set_field(&mut self, key: EntityKey, field: &str, text: &str) -> bool {
        let Some(entity) = self.document.world().entity(key).cloned() else {
            self.report::<()>(Err(format!("no entity {key}")));
            return false;
        };
        let Some(edited) = self.report(inspector::apply_field(&entity, field, text)) else { return false };
        if edited == entity {
            return true;
        }
        let label = format!("Edit {}", field.trim_start_matches("property:"));
        let coalesce = Some(CoalesceKey { entity: key, field: field.to_string() });
        let result = self.document.execute_coalescing(&label, vec![Edit::Replace(edited)], coalesce);
        self.report(result).is_some()
    }

    /// Sets one field on every selected entity in one undo step. The name is left alone on all
    /// but the primary, since two entities with one name would be confusing.
    pub fn set_field_on_selection(&mut self, field: &str, text: &str) -> bool {
        if self.selection.len() <= 1 || field == "name" {
            return self.primary_selection().is_some_and(|key| self.set_field(key, field, text));
        }
        self.document.seal_history();
        let world = self.document.world();
        let mut edits = Vec::new();
        for key in &self.selection {
            let Some(entity) = world.entity(*key) else { continue };
            match inspector::apply_field(entity, field, text) {
                Ok(edited) if edited != *entity => edits.push(Edit::Replace(edited)),
                Ok(_) => {}
                Err(error) => {
                    self.report::<()>(Err(error));
                    return false;
                }
            }
        }
        if edits.is_empty() {
            return true;
        }
        let label = format!("Edit {} on {}", field.trim_start_matches("property:"), count_entities(edits.len()));
        let result = self.document.execute(&label, edits);
        self.report(result).is_some()
    }

    /// Ends a run of typing into one field, so the next edit is its own undo step.
    pub fn commit_field(&mut self) {
        self.document.seal_history();
    }

    /// Renames a game property on every selected entity that has it.
    pub fn rename_property(&mut self, old_name: &str, new_name: &str) -> bool {
        let new_name = new_name.trim();
        if new_name.is_empty() {
            self.report::<()>(Err("a property needs a name".to_string()));
            return false;
        }
        let world = self.document.world();
        let mut edits = Vec::new();
        for key in &self.selection {
            let Some(entity) = world.entity(*key) else { continue };
            let Some(value) = entity.properties.get(old_name).cloned() else { continue };
            let mut edited = entity.clone();
            edited.properties.remove(old_name);
            edited.properties.insert(new_name.to_string(), value);
            edits.push(Edit::Replace(edited));
        }
        if edits.is_empty() {
            return false;
        }
        let result = self.document.execute(&format!("Rename {old_name} to {new_name}"), edits);
        self.report(result).is_some()
    }

    // ----- Adding and removing -------------------------------------------------------------------

    /// Adds an entity from a palette template where the viewport's centre points (or at the
    /// cursor when `at_cursor`), selects it and returns its id.
    pub fn add_entity(&mut self, template_id: &str, at_cursor: bool) -> Option<EntityKey> {
        let Some(template) = self.palette.template(template_id).cloned() else {
            self.report::<()>(Err(format!("no template '{template_id}'")));
            return None;
        };
        let pixel = if at_cursor { self.pointer } else { self.viewport_size * 0.5 };
        let ray = self.projection().ray(pixel);
        let target = self.camera.target;
        let surface = self.geometry().placement_point(ray).unwrap_or(target);
        let step = if self.snap.enabled { self.snap.translate_step } else { 0.0 };
        let mut position = Vec3::new(snap(surface.x, step), surface.y, snap(surface.z, step));
        // Boxes are centred, so lift them by half their height to rest on the surface.
        if let Some(lantern_world_format::Shape::Box { half_extents }) = template.shape {
            position.y += half_extents[1];
        }
        position.y += template.lift;
        let world = self.document.world();
        let key = world.next_entity_key();
        let entity = WorldEntity {
            id: key,
            name: unique_name(world, &template.id),
            parent: None,
            kind: template.kind.clone(),
            transform: TransformData::from_position(position),
            shape: template.shape,
            material: template.material.clone(),
            tint: template.tint.clone(),
            collider: template.collider,
            properties: template.properties.clone(),
        };
        let index = world.entities.len();
        let result = self.document.execute(&format!("Add {}", template.label), vec![Edit::Insert { index, entity }]);
        self.report(result)?;
        self.selection = vec![key];
        self.selection_changed();
        self.set_status(format!("Added {}", template.label.to_lowercase()));
        Some(key)
    }

    /// Deletes the selection and everything parented under it.
    pub fn delete_selection(&mut self) -> bool {
        if self.selection.is_empty() {
            return false;
        }
        let world = self.document.world();
        let mut doomed: Vec<EntityKey> = self.selection.clone();
        let mut index = 0;
        while index < doomed.len() {
            let parent = doomed[index];
            for entity in &world.entities {
                if entity.parent == Some(parent) && !doomed.contains(&entity.id) {
                    doomed.push(entity.id);
                }
            }
            index += 1;
        }
        // Remove in reverse file order so each recorded index is right when undone forwards.
        let mut ordered: Vec<(usize, EntityKey)> = doomed
            .iter()
            .filter_map(|key| world.entities.iter().position(|entity| entity.id == *key).map(|index| (index, *key)))
            .collect();
        ordered.sort_unstable_by_key(|(index, _)| std::cmp::Reverse(*index));
        let count = ordered.len();
        let edits = ordered.into_iter().map(|(_, key)| Edit::Remove(key)).collect();
        let result = self.document.execute(&format!("Delete {}", count_entities(count)), edits);
        if self.report(result).is_none() {
            return false;
        }
        self.selection.clear();
        self.selection_changed();
        self.set_status(format!("Deleted {}", count_entities(count)));
        true
    }

    /// Copies the selected entities (not their children) one step to the side along +X, and
    /// selects the copies.
    pub fn duplicate_selection(&mut self) -> bool {
        let offset = if self.snap.enabled { self.snap.translate_step.max(0.25) * 4.0 } else { 1.0 };
        self.duplicate_with_offset(Vec3::X * offset, "Duplicate")
    }

    /// Copies the selection so each copy sits just past the original along `axis`.
    pub fn duplicate_selection_along(&mut self, axis: Axis) -> bool {
        let selection = self.selection.clone();
        let bounds = self.geometry().bounds_of(&selection);
        let extent = if bounds.is_empty() { 1.0 } else { (bounds.max - bounds.min)[axis.index()].max(0.25) };
        let step = if self.snap.enabled {
            snap(extent, self.snap.translate_step).max(self.snap.translate_step)
        } else {
            extent
        };
        self.duplicate_with_offset(axis.unit() * step, &format!("Duplicate along {}", axis.name().to_uppercase()))
    }

    fn duplicate_with_offset(&mut self, world_offset: Vec3, verb: &str) -> bool {
        if self.selection.is_empty() {
            return false;
        }
        let originals = self.drag_originals_for(&self.selection.clone());
        let world = self.document.world();
        let mut next_key = world.next_entity_key();
        let mut names: Vec<String> = world.entities.iter().map(|entity| entity.name.clone()).collect();
        let mut edits = Vec::new();
        let mut copies = Vec::new();
        for original in &originals {
            let mut copy = original.entity.clone();
            copy.id = next_key;
            next_key = EntityKey(next_key.0 + 1);
            copy.name = unique_name_among(&names, &original.entity.name);
            names.push(copy.name.clone());
            let new_world = original.world_position + world_offset;
            copy.transform.position = original.parent_world.inverse().transform_point3(new_world).to_array();
            copies.push(copy.id);
            edits.push(Edit::Insert { index: usize::MAX, entity: copy });
        }
        let result = self.document.execute(&format!("{verb} {}", count_entities(copies.len())), edits);
        if self.report(result).is_none() {
            return false;
        }
        self.selection = copies;
        self.selection_changed();
        true
    }

    /// Mirrors the selection about its own centre across `axis`: positions flip to the other
    /// side and rotations about the other two axes reverse.
    pub fn mirror_selection(&mut self, axis: Axis) -> bool {
        let movable = self.movable_selection();
        if movable.is_empty() {
            return false;
        }
        let center = self.pivot_point(PivotMode::Center).unwrap_or(Vec3::ZERO);
        let originals = self.drag_originals_for(&movable);
        let mut edits = Vec::new();
        for original in &originals {
            let mut entity = original.entity.clone();
            let mut new_world = original.world_position;
            new_world[axis.index()] = 2.0 * center[axis.index()] - new_world[axis.index()];
            entity.transform.position = original.parent_world.inverse().transform_point3(new_world).to_array();
            for other in axis.others() {
                entity.transform.rotation_degrees[other.index()] = -entity.transform.rotation_degrees[other.index()];
            }
            edits.push(Edit::Replace(entity));
        }
        let label = format!("Mirror {} across {}", count_entities(edits.len()), axis.name().to_uppercase());
        let result = self.document.execute(&label, edits);
        self.report(result).is_some()
    }

    /// Drops each selected entity so its bounds rest on whatever is below it, or on the ground.
    pub fn snap_selection_to_ground(&mut self) -> bool {
        let movable = self.movable_selection();
        if movable.is_empty() {
            return false;
        }
        let selected: HashSet<EntityKey> = self.selection.iter().copied().collect();
        let originals = self.drag_originals_for(&movable);
        let geometry = self.geometry().clone();
        let mut edits = Vec::new();
        for original in &originals {
            let Some(placed) = geometry.get(original.entity.id) else { continue };
            let bottom = placed.bounds.min.y;
            let ray = crate::camera::Ray {
                origin: Vec3::new(original.world_position.x, bottom + 0.001, original.world_position.z),
                direction: Vec3::NEG_Y,
            };
            let surface_y = geometry.pick_excluding(ray, &selected).map_or(0.0, |hit| hit.point.y);
            let delta = surface_y - bottom;
            if delta.abs() < 1e-4 {
                continue;
            }
            let mut entity = original.entity.clone();
            let new_world = original.world_position + Vec3::Y * delta;
            entity.transform.position = original.parent_world.inverse().transform_point3(new_world).to_array();
            edits.push(Edit::Replace(entity));
        }
        if edits.is_empty() {
            self.set_status("Already on the ground");
            return true;
        }
        let result = self.document.execute(&format!("Snap {} to ground", count_entities(edits.len())), edits);
        self.report(result).is_some()
    }

    /// Makes `parent` the parent of `child` (or the root for `None`), keeping where `child`
    /// is in the world when the parent has no rotation or scale, and its local values otherwise.
    pub fn set_parent(&mut self, child: EntityKey, parent: Option<EntityKey>) -> bool {
        let world = self.document.world();
        let Some(entity) = world.entity(child).cloned() else { return false };
        if entity.parent == parent {
            return true;
        }
        if let Some(parent) = parent {
            // Refuse cycles: the new parent may not be the child or below it.
            let mut current = Some(parent);
            while let Some(key) = current {
                if key == child {
                    self.report::<()>(Err("an entity cannot be parented under itself".to_string()));
                    return false;
                }
                current = world.entity(key).and_then(|entity| entity.parent);
            }
        }
        let Some(edited) = self.reparented(&entity, parent) else { return false };
        let result = self.document.execute("Change parent", vec![Edit::Replace(edited)]);
        self.report(result).is_some()
    }

    fn reparented(&mut self, entity: &WorldEntity, parent: Option<EntityKey>) -> Option<WorldEntity> {
        let geometry = self.geometry().clone();
        let mut edited = entity.clone();
        edited.parent = parent;
        let world_position = geometry.get(entity.id).map_or(Vec3::ZERO, |entity| entity.origin());
        let new_parent_world =
            parent.and_then(|key| geometry.get(key)).map_or(Mat4::IDENTITY, |entity| entity.world_matrix);
        edited.transform.position = new_parent_world.inverse().transform_point3(world_position).to_array();
        Some(edited)
    }

    /// Parents the selection under a new empty entity at its centre, and selects that.
    pub fn group_selection(&mut self) -> bool {
        if self.selection.is_empty() {
            return false;
        }
        let selection = self.selection.clone();
        let center = self.pivot_point(PivotMode::Center).unwrap_or(Vec3::ZERO);
        let world = self.document.world();
        // The group joins the selection's common parent, so grouping inside a group nests.
        let first_parent = world.entity(selection[0]).and_then(|entity| entity.parent);
        let common_parent = selection
            .iter()
            .all(|key| world.entity(*key).is_some_and(|entity| entity.parent == first_parent))
            .then_some(first_parent)
            .flatten();
        let group_key = world.next_entity_key();
        let geometry = self.geometry().clone();
        let parent_world =
            common_parent.and_then(|key| geometry.get(key)).map_or(Mat4::IDENTITY, |entity| entity.world_matrix);
        let group = WorldEntity {
            id: group_key,
            name: unique_name(self.document.world(), "group"),
            parent: common_parent,
            kind: "group".to_string(),
            transform: TransformData::from_position(parent_world.inverse().transform_point3(center)),
            shape: None,
            material: None,
            tint: None,
            collider: None,
            properties: BTreeMap::new(),
        };
        let mut edits = vec![Edit::Insert { index: usize::MAX, entity: group }];
        let group_world = Mat4::from_translation(center);
        for key in &selection {
            let Some(entity) = self.document.world().entity(*key).cloned() else { continue };
            let mut edited = entity.clone();
            edited.parent = Some(group_key);
            let world_position = geometry.get(*key).map_or(Vec3::ZERO, |placed| placed.origin());
            edited.transform.position = group_world.inverse().transform_point3(world_position).to_array();
            edits.push(Edit::Replace(edited));
        }
        let result = self.document.execute(&format!("Group {}", count_entities(selection.len())), edits);
        if self.report(result).is_none() {
            return false;
        }
        self.selection = vec![group_key];
        self.selection_changed();
        self.set_status(format!("Grouped {}", count_entities(selection.len())));
        true
    }

    /// Moves every selected entity's `axis` coordinate to the primary's.
    pub fn align_selection(&mut self, axis: Axis) -> bool {
        let Some(primary) = self.primary_selection() else { return false };
        let movable: Vec<EntityKey> = self.movable_selection().into_iter().filter(|key| *key != primary).collect();
        if movable.is_empty() {
            return false;
        }
        let target = self.geometry().get(primary).map_or(Vec3::ZERO, |placed| placed.origin())[axis.index()];
        let originals = self.drag_originals_for(&movable);
        let edits = originals
            .iter()
            .map(|original| {
                let mut entity = original.entity.clone();
                let mut new_world = original.world_position;
                new_world[axis.index()] = target;
                entity.transform.position = original.parent_world.inverse().transform_point3(new_world).to_array();
                Edit::Replace(entity)
            })
            .collect();
        let result = self
            .document
            .execute(&format!("Align {} on {}", count_entities(movable.len()), axis.name().to_uppercase()), edits);
        self.report(result).is_some()
    }

    /// Spaces the selection evenly along `axis` between its two outermost entities.
    pub fn distribute_selection(&mut self, axis: Axis) -> bool {
        let movable = self.movable_selection();
        if movable.len() < 3 {
            self.report::<()>(Err("distributing needs at least three entities".to_string()));
            return false;
        }
        let mut originals = self.drag_originals_for(&movable);
        originals.sort_by(|a, b| a.world_position[axis.index()].total_cmp(&b.world_position[axis.index()]));
        let first = originals[0].world_position[axis.index()];
        let last = originals[originals.len() - 1].world_position[axis.index()];
        let step = (last - first) / (originals.len() - 1) as f32;
        let edits = originals
            .iter()
            .enumerate()
            .map(|(index, original)| {
                let mut entity = original.entity.clone();
                let mut new_world = original.world_position;
                new_world[axis.index()] = first + step * index as f32;
                entity.transform.position = original.parent_world.inverse().transform_point3(new_world).to_array();
                Edit::Replace(entity)
            })
            .collect();
        let label = format!("Distribute {} along {}", count_entities(movable.len()), axis.name().to_uppercase());
        let result = self.document.execute(&label, edits);
        self.report(result).is_some()
    }

    // ----- Templates -----------------------------------------------------------------------------

    /// The palette template an entity came from, by kind: the first template of that kind.
    pub fn template_for_kind(&self, kind: &str) -> Option<&EntityTemplate> {
        self.palette.templates.iter().find(|template| template.kind == kind)
    }

    /// Resets each selected entity's shape, look, collider and properties to its kind's template,
    /// keeping its name and transform.
    pub fn revert_selection_to_template(&mut self) -> bool {
        let world = self.document.world();
        let mut edits = Vec::new();
        for key in &self.selection {
            let Some(entity) = world.entity(*key) else { continue };
            let Some(template) = self.template_for_kind(&entity.kind) else { continue };
            let mut edited = entity.clone();
            edited.shape = template.shape;
            edited.material = template.material.clone();
            edited.tint = template.tint.clone();
            edited.collider = template.collider;
            edited.properties = template.properties.clone();
            if edited != *entity {
                edits.push(Edit::Replace(edited));
            }
        }
        if edits.is_empty() {
            self.set_status("Nothing to revert");
            return false;
        }
        let result = self.document.execute(&format!("Revert {} to template", count_entities(edits.len())), edits);
        self.report(result).is_some()
    }

    /// Makes the primary entity's shape, look, collider and properties the template for its
    /// kind, and saves the palette so new entities start from them.
    pub fn apply_primary_to_template(&mut self) -> bool {
        let Some(key) = self.primary_selection() else { return false };
        let Some(entity) = self.document.world().entity(key).cloned() else { return false };
        let Some(index) = self.palette.templates.iter().position(|template| template.kind == entity.kind) else {
            self.report::<()>(Err(format!("no template has the kind '{}'", entity.kind)));
            return false;
        };
        let template = &mut self.palette.templates[index];
        template.shape = entity.shape;
        template.material = entity.material.clone();
        template.tint = entity.tint.clone();
        template.collider = entity.collider;
        template.properties = entity.properties.clone();
        let label = template.label.clone();
        self.palette_revision += 1;
        match &self.palette_path {
            Some(path) => {
                let result = self.palette.save(path);
                if self.report(result).is_none() {
                    return false;
                }
                self.set_status(format!("Applied {} to the {label} template", entity.name));
            }
            None => {
                self.set_status(format!("Applied {} to the {label} template (palette not saved: no path)", entity.name))
            }
        }
        true
    }

    // ----- Camera --------------------------------------------------------------------------

    pub fn frame_selection(&mut self) {
        if self.selection.is_empty() {
            self.frame_all();
            return;
        }
        let selection = self.selection.clone();
        let bounds = self.geometry().bounds_of(&selection);
        self.camera.frame(&bounds);
    }

    pub fn frame_all(&mut self) {
        let bounds = self.geometry().level_bounds();
        if !bounds.is_empty() {
            self.camera.frame(&bounds);
            // Framing the whole level from straight on is too far; a three-quarter view reads better.
            self.camera.distance *= 0.8;
        }
    }

    /// Looks at the target from a named direction (`front`, `top`, `perspective`...).
    pub fn camera_preset(&mut self, name: &str) -> bool {
        if self.camera.apply_preset(name).is_none() {
            self.report::<()>(Err(format!("unknown camera preset '{name}'")));
            return false;
        }
        self.set_status(format!("{} view", capitalise(name)));
        true
    }

    pub fn set_orthographic(&mut self, orthographic: bool) {
        if self.camera.orthographic != orthographic {
            self.camera.orthographic = orthographic;
            self.local_revision += 1;
        }
    }

    pub fn set_bookmark(&mut self, slot: usize) -> bool {
        let Some(bookmark) = self.bookmarks.get_mut(slot) else { return false };
        *bookmark = Some(self.camera);
        self.set_status(format!("Saved camera bookmark {}", slot + 1));
        true
    }

    pub fn go_to_bookmark(&mut self, slot: usize) -> bool {
        match self.bookmarks.get(slot).copied().flatten() {
            Some(camera) => {
                self.camera = camera;
                self.set_status(format!("Camera bookmark {}", slot + 1));
                true
            }
            None => {
                self.report::<()>(Err(format!("camera bookmark {} is empty", slot + 1)));
                false
            }
        }
    }

    /// Moves the camera to the primary entity, looking the way it faces.
    pub fn align_camera_to_selection(&mut self) -> bool {
        let Some(primary) = self.primary_selection() else { return false };
        let Some(placed) = self.geometry().get(primary).cloned() else { return false };
        let forward = placed.world_matrix.transform_vector3(Vec3::NEG_Z);
        let eye = placed.origin() + Vec3::Y * 1.6;
        self.camera.orthographic = false;
        self.camera.look_from(eye, forward);
        self.set_status("Camera aligned to the selection");
        true
    }

    /// Moves the primary entity to the camera, facing the way it looks.
    pub fn align_selection_to_camera(&mut self) -> bool {
        let Some(primary) = self.primary_selection() else { return false };
        let Some(entity) = self.document.world().entity(primary).cloned() else { return false };
        let originals = self.drag_originals_for(&[primary]);
        let Some(original) = originals.first() else { return false };
        let mut edited = entity;
        edited.transform.position = original.parent_world.inverse().transform_point3(self.camera.position()).to_array();
        let (_, parent_rotation, _) = original.parent_world.to_scale_rotation_translation();
        let world_rotation = Quat::from_rotation_y(self.camera.yaw) * Quat::from_rotation_x(-self.camera.pitch);
        edited.transform.rotation_degrees = euler_degrees(parent_rotation.inverse() * world_rotation);
        let result = self.document.execute("Align to camera", vec![Edit::Replace(edited)]);
        self.report(result).is_some()
    }

    // ----- Input ---------------------------------------------------------------------------

    pub fn set_viewport_size(&mut self, size: Vec2) {
        self.viewport_size = size.max(Vec2::ONE);
    }

    /// Where the gizmo sits for the selection, and the primary's orientation.
    fn pivot_point(&mut self, mode: PivotMode) -> Option<Vec3> {
        let primary = self.primary_selection()?;
        match mode {
            PivotMode::Primary => Some(self.geometry().get(primary)?.origin()),
            PivotMode::Center => {
                let selection = self.selection.clone();
                let bounds = self.geometry().bounds_of(&selection);
                if bounds.is_empty() { Some(self.geometry().get(primary)?.origin()) } else { Some(bounds.center()) }
            }
        }
    }

    fn pivot_and_orientation(&mut self) -> Option<(Vec3, Quat)> {
        let primary = self.primary_selection()?;
        let pivot = self.pivot_point(self.pivot_mode)?;
        let geometry = self.geometry();
        let entity = geometry.get(primary)?;
        let (_, rotation, _) = entity.world_matrix.to_scale_rotation_translation();
        Some((pivot, rotation))
    }

    fn gizmo_frame(&mut self) -> Option<GizmoFrame> {
        let (pivot, orientation) = self.pivot_and_orientation()?;
        let projection = self.projection();
        Some(GizmoFrame::new(self.tool, self.space, pivot, orientation, &projection))
    }

    /// The gizmo to draw this frame, if anything is selected.
    pub fn visible_gizmo(&mut self) -> Option<GizmoFrame> {
        if let Some(Drag::Gizmo { frame, .. }) = &self.drag {
            // Keep the arms where they started while dragging, so the drag maths stays stable.
            let mut frame = *frame;
            if let Some((pivot, _)) = self.pivot_and_orientation() {
                frame.pivot = pivot;
            }
            return Some(frame);
        }
        self.gizmo_frame()
    }

    fn drag_originals(&mut self) -> Vec<DragOriginal> {
        let movable = self.movable_selection();
        self.drag_originals_for(&movable)
    }

    fn drag_originals_for(&mut self, keys: &[EntityKey]) -> Vec<DragOriginal> {
        let geometry = self.geometry().clone();
        let world = self.document.world();
        keys.iter()
            .filter_map(|key| {
                let entity = world.entity(*key)?.clone();
                let parent_world = entity
                    .parent
                    .and_then(|parent| geometry.get(parent))
                    .map_or(Mat4::IDENTITY, |parent| parent.world_matrix);
                let world_position = geometry.get(*key).map_or(Vec3::ZERO, |entity| entity.origin());
                Some(DragOriginal { entity, parent_world, world_position })
            })
            .collect()
    }

    pub fn pointer_moved(&mut self, position: Vec2) {
        let delta = position - self.pointer;
        self.pointer = position;
        match self.drag.clone() {
            None => self.update_hover(),
            Some(Drag::Orbit) => self.camera.orbit(delta),
            Some(Drag::Pan) => {
                let height = self.viewport_size.y;
                self.camera.pan(delta, height);
            }
            Some(Drag::Pending { press, hit, additive }) => {
                if (position - press).length() > CLICK_SLOP_PIXELS {
                    match hit {
                        Some(hit) if self.tool == GizmoTool::Translate && !self.locked.contains(&hit) => {
                            if !self.selection.contains(&hit) {
                                if additive {
                                    self.selection.push(hit);
                                } else {
                                    self.selection = vec![hit];
                                }
                                self.selection_changed();
                            }
                            self.begin_body_drag(press);
                        }
                        Some(_) => {}
                        None => {
                            self.drag = Some(Drag::RubberBand { press, additive });
                            self.local_revision += 1;
                        }
                    }
                }
            }
            Some(Drag::Gizmo { tool, handle, frame, start, start_point, originals }) => {
                self.update_gizmo_drag(tool, handle, frame, start, start_point, &originals);
            }
            Some(Drag::Body { start_point, originals }) => self.update_body_drag(start_point, &originals),
            Some(Drag::RubberBand { .. }) => {}
        }
    }

    fn update_hover(&mut self) {
        let projection = self.projection();
        let tool = self.tool;
        let hovered_handle = self.gizmo_frame().and_then(|frame| frame.hit_test(tool, &projection, self.pointer));
        let ray = projection.ray(self.pointer);
        let hovered_entity = if hovered_handle.is_some() { None } else { self.pick(ray) };
        self.hovered_handle = hovered_handle;
        self.hovered_entity = hovered_entity;
    }

    /// The entity under a ray, ignoring hidden and locked ones.
    fn pick(&mut self, ray: crate::camera::Ray) -> Option<EntityKey> {
        let mut excluded: HashSet<EntityKey> = self.hidden.iter().copied().collect();
        excluded.extend(self.locked.iter().copied());
        if !self.view.show_markers {
            let geometry = self.geometry();
            excluded.extend(geometry.entities.iter().filter(|entity| entity.is_marker()).map(|entity| entity.key));
        }
        self.geometry().pick_excluding(ray, &excluded).map(|hit| hit.key)
    }

    pub fn pointer_button(&mut self, button: PointerButton, down: bool, modifiers: Modifiers) {
        let index = button as usize;
        self.buttons_down[index] = down;
        if down {
            self.pointer_pressed(button, modifiers);
        } else {
            self.pointer_released(button, modifiers);
        }
    }

    fn pointer_pressed(&mut self, button: PointerButton, modifiers: Modifiers) {
        if self.drag.is_some() {
            return;
        }
        match button {
            PointerButton::Right if modifiers.shift => self.drag = Some(Drag::Pan),
            PointerButton::Right => self.drag = Some(Drag::Orbit),
            PointerButton::Middle => self.drag = Some(Drag::Pan),
            PointerButton::Left if modifiers.alt => self.drag = Some(Drag::Orbit),
            PointerButton::Left => {
                let projection = self.projection();
                let tool = self.tool;
                if let Some(frame) = self.gizmo_frame() {
                    if let Some(handle) = frame.hit_test(tool, &projection, self.pointer) {
                        let ray = projection.ray(self.pointer);
                        let start = match (tool, handle) {
                            (GizmoTool::Rotate, Handle::Axis(axis)) => frame.ring_angle(axis, ray),
                            (_, Handle::Axis(axis)) => frame.axis_parameter(axis, ray),
                            (_, Handle::Plane(_) | Handle::Uniform) => Some(0.0),
                        };
                        let start_point = frame.plane_point(handle, ray).unwrap_or(frame.pivot);
                        if let Some(start) = start {
                            let originals = self.drag_originals();
                            if !originals.is_empty() {
                                self.drag = Some(Drag::Gizmo { tool, handle, frame, start, start_point, originals });
                                return;
                            }
                        }
                    }
                }
                let ray = projection.ray(self.pointer);
                let hit = self.pick(ray);
                self.drag =
                    Some(Drag::Pending { press: self.pointer, hit, additive: modifiers.shift || modifiers.command });
            }
        }
    }

    fn pointer_released(&mut self, button: PointerButton, modifiers: Modifiers) {
        let Some(drag) = self.drag.take() else { return };
        match (button, drag) {
            (PointerButton::Left, Drag::Pending { hit, additive, .. }) => match (hit, additive) {
                (Some(key), true) => self.toggle_selected(key),
                (Some(key), false) => self.select_only(key),
                (None, false) => self.clear_selection(),
                (None, true) => {}
            },
            (PointerButton::Left, Drag::Gizmo { tool, handle, originals, .. }) => {
                let verb = match (tool, handle) {
                    (GizmoTool::Translate, _) => "Move",
                    (GizmoTool::Rotate, _) => "Rotate",
                    (GizmoTool::Scale, _) => "Scale",
                };
                self.document.commit_preview(&format!("{verb} {}", count_entities(originals.len())));
            }
            (PointerButton::Left, Drag::Body { originals, .. }) => {
                self.document.commit_preview(&format!("Move {}", count_entities(originals.len())));
            }
            (PointerButton::Left, Drag::RubberBand { press, additive }) => {
                self.finish_rubber_band(press, additive || modifiers.shift);
            }
            (PointerButton::Right | PointerButton::Middle, Drag::Orbit | Drag::Pan) => {}
            (PointerButton::Left, Drag::Orbit) => {}
            // A different button was released than the one that started the drag: keep going.
            (_, other) => self.drag = Some(other),
        }
        self.update_hover();
    }

    /// Selects every pickable entity whose origin projects inside the rectangle.
    fn finish_rubber_band(&mut self, press: Vec2, additive: bool) {
        let projection = self.projection();
        let geometry = self.geometry().clone();
        let mut picked: Vec<EntityKey> = geometry
            .entities
            .iter()
            .filter(|entity| !self.hidden.contains(&entity.key) && !self.locked.contains(&entity.key))
            .filter(|entity| self.view.show_markers || !entity.is_marker())
            .filter(|entity| {
                projection
                    .project(entity.origin())
                    .is_some_and(|pixel| ViewportProjection::pixel_in_rect(pixel, press, self.pointer))
            })
            .map(|entity| entity.key)
            .collect();
        if additive {
            picked.retain(|key| !self.selection.contains(key));
            self.selection.extend(picked);
        } else {
            self.selection = picked;
        }
        self.selection_changed();
        if !self.selection.is_empty() {
            self.set_status(format!("Selected {}", count_entities(self.selection.len())));
        }
    }

    /// Cancels a drag in progress, putting everything back.
    pub fn cancel_drag(&mut self) -> bool {
        match self.drag.take() {
            Some(Drag::Gizmo { .. } | Drag::Body { .. }) => {
                self.document.cancel_preview();
                true
            }
            Some(_) => {
                self.local_revision += 1;
                true
            }
            None => false,
        }
    }

    pub fn wheel(&mut self, notches: f32) {
        self.camera.dolly(notches);
    }

    /// Forgets every held key. The front end calls it when the pointer leaves the viewport,
    /// since releases that happen elsewhere never reach the editor.
    pub fn release_keys(&mut self) {
        self.keys_held.clear();
    }

    /// Returns true when the key did something (so the front end can stop it propagating).
    pub fn key(&mut self, key: EditorKey, down: bool, modifiers: Modifiers) -> bool {
        if let EditorKey::Letter(letter) = key {
            if down {
                self.keys_held.insert(letter);
            } else {
                self.keys_held.remove(&letter);
            }
        }
        if !down {
            return false;
        }
        let flying = self.buttons_down[PointerButton::Right as usize];
        match key {
            EditorKey::Escape => {
                self.cancel_drag() || {
                    self.clear_selection();
                    true
                }
            }
            EditorKey::Delete => self.delete_selection(),
            EditorKey::End => self.snap_selection_to_ground(),
            EditorKey::Home => {
                self.frame_all();
                true
            }
            EditorKey::Digit(0) => false,
            EditorKey::Digit(digit) if modifiers.command => self.set_bookmark(digit as usize - 1),
            EditorKey::Digit(digit) => self.go_to_bookmark(digit as usize - 1),
            EditorKey::Letter('z') if modifiers.command && modifiers.shift => self.redo(),
            EditorKey::Letter('z') if modifiers.command => self.undo(),
            EditorKey::Letter('y') if modifiers.command => self.redo(),
            EditorKey::Letter('d') if modifiers.command => self.duplicate_selection(),
            EditorKey::Letter('s') if modifiers.command => self.save(),
            EditorKey::Letter('g') if modifiers.command => self.group_selection(),
            EditorKey::Letter('a') if modifiers.command => {
                self.select_all();
                true
            }
            EditorKey::Letter('h') if modifiers.shift => {
                self.unhide_all();
                true
            }
            // While flying, the letters below steer instead.
            EditorKey::Letter(_) if flying => true,
            EditorKey::Letter('w') => {
                self.set_tool(GizmoTool::Translate);
                true
            }
            EditorKey::Letter('e') => {
                self.set_tool(GizmoTool::Rotate);
                true
            }
            EditorKey::Letter('r') => {
                self.set_tool(GizmoTool::Scale);
                true
            }
            EditorKey::Letter('f') => {
                self.frame_selection();
                true
            }
            EditorKey::Letter('x') => {
                self.set_space(self.space.toggled());
                true
            }
            EditorKey::Letter('z') => {
                self.set_pivot_mode(self.pivot_mode.toggled());
                true
            }
            EditorKey::Letter('g') => {
                let shown = !self.view.show_grid;
                self.view.show_grid = shown;
                self.local_revision += 1;
                true
            }
            EditorKey::Letter('h') => {
                if self.selection.is_empty() {
                    return false;
                }
                self.hide_selection();
                self.clear_selection();
                true
            }
            _ => false,
        }
    }

    /// Advances time-based input: flying with W/A/S/D/Q/E while the right button is held.
    pub fn update(&mut self, seconds: f32) {
        if !self.buttons_down[PointerButton::Right as usize] {
            return;
        }
        let held = |letter: char| if self.keys_held.contains(&letter) { 1.0 } else { 0.0 };
        let direction = Vec3::new(held('d') - held('a'), held('e') - held('q'), held('w') - held('s'));
        if direction != Vec3::ZERO {
            self.camera.fly(direction, seconds);
        }
    }

    // ----- Drag maths ----------------------------------------------------------------------

    fn begin_body_drag(&mut self, press: Vec2) {
        let Some((pivot, _)) = self.pivot_and_orientation() else { return };
        let ray = self.projection().ray(press);
        let Some(start_point) = ray.intersect_plane(pivot, Vec3::Y) else { return };
        let originals = self.drag_originals();
        if originals.is_empty() {
            return;
        }
        self.drag = Some(Drag::Body { start_point, originals: originals.clone() });
        self.update_body_drag(start_point, &originals);
    }

    fn update_body_drag(&mut self, start_point: Vec3, originals: &[DragOriginal]) {
        let Some(primary) = originals.last() else { return };
        let ray = self.projection().ray(self.pointer);
        let Some(point) = ray.intersect_plane(start_point, Vec3::Y) else { return };
        let mut delta = point - start_point;
        delta.y = 0.0;
        if self.snap.enabled {
            let step = self.snap.translate_step;
            let target = primary.world_position + delta;
            delta.x = snap(target.x, step) - primary.world_position.x;
            delta.z = snap(target.z, step) - primary.world_position.z;
        }
        self.preview_world_translation(originals, delta);
    }

    fn preview_world_translation(&mut self, originals: &[DragOriginal], world_delta: Vec3) {
        for original in originals {
            let mut entity = original.entity.clone();
            let new_world = original.world_position + world_delta;
            let local = original.parent_world.inverse().transform_point3(new_world);
            entity.transform.position = local.to_array();
            self.document.preview_replace(entity);
        }
    }

    fn snapped_translation(&self, primary: &DragOriginal, delta: Vec3, frame: &GizmoFrame, handle: Handle) -> Vec3 {
        if !self.snap.enabled {
            return delta;
        }
        let step = self.snap.translate_step;
        match handle {
            Handle::Uniform => {
                let target = primary.world_position + delta;
                Vec3::new(snap(target.x, step), snap(target.y, step), snap(target.z, step)) - primary.world_position
            }
            Handle::Plane(normal) => {
                // Snap along each of the plane's two axes separately, so a world-aligned plane
                // snaps to the grid and a local one to its own steps.
                let mut snapped = Vec3::ZERO;
                for axis in normal.others() {
                    let direction = frame.axis(axis);
                    let component = primary.world_position.dot(direction);
                    let along = delta.dot(direction);
                    snapped += direction * (snap(component + along, step) - component);
                }
                snapped
            }
            Handle::Axis(axis) => {
                let direction = frame.axis(axis);
                let component = primary.world_position.dot(direction);
                direction * (snap(component + delta.dot(direction), step) - component)
            }
        }
    }

    fn update_gizmo_drag(
        &mut self,
        tool: GizmoTool,
        handle: Handle,
        frame: GizmoFrame,
        start: f32,
        start_point: Vec3,
        originals: &[DragOriginal],
    ) {
        let ray = self.projection().ray(self.pointer);
        let Some(primary) = originals.last() else { return };
        match (tool, handle) {
            (GizmoTool::Translate, Handle::Axis(axis)) => {
                let Some(parameter) = frame.axis_parameter(axis, ray) else { return };
                let delta = frame.axis(axis) * (parameter - start);
                let delta = self.snapped_translation(primary, delta, &frame, handle);
                self.preview_world_translation(originals, delta);
            }
            (GizmoTool::Translate, Handle::Plane(_) | Handle::Uniform) => {
                let Some(point) = frame.plane_point(handle, ray) else { return };
                let delta = self.snapped_translation(primary, point - start_point, &frame, handle);
                self.preview_world_translation(originals, delta);
            }
            (GizmoTool::Rotate, Handle::Axis(axis)) => {
                let Some(angle) = frame.ring_angle(axis, ray) else { return };
                let mut turn = lantern_core::math::wrap_angle(angle - start);
                if self.snap.enabled {
                    turn = snap(turn.to_degrees(), self.snap.rotate_step_degrees).to_radians();
                }
                let world_turn = Quat::from_axis_angle(frame.axis(axis), turn);
                let pivot = frame.pivot;
                for original in originals {
                    let mut entity = original.entity.clone();
                    let (_, parent_rotation, _) = original.parent_world.to_scale_rotation_translation();
                    let local_turn = parent_rotation.inverse() * world_turn * parent_rotation;
                    entity.transform.rotation_degrees =
                        euler_degrees(local_turn * original.entity.transform.rotation());
                    // Entities away from the pivot swing around it, so a group turns as one.
                    let new_world = pivot + world_turn * (original.world_position - pivot);
                    entity.transform.position = original.parent_world.inverse().transform_point3(new_world).to_array();
                    self.document.preview_replace(entity);
                }
            }
            (GizmoTool::Rotate, _) => {}
            (GizmoTool::Scale, Handle::Axis(axis)) => {
                let Some(parameter) = frame.axis_parameter(axis, ray) else { return };
                if start.abs() < 1e-4 {
                    return;
                }
                self.preview_scale(originals, |scale| scale[axis.index()] *= parameter / start);
            }
            (GizmoTool::Scale, Handle::Plane(normal)) => {
                let Some(point) = frame.plane_point(handle, ray) else { return };
                let ratio = scale_ratio(frame.pivot, start_point, point);
                self.preview_scale(originals, |scale| {
                    for axis in normal.others() {
                        scale[axis.index()] *= ratio;
                    }
                });
            }
            (GizmoTool::Scale, Handle::Uniform) => {
                let Some(point) = frame.plane_point(handle, ray) else { return };
                let ratio = scale_ratio(frame.pivot, start_point, point);
                self.preview_scale(originals, |scale| {
                    for component in scale.iter_mut() {
                        *component *= ratio;
                    }
                });
            }
        }
    }

    fn preview_scale(&mut self, originals: &[DragOriginal], mut apply: impl FnMut(&mut [f32; 3])) {
        for original in originals {
            let mut entity = original.entity.clone();
            let mut scale = original.entity.transform.scale;
            apply(&mut scale);
            for value in scale.iter_mut() {
                if self.snap.enabled {
                    *value = snap(*value, self.snap.scale_step);
                }
                if value.abs() < 0.01 {
                    *value = 0.01_f32.copysign(*value);
                }
            }
            entity.transform.scale = scale;
            self.document.preview_replace(entity);
        }
    }
}

/// How much further from the pivot the pointer is now than when the drag began.
fn scale_ratio(pivot: Vec3, start_point: Vec3, point: Vec3) -> f32 {
    let start_distance = (start_point - pivot).length();
    if start_distance < 1e-4 {
        return 1.0;
    }
    ((point - pivot).length() / start_distance).max(0.01)
}

fn count_entities(count: usize) -> String {
    if count == 1 { "entity".to_string() } else { format!("{count} entities") }
}

fn capitalise(text: &str) -> String {
    let mut characters = text.chars();
    match characters.next() {
        Some(first) => first.to_uppercase().chain(characters).collect(),
        None => String::new(),
    }
}

fn display_name(path: &Path) -> String {
    path.file_name().map_or_else(|| path.display().to_string(), |name| name.to_string_lossy().into_owned())
}

/// `base_1`, `base_2`, ... whichever is free first.
fn unique_name(world: &WorldFile, base: &str) -> String {
    let names: Vec<String> = world.entities.iter().map(|entity| entity.name.clone()).collect();
    unique_name_among(&names, base)
}

fn unique_name_among(names: &[String], base: &str) -> String {
    // Strip an existing numeric suffix, so copying `pillar_2` gives `pillar_3`, not `pillar_2_1`.
    let stem = match base.rsplit_once('_') {
        Some((stem, suffix)) if suffix.chars().all(|character| character.is_ascii_digit()) && !suffix.is_empty() => {
            stem
        }
        _ => base,
    };
    (1..)
        .map(|number| format!("{stem}_{number}"))
        .find(|candidate| !names.iter().any(|name| name == candidate))
        .expect("an unused name exists")
}
