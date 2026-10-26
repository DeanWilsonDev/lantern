use std::collections::{HashMap, HashSet};

use glam::Vec3;

/// What scripts can read during one hook call. Filled in by the game.
#[derive(Clone, Debug, Default)]
pub struct ScriptSnapshot {
    pub time_seconds: f32,
    pub pressed_actions: HashSet<String>,
    pub held_actions: HashSet<String>,
    pub entity_ids_by_name: HashMap<String, i64>,
    pub entities: HashMap<i64, ScriptEntity>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScriptEntity {
    pub position: Vec3,
    pub visible: bool,
}

/// What scripts ask the game to do. Applied after the hook returns, in order.
#[derive(Clone, Debug, PartialEq)]
pub enum ScriptCommand {
    Log { text: String },
    ResetPlayer,
    SetStatus { text: String },
    ShowMessage { text: String, seconds: f32 },
    SetEntityPosition { entity: i64, position: Vec3 },
    SetEntityVisible { entity: i64, visible: bool },
    PlaySound { name: String, volume: f32 },
}

/// The state host functions share with the host: the snapshot for the current call and the
/// commands written so far.
#[derive(Default)]
pub(crate) struct CallState {
    pub snapshot: ScriptSnapshot,
    pub commands: Vec<ScriptCommand>,
}
