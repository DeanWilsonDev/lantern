//! `EditorSession`: one open world plus everything around it (selection, camera, tool, input
//! state). It is the editor backend's single object: the front end calls its operations and
//! reads its views; the viewport feeds it pointer and key input.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use glam::{Mat4, Quat, Vec2, Vec3};
use lantern_world_format::{EntityKey, TransformData, WorldEntity, WorldFile};

use crate::camera::{OrbitCamera, ViewportProjection};
use crate::document::{CoalesceKey, Edit, EditorDocument};
use crate::gizmo::{Axis, GizmoFrame, GizmoTool, SnapSettings, euler_degrees, snap};
use crate::inspector::{self, InspectorField};
use crate::palette::Palette;
use crate::scene_geometry::SceneGeometry;

/// A press that moves less than this many pixels is a click.
const CLICK_SLOP_PIXELS: f32 = 4.0;

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

/// The keys the viewport reacts to. Letters are their lowercase ASCII codes across the C API.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EditorKey {
    Letter(char),
    Delete,
    Escape,
}

impl EditorKey {
    pub const DELETE_CODE: u32 = 127;
    pub const ESCAPE_CODE: u32 = 27;

    pub fn from_code(code: u32) -> Option<EditorKey> {
        match code {
            EditorKey::DELETE_CODE | 8 => Some(EditorKey::Delete),
            EditorKey::ESCAPE_CODE => Some(EditorKey::Escape),
            _ => char::from_u32(code)
                .filter(char::is_ascii_alphabetic)
                .map(|letter| EditorKey::Letter(letter.to_ascii_lowercase())),
        }
    }
}

/// One row of the outline (the entity tree), in display order.
#[derive(Clone, Debug, PartialEq)]
pub struct OutlineRow {
    pub key: EntityKey,
    pub name: String,
    pub kind: String,
    pub depth: u32,
    pub selected: bool,
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
    /// Left button down; becomes a click or a move once the pointer travels.
    Pending {
        press: Vec2,
        hit: Option<EntityKey>,
        additive: bool,
    },
    Orbit,
    Pan,
    Gizmo {
        tool: GizmoTool,
        axis: Axis,
        frame: GizmoFrame,
        start: f32,
        originals: Vec<DragOriginal>,
    },
    /// Dragging the selection itself across the horizontal plane through its pivot.
    Body {
        start_point: Vec3,
        originals: Vec<DragOriginal>,
    },
}

pub struct EditorSession {
    document: EditorDocument,
    palette: Palette,
    selection: Vec<EntityKey>,
    pub camera: OrbitCamera,
    tool: GizmoTool,
    pub snap: SnapSettings,
    geometry: SceneGeometry,
    viewport_size: Vec2,
    pointer: Vec2,
    drag: Option<Drag>,
    hovered_axis: Option<Axis>,
    hovered_entity: Option<EntityKey>,
    buttons_down: [bool; 3],
    keys_held: HashSet<char>,
    status: String,
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
            selection: Vec::new(),
            camera: OrbitCamera::default(),
            tool: GizmoTool::Translate,
            snap: SnapSettings::default(),
            geometry,
            viewport_size: Vec2::new(1280.0, 720.0),
            pointer: Vec2::ZERO,
            drag: None,
            hovered_axis: None,
            hovered_entity: None,
            buttons_down: [false; 3],
            keys_held: HashSet::new(),
            status: String::new(),
            local_revision: 0,
        };
        session.frame_all();
        session
    }

    pub fn open(path: &Path, palette: Palette) -> Result<EditorSession, String> {
        let document = EditorDocument::open(path).map_err(|error| error.to_string())?;
        let world = document.world().clone();
        let mut session = EditorSession::new(world, Some(path.to_path_buf()), palette);
        session.set_status(format!("Opened {}", display_name(path)));
        Ok(session)
    }

    // ----- Views -------------------------------------------------------------------------

    pub fn world(&self) -> &WorldFile {
        self.document.world()
    }

    pub fn palette(&self) -> &Palette {
        &self.palette
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

    pub fn status(&self) -> &str {
        &self.status
    }

    pub fn hovered_axis(&self) -> Option<Axis> {
        match &self.drag {
            Some(Drag::Gizmo { axis, .. }) => Some(*axis),
            _ => self.hovered_axis,
        }
    }

    pub fn hovered_entity(&self) -> Option<EntityKey> {
        self.hovered_entity
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
            rows.push(OutlineRow {
                key,
                name: entity.name.clone(),
                kind: entity.kind.clone(),
                depth,
                selected: self.selection.contains(&key),
            });
            // A cycle (an invalid file) would otherwise loop forever.
            if rows.len() > world.entities.len() {
                break;
            }
            for child in world.entities.iter().rev().filter(|child| parent_of(child) == Some(key)) {
                stack.push((child.id, depth + 1));
            }
        }
        rows
    }

    /// The inspector for the primary selection; empty when nothing is selected.
    pub fn inspector(&self) -> Vec<InspectorField> {
        let Some(key) = self.primary_selection() else { return Vec::new() };
        let world = self.document.world();
        world.entity(key).map_or_else(Vec::new, |entity| inspector::fields_for(entity, world, &self.palette))
    }

    // ----- Status --------------------------------------------------------------------------

    fn set_status(&mut self, status: impl Into<String>) {
        self.status = status.into();
        self.local_revision += 1;
    }

    fn report<T>(&mut self, result: Result<T, String>) -> Option<T> {
        match result {
            Ok(value) => Some(value),
            Err(error) => {
                log::warn!(target: "editor", "{error}");
                self.set_status(error);
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
                self.set_status(format!("Opened {}", display_name(path)));
                true
            }
            Err(error) => {
                self.set_status(error.to_string());
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
        self.drag = None;
        self.geometry = SceneGeometry::build(self.document.world(), self.document.revision());
        self.frame_all();
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

    fn prune_selection(&mut self) {
        let world = self.document.world();
        let before = self.selection.len();
        self.selection.retain(|key| world.entity(*key).is_some());
        if self.selection.len() != before {
            self.local_revision += 1;
        }
    }

    // ----- Edits ---------------------------------------------------------------------------

    pub fn set_tool(&mut self, tool: GizmoTool) {
        if self.tool != tool {
            self.tool = tool;
            self.local_revision += 1;
        }
    }

    pub fn set_snap_enabled(&mut self, enabled: bool) {
        if self.snap.enabled != enabled {
            self.snap.enabled = enabled;
            self.local_revision += 1;
        }
    }

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

    /// Sets one inspector field on `key` from text. Typing into the same field again merges
    /// into one undo step until the selection changes or `commit_field` is called.
    pub fn set_field(&mut self, key: EntityKey, field: &str, text: &str) -> bool {
        let Some(entity) = self.document.world().entity(key).cloned() else {
            self.set_status(format!("no entity {key}"));
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

    /// Ends a run of typing into one field, so the next edit is its own undo step.
    pub fn commit_field(&mut self) {
        self.document.seal_history();
    }

    /// Adds an entity from a palette template where the viewport's centre points (or at the
    /// cursor when `at_cursor`), selects it and returns its id.
    pub fn add_entity(&mut self, template_id: &str, at_cursor: bool) -> Option<EntityKey> {
        let Some(template) = self.palette.template(template_id).cloned() else {
            self.set_status(format!("no template '{template_id}'"));
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
        let label = if count == 1 { "Delete entity".to_string() } else { format!("Delete {count} entities") };
        let result = self.document.execute(&label, edits);
        if self.report(result).is_none() {
            return false;
        }
        self.selection.clear();
        self.selection_changed();
        self.set_status(format!("Deleted {count} {}", if count == 1 { "entity" } else { "entities" }));
        true
    }

    /// Copies the selected entities (not their children) one snap step to the side, and
    /// selects the copies.
    pub fn duplicate_selection(&mut self) -> bool {
        if self.selection.is_empty() {
            return false;
        }
        let world = self.document.world();
        let mut next_key = world.next_entity_key();
        let offset = if self.snap.enabled { self.snap.translate_step.max(0.25) * 4.0 } else { 1.0 };
        let mut names: Vec<String> = world.entities.iter().map(|entity| entity.name.clone()).collect();
        let mut edits = Vec::new();
        let mut copies = Vec::new();
        for key in &self.selection {
            let Some(original) = world.entity(*key) else { continue };
            let mut copy = original.clone();
            copy.id = next_key;
            next_key = EntityKey(next_key.0 + 1);
            copy.name = unique_name_among(&names, &original.name);
            names.push(copy.name.clone());
            copy.transform.position[0] += offset;
            copies.push(copy.id);
            edits.push(Edit::Insert { index: usize::MAX, entity: copy });
        }
        let label = if copies.len() == 1 {
            "Duplicate entity".to_string()
        } else {
            format!("Duplicate {} entities", copies.len())
        };
        let result = self.document.execute(&label, edits);
        if self.report(result).is_none() {
            return false;
        }
        self.selection = copies;
        self.selection_changed();
        true
    }

    /// Makes `parent` the parent of `child` (or the root for `None`), keeping where `child`
    /// is in the world when the parent has no rotation or scale, and its local values otherwise.
    pub fn set_parent(&mut self, child: EntityKey, parent: Option<EntityKey>) -> bool {
        let world = self.document.world();
        let Some(entity) = world.entity(child).cloned() else { return false };
        if let Some(parent) = parent {
            // Refuse cycles: the new parent may not be the child or below it.
            let mut current = Some(parent);
            while let Some(key) = current {
                if key == child {
                    self.set_status("an entity cannot be parented under itself");
                    return false;
                }
                current = world.entity(key).and_then(|entity| entity.parent);
            }
        }
        let mut edited = entity.clone();
        edited.parent = parent;
        let geometry = self.geometry().clone();
        let world_position = geometry.get(child).map_or(Vec3::ZERO, |entity| entity.origin());
        let new_parent_world =
            parent.and_then(|key| geometry.get(key)).map_or(Mat4::IDENTITY, |entity| entity.world_matrix);
        edited.transform.position = new_parent_world.inverse().transform_point3(world_position).to_array();
        let result = self.document.execute("Change parent", vec![Edit::Replace(edited)]);
        self.report(result).is_some()
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

    // ----- Input ---------------------------------------------------------------------------

    pub fn set_viewport_size(&mut self, size: Vec2) {
        self.viewport_size = size.max(Vec2::ONE);
    }

    fn pivot_and_orientation(&mut self) -> Option<(Vec3, Quat)> {
        let primary = self.primary_selection()?;
        let geometry = self.geometry();
        let entity = geometry.get(primary)?;
        let (_, rotation, translation) = entity.world_matrix.to_scale_rotation_translation();
        Some((translation, rotation))
    }

    fn gizmo_frame(&mut self) -> Option<GizmoFrame> {
        let (pivot, orientation) = self.pivot_and_orientation()?;
        let projection = self.projection();
        Some(GizmoFrame::new(self.tool, pivot, orientation, &projection))
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
        let selection = self.selection.clone();
        let geometry = self.geometry().clone();
        let world = self.document.world();
        selection
            .iter()
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
                if (position - press).length() > CLICK_SLOP_PIXELS && self.tool == GizmoTool::Translate {
                    if let Some(hit) = hit {
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
                }
            }
            Some(Drag::Gizmo { tool, axis, frame, start, originals }) => {
                self.update_gizmo_drag(tool, axis, frame, start, &originals);
            }
            Some(Drag::Body { start_point, originals }) => self.update_body_drag(start_point, &originals),
        }
    }

    fn update_hover(&mut self) {
        let projection = self.projection();
        let tool = self.tool;
        let hovered_axis = self.gizmo_frame().and_then(|frame| frame.hit_test(tool, &projection, self.pointer));
        let ray = projection.ray(self.pointer);
        let hovered_entity = if hovered_axis.is_some() { None } else { self.geometry().pick(ray).map(|hit| hit.key) };
        self.hovered_axis = hovered_axis;
        self.hovered_entity = hovered_entity;
    }

    pub fn pointer_button(&mut self, button: PointerButton, down: bool, modifiers: Modifiers) {
        let index = button as usize;
        self.buttons_down[index] = down;
        if down {
            self.pointer_pressed(button, modifiers);
        } else {
            self.pointer_released(button);
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
                    if let Some(axis) = frame.hit_test(tool, &projection, self.pointer) {
                        let ray = projection.ray(self.pointer);
                        let start = match tool {
                            GizmoTool::Rotate => frame.ring_angle(axis, ray),
                            GizmoTool::Translate | GizmoTool::Scale => frame.axis_parameter(axis, ray),
                        };
                        if let Some(start) = start {
                            let originals = self.drag_originals();
                            self.drag = Some(Drag::Gizmo { tool, axis, frame, start, originals });
                            return;
                        }
                    }
                }
                let ray = projection.ray(self.pointer);
                let hit = self.geometry().pick(ray).map(|hit| hit.key);
                self.drag =
                    Some(Drag::Pending { press: self.pointer, hit, additive: modifiers.shift || modifiers.command });
            }
        }
    }

    fn pointer_released(&mut self, button: PointerButton) {
        let Some(drag) = self.drag.take() else { return };
        match (button, drag) {
            (PointerButton::Left, Drag::Pending { hit, additive, .. }) => match (hit, additive) {
                (Some(key), true) => self.toggle_selected(key),
                (Some(key), false) => self.select_only(key),
                (None, false) => self.clear_selection(),
                (None, true) => {}
            },
            (PointerButton::Left, Drag::Gizmo { tool, originals, .. }) => {
                let verb = match tool {
                    GizmoTool::Translate => "Move",
                    GizmoTool::Rotate => "Rotate",
                    GizmoTool::Scale => "Scale",
                };
                self.document.commit_preview(&drag_label(verb, originals.len()));
            }
            (PointerButton::Left, Drag::Body { originals, .. }) => {
                self.document.commit_preview(&drag_label("Move", originals.len()));
            }
            (PointerButton::Right | PointerButton::Middle, Drag::Orbit | Drag::Pan) => {}
            (PointerButton::Left, Drag::Orbit) => {}
            // A different button was released than the one that started the drag: keep going.
            (_, other) => self.drag = Some(other),
        }
        self.update_hover();
    }

    /// Cancels a drag in progress, putting everything back.
    pub fn cancel_drag(&mut self) -> bool {
        match self.drag.take() {
            Some(Drag::Gizmo { .. } | Drag::Body { .. }) => {
                self.document.cancel_preview();
                true
            }
            Some(_) => true,
            None => false,
        }
    }

    pub fn wheel(&mut self, notches: f32) {
        self.camera.dolly(notches);
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
            EditorKey::Letter('z') if modifiers.command && modifiers.shift => self.redo(),
            EditorKey::Letter('z') if modifiers.command => self.undo(),
            EditorKey::Letter('y') if modifiers.command => self.redo(),
            EditorKey::Letter('d') if modifiers.command => self.duplicate_selection(),
            EditorKey::Letter('s') if modifiers.command => self.save(),
            EditorKey::Letter('a') if modifiers.command => {
                self.select_all();
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
        self.drag = Some(Drag::Body { start_point, originals });
        self.update_body_drag(start_point, &self.drag_originals_from_state());
    }

    fn drag_originals_from_state(&self) -> Vec<DragOriginal> {
        match &self.drag {
            Some(Drag::Body { originals, .. } | Drag::Gizmo { originals, .. }) => originals.clone(),
            _ => Vec::new(),
        }
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

    fn update_gizmo_drag(
        &mut self,
        tool: GizmoTool,
        axis: Axis,
        frame: GizmoFrame,
        start: f32,
        originals: &[DragOriginal],
    ) {
        let ray = self.projection().ray(self.pointer);
        match tool {
            GizmoTool::Translate => {
                let Some(parameter) = frame.axis_parameter(axis, ray) else { return };
                let Some(primary) = originals.last() else { return };
                let direction = frame.axis(axis);
                let mut distance = parameter - start;
                if self.snap.enabled {
                    let component = primary.world_position.dot(direction);
                    distance = snap(component + distance, self.snap.translate_step) - component;
                }
                self.preview_world_translation(originals, direction * distance);
            }
            GizmoTool::Rotate => {
                let Some(angle) = frame.ring_angle(axis, ray) else { return };
                let mut turn = lantern_core::math::wrap_angle(angle - start);
                if self.snap.enabled {
                    turn = snap(turn.to_degrees(), self.snap.rotate_step_degrees).to_radians();
                }
                let world_turn = Quat::from_axis_angle(frame.axis(axis), turn);
                for original in originals {
                    let mut entity = original.entity.clone();
                    let (_, parent_rotation, _) = original.parent_world.to_scale_rotation_translation();
                    let local_turn = parent_rotation.inverse() * world_turn * parent_rotation;
                    entity.transform.rotation_degrees =
                        euler_degrees(local_turn * original.entity.transform.rotation());
                    self.document.preview_replace(entity);
                }
            }
            GizmoTool::Scale => {
                let Some(parameter) = frame.axis_parameter(axis, ray) else { return };
                if start.abs() < 1e-4 {
                    return;
                }
                let ratio = parameter / start;
                for original in originals {
                    let mut entity = original.entity.clone();
                    let mut value = original.entity.transform.scale[axis.index()] * ratio;
                    if self.snap.enabled {
                        value = snap(value, self.snap.scale_step);
                    }
                    if value.abs() < 0.01 {
                        value = 0.01_f32.copysign(value);
                    }
                    entity.transform.scale[axis.index()] = value;
                    self.document.preview_replace(entity);
                }
            }
        }
    }
}

fn drag_label(verb: &str, count: usize) -> String {
    if count == 1 { format!("{verb} entity") } else { format!("{verb} {count} entities") }
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
