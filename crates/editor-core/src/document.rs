//! The open world and its edit history.
//!
//! Every change to the world goes through an [`Edit`], which returns its own inverse when
//! applied. A user action is a list of edits recorded as one [`HistoryEntry`], so one undo
//! reverts the whole action (moving three entities is one step, not three).
//!
//! Two kinds of change bypass the normal path:
//!
//! - **Previews** (gizmo drags). While the mouse is held the world is changed directly, with
//!   the original entities remembered. `commit_preview` records one entry from the originals
//!   to the final state; `cancel_preview` restores the originals.
//! - **Coalesced edits** (typing into an inspector field). An entry may carry a coalesce key;
//!   a new entry with the same key replaces the previous entry's "redo" side and keeps its
//!   "undo" side, so typing `1`, `1.`, `1.5` is one undo step.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use lantern_world_format::{EntityKey, WorldEntity, WorldFile, WorldFileError};

/// One primitive change to a world.
#[derive(Clone, Debug, PartialEq)]
pub enum Edit {
    /// Puts `entity` where the entity with the same id is.
    Replace(WorldEntity),
    /// Inserts `entity` at `index` in the entity list (clamped to the end).
    Insert {
        index: usize,
        entity: WorldEntity,
    },
    Remove(EntityKey),
}

impl Edit {
    /// Applies the edit and returns the edit that undoes it.
    pub fn apply(self, world: &mut WorldFile) -> Result<Edit, String> {
        match self {
            Edit::Replace(entity) => {
                let slot = world
                    .entities
                    .iter_mut()
                    .find(|existing| existing.id == entity.id)
                    .ok_or_else(|| format!("no entity {}", entity.id))?;
                let previous = std::mem::replace(slot, entity);
                Ok(Edit::Replace(previous))
            }
            Edit::Insert { index, entity } => {
                if world.entity(entity.id).is_some() {
                    return Err(format!("entity {} already exists", entity.id));
                }
                let id = entity.id;
                let index = index.min(world.entities.len());
                world.entities.insert(index, entity);
                Ok(Edit::Remove(id))
            }
            Edit::Remove(id) => {
                let index = world
                    .entities
                    .iter()
                    .position(|existing| existing.id == id)
                    .ok_or_else(|| format!("no entity {id}"))?;
                let entity = world.entities.remove(index);
                Ok(Edit::Insert { index, entity })
            }
        }
    }
}

/// What makes two consecutive entries merge: the same entity and the same field.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct CoalesceKey {
    pub entity: EntityKey,
    pub field: String,
}

#[derive(Clone, Debug)]
pub struct HistoryEntry {
    pub label: String,
    /// Applied in order to undo.
    undo: Vec<Edit>,
    /// Applied in order to redo.
    redo: Vec<Edit>,
    coalesce: Option<CoalesceKey>,
    /// Identifies the world state after this entry, for "unsaved changes".
    state_id: u64,
}

/// Applies `edits` in order. On failure, rolls back what was applied and returns the error.
/// On success returns the edits that undo them, in the order to apply them.
fn apply_all(world: &mut WorldFile, edits: Vec<Edit>) -> Result<Vec<Edit>, String> {
    let mut inverses = Vec::with_capacity(edits.len());
    for edit in edits {
        match edit.apply(world) {
            Ok(inverse) => inverses.push(inverse),
            Err(error) => {
                for inverse in inverses.into_iter().rev() {
                    let _ = inverse.apply(world);
                }
                return Err(error);
            }
        }
    }
    inverses.reverse();
    Ok(inverses)
}

pub struct EditorDocument {
    world: WorldFile,
    path: Option<PathBuf>,
    undo_stack: Vec<HistoryEntry>,
    redo_stack: Vec<HistoryEntry>,
    next_state_id: u64,
    saved_state_id: u64,
    /// The entry on top of the undo stack may still absorb coalescing edits.
    top_open_for_coalescing: bool,
    /// Entities as they were before the current preview, by id. Empty when not previewing.
    preview_originals: HashMap<EntityKey, WorldEntity>,
    /// Bumped on every change to the world, including previews. Views compare it to decide
    /// whether to rebuild.
    revision: u64,
}

impl EditorDocument {
    pub fn new(world: WorldFile, path: Option<PathBuf>) -> EditorDocument {
        EditorDocument {
            world,
            path,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            next_state_id: 1,
            saved_state_id: 0,
            top_open_for_coalescing: false,
            preview_originals: HashMap::new(),
            revision: 1,
        }
    }

    pub fn open(path: &Path) -> Result<EditorDocument, WorldFileError> {
        let world = WorldFile::load(path)?;
        Ok(EditorDocument::new(world, Some(path.to_path_buf())))
    }

    pub fn world(&self) -> &WorldFile {
        &self.world
    }

    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    pub fn revision(&self) -> u64 {
        self.revision
    }

    fn current_state_id(&self) -> u64 {
        self.undo_stack.last().map_or(0, |entry| entry.state_id)
    }

    pub fn is_dirty(&self) -> bool {
        self.current_state_id() != self.saved_state_id || !self.preview_originals.is_empty()
    }

    /// Saves to `path`, or to the document's own path when `None`.
    pub fn save(&mut self, path: Option<&Path>) -> Result<PathBuf, String> {
        self.commit_preview("Edit");
        let target = path.map(Path::to_path_buf).or_else(|| self.path.clone()).ok_or("the world has no file yet")?;
        self.world.save(&target).map_err(|error| error.to_string())?;
        self.path = Some(target.clone());
        self.saved_state_id = self.current_state_id();
        self.top_open_for_coalescing = false;
        self.revision += 1;
        Ok(target)
    }

    /// Applies a user action as one undo step.
    pub fn execute(&mut self, label: &str, edits: Vec<Edit>) -> Result<(), String> {
        self.execute_coalescing(label, edits, None)
    }

    /// Like `execute`, but merges into the previous step when both carry the same key.
    pub fn execute_coalescing(
        &mut self,
        label: &str,
        edits: Vec<Edit>,
        coalesce: Option<CoalesceKey>,
    ) -> Result<(), String> {
        self.commit_preview("Edit");
        if edits.is_empty() {
            return Ok(());
        }
        let redo = edits.clone();
        let undo = apply_all(&mut self.world, edits)?;
        self.revision += 1;
        self.redo_stack.clear();
        let merge = self.top_open_for_coalescing
            && coalesce.is_some()
            && self.undo_stack.last().is_some_and(|top| top.coalesce == coalesce);
        if merge {
            let top = self.undo_stack.last_mut().expect("checked above");
            top.redo = redo;
            top.state_id = self.next_state_id;
        } else {
            self.undo_stack.push(HistoryEntry {
                label: label.to_string(),
                undo,
                redo,
                coalesce: coalesce.clone(),
                state_id: self.next_state_id,
            });
        }
        self.next_state_id += 1;
        self.top_open_for_coalescing = coalesce.is_some();
        Ok(())
    }

    /// Stops the last step from absorbing further coalescing edits (a field lost focus).
    pub fn seal_history(&mut self) {
        self.top_open_for_coalescing = false;
    }

    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty() || !self.preview_originals.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    pub fn undo_label(&self) -> Option<&str> {
        self.undo_stack.last().map(|entry| entry.label.as_str())
    }

    pub fn redo_label(&self) -> Option<&str> {
        self.redo_stack.last().map(|entry| entry.label.as_str())
    }

    pub fn undo(&mut self) -> bool {
        if !self.preview_originals.is_empty() {
            self.cancel_preview();
            return true;
        }
        let Some(entry) = self.undo_stack.pop() else { return false };
        // History edits were valid when recorded and are applied in reverse order of the
        // world's changes since, so they cannot fail; an error here is a bug worth a log line.
        if let Err(error) = apply_all(&mut self.world, entry.undo.clone()) {
            log::error!(target: "editor", "undo of '{}' failed: {error}", entry.label);
        }
        self.redo_stack.push(entry);
        self.top_open_for_coalescing = false;
        self.revision += 1;
        true
    }

    pub fn redo(&mut self) -> bool {
        self.commit_preview("Edit");
        let Some(entry) = self.redo_stack.pop() else { return false };
        if let Err(error) = apply_all(&mut self.world, entry.redo.clone()) {
            log::error!(target: "editor", "redo of '{}' failed: {error}", entry.label);
        }
        self.undo_stack.push(entry);
        self.top_open_for_coalescing = false;
        self.revision += 1;
        true
    }

    /// Changes an entity during a drag without recording history. The first preview of each
    /// entity remembers how it was.
    pub fn preview_replace(&mut self, entity: WorldEntity) {
        let Some(current) = self.world.entity_mut(entity.id) else { return };
        self.preview_originals.entry(entity.id).or_insert_with(|| current.clone());
        *current = entity;
        self.revision += 1;
    }

    pub fn is_previewing(&self) -> bool {
        !self.preview_originals.is_empty()
    }

    /// Records the preview as one undo step labelled `label`. Unchanged entities are skipped,
    /// so a click without movement records nothing.
    pub fn commit_preview(&mut self, label: &str) {
        if self.preview_originals.is_empty() {
            return;
        }
        let originals = std::mem::take(&mut self.preview_originals);
        let mut undo = Vec::new();
        let mut redo = Vec::new();
        // Keep file order so undo labels and results are deterministic.
        for entity in &self.world.entities {
            let Some(original) = originals.get(&entity.id) else { continue };
            if original != entity {
                undo.push(Edit::Replace(original.clone()));
                redo.push(Edit::Replace(entity.clone()));
            }
        }
        if undo.is_empty() {
            return;
        }
        self.redo_stack.clear();
        self.undo_stack.push(HistoryEntry {
            label: label.to_string(),
            undo,
            redo,
            coalesce: None,
            state_id: self.next_state_id,
        });
        self.next_state_id += 1;
        self.top_open_for_coalescing = false;
        self.revision += 1;
    }

    /// Puts every previewed entity back as it was.
    pub fn cancel_preview(&mut self) {
        for (id, original) in std::mem::take(&mut self.preview_originals) {
            if let Some(entity) = self.world.entity_mut(id) {
                *entity = original;
            }
        }
        self.revision += 1;
    }
}
