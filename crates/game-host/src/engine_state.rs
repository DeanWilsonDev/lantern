//! Everything the engine owns while a game runs, in one place the script API can reach.
//!
//! The host and every host function share one `EngineState` through `Rc<RefCell<…>>`. A host
//! function borrows it for the length of the call and never calls back into Nyx, so borrows
//! never overlap. The host itself never holds a borrow while a script hook runs.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use glam::Vec2;
use lantern_core::color::Color;
use lantern_core::debug::DebugValues;
use lantern_core::input::{ActionId, ActionKind, ActionSet, Binding, Bindings, GamepadButton, InputSnapshot, KeyCode};
use lantern_debug::DebugHud;
use lantern_engine::audio::AudioSystem;
use lantern_engine::world::{EntityId, SceneId, World};
use lantern_physics::{BodyHandle, CollisionWorld};
use lantern_renderer_3d::ui::{Ui, UiColumn};
use lantern_renderer_3d::{MeshHandle, RenderWorld, TextureHandle};
use lantern_world_format::PropertyValue;

use crate::render_backend::RenderBackend;

/// Every entity a script spawns, or a world file it loads, belongs to this scene, so
/// `World:::Clear` and a reload remove exactly the game's entities.
pub const GAME_SCENE: SceneId = SceneId(1);

/// Engine actions come first in every action set, so their ids never change. Scripts cannot
/// rebind them: reloading has to work even when the script does not load.
pub const ENGINE_RELOAD_ACTION: ActionId = ActionId(0);
pub const ENGINE_TOGGLE_HUD_ACTION: ActionId = ActionId(1);
pub const ENGINE_RELOAD_ACTION_NAME: &str = "engine_reload_scripts";
pub const ENGINE_TOGGLE_HUD_ACTION_NAME: &str = "engine_toggle_debug_hud";

/// The engine's own actions: F5 reloads the game's scripts, F1 (or `) and the pad's Back
/// button toggle the debug HUD.
pub fn engine_input() -> (ActionSet, Bindings) {
    let mut actions = ActionSet::default();
    let reload = actions.register(ENGINE_RELOAD_ACTION_NAME, ActionKind::Button);
    let toggle_hud = actions.register(ENGINE_TOGGLE_HUD_ACTION_NAME, ActionKind::Button);
    debug_assert_eq!((reload, toggle_hud), (ENGINE_RELOAD_ACTION, ENGINE_TOGGLE_HUD_ACTION));
    let mut bindings = Bindings::default();
    bindings
        .bind(reload, Binding::Key(KeyCode::F5))
        .bind(toggle_hud, Binding::Key(KeyCode::F1))
        .bind(toggle_hud, Binding::Key(KeyCode::Backquote))
        .bind(toggle_hud, Binding::Gamepad(GamepadButton::Back));
    (actions, bindings)
}

/// Actions and bindings: the ones in effect, the ones a script is defining, and the snapshot
/// the current hook reads.
pub struct InputState {
    pub actions: ActionSet,
    pub bindings: Bindings,
    /// Set when a script changed the actions or bindings; the frame loop picks it up.
    pub configuration_changed: bool,
    pub snapshot: InputSnapshot,
}

impl Default for InputState {
    fn default() -> Self {
        let (actions, bindings) = engine_input();
        InputState { actions, bindings, configuration_changed: true, snapshot: InputSnapshot::default() }
    }
}

impl InputState {
    pub fn pressed(&self, name: &str) -> Option<bool> {
        self.actions.find(name).map(|action| self.snapshot.pressed(action))
    }
}

/// What a world-file entity carries for game code: its kind and its properties.
#[derive(Clone, Debug, Default)]
pub struct EntityTags {
    pub kind: String,
    pub properties: std::collections::BTreeMap<String, PropertyValue>,
}

/// Requests a script makes that need the platform, applied in the next frame update.
#[derive(Default)]
pub struct PlatformRequests {
    pub quit: bool,
    pub mouse_captured: Option<bool>,
}

pub struct EngineState {
    pub assets_directory: Option<PathBuf>,
    pub world: World,
    pub collision: CollisionWorld,
    pub renderer: Box<dyn RenderBackend>,
    pub audio: AudioSystem,
    pub render_world: RenderWorld,
    pub ui: Ui,
    /// The menu column a script is drawing (between `Ui:::Begin` and `Ui:::End`).
    pub ui_column: Option<UiColumn>,
    pub debug_values: DebugValues,
    pub debug_hud: DebugHud,
    pub input: InputState,
    pub requests: PlatformRequests,
    /// Window size in pixels and the text scale for its pixel density.
    pub screen_size: Vec2,
    pub ui_scale: f32,
    /// Texture per world-file material name ("ground", "wall", ...), set by the game.
    pub materials: HashMap<String, TextureHandle>,
    /// Meshes and textures the script created; released on reload.
    pub script_meshes: Vec<MeshHandle>,
    pub script_textures: Vec<TextureHandle>,
    /// Meshes built for loaded world files; released by `World:::Clear` and on reload.
    pub world_meshes: Vec<MeshHandle>,
    pub entity_tags: HashMap<EntityId, EntityTags>,
    /// Kinematic bodies of world-file entities: moving the entity moves the body.
    pub entity_bodies: HashMap<EntityId, BodyHandle>,
    pub font_atlas: TextureHandle,
    /// Overrides for `Engine:::Flag`, checked before the environment (tests set these).
    pub flags: HashMap<String, bool>,
}

impl EngineState {
    pub fn new(
        mut renderer: Box<dyn RenderBackend>,
        audio: AudioSystem,
        assets_directory: Option<PathBuf>,
    ) -> Result<EngineState, String> {
        let font_atlas = renderer.create_texture(&lantern_renderer_3d::text::font_atlas())?;
        Ok(EngineState {
            assets_directory,
            world: World::new(),
            collision: CollisionWorld::new(),
            renderer,
            audio,
            render_world: RenderWorld { overlay_texture: Some(font_atlas), ..RenderWorld::default() },
            ui: Ui::default(),
            ui_column: None,
            debug_values: DebugValues::default(),
            debug_hud: DebugHud { visible: false, ..DebugHud::default() },
            input: InputState::default(),
            requests: PlatformRequests::default(),
            screen_size: Vec2::new(1280.0, 720.0),
            ui_scale: 2.0,
            materials: HashMap::new(),
            script_meshes: Vec::new(),
            script_textures: Vec::new(),
            world_meshes: Vec::new(),
            entity_tags: HashMap::new(),
            entity_bodies: HashMap::new(),
            font_atlas,
            flags: HashMap::new(),
        })
    }

    /// A path a script names, relative to the assets directory.
    pub fn asset_path(&self, relative: &str) -> PathBuf {
        match &self.assets_directory {
            Some(directory) => directory.join(relative),
            None => Path::new(relative).to_path_buf(),
        }
    }

    /// `Engine:::Flag`: an override, else an environment variable that is set, not empty
    /// and not "0".
    pub fn flag(&self, name: &str) -> bool {
        if let Some(value) = self.flags.get(name) {
            return *value;
        }
        std::env::var(name).is_ok_and(|value| !value.is_empty() && value != "0")
    }

    /// Removes every game entity, the world-file meshes and all collision (`World:::Clear`).
    pub fn clear_world(&mut self) {
        self.world.despawn_scene(GAME_SCENE);
        for mesh in self.world_meshes.drain(..) {
            self.renderer.remove_mesh(mesh);
        }
        self.collision = CollisionWorld::new();
        self.entity_tags.clear();
        self.entity_bodies.clear();
    }

    /// Everything a script session made goes: entities, meshes, textures, materials, music,
    /// the menu state, the game's debug values and its input definitions.
    pub fn reset_for_new_session(&mut self) {
        self.clear_world();
        for mesh in self.script_meshes.drain(..) {
            self.renderer.remove_mesh(mesh);
        }
        for texture in self.script_textures.drain(..) {
            self.renderer.remove_texture(texture);
        }
        self.materials.clear();
        self.audio.stop_music();
        self.ui = Ui::default();
        self.ui_column = None;
        self.debug_values.clear();
        self.input = InputState::default();
        self.requests.mouse_captured = Some(false);
        let overlay_texture = self.render_world.overlay_texture;
        self.render_world = RenderWorld { overlay_texture, ..RenderWorld::default() };
    }
}

/// Parses "RRGGBB" or "RRGGBBAA", with or without a leading '#'.
pub fn parse_color(text: &str) -> Option<Color> {
    let hex = text.strip_prefix('#').unwrap_or(text);
    let value = u32::from_str_radix(hex, 16).ok()?;
    match hex.len() {
        6 => Some(Color::from_hex(value)),
        8 => Some(Color::from_hex(value >> 8).with_alpha((value & 0xFF) as f32 / 255.0)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colors_parse_with_and_without_alpha() {
        let opaque = parse_color("#FF8000").unwrap();
        assert_eq!(opaque.to_array()[3], 1.0);
        let translucent = parse_color("00000099").unwrap();
        assert!((translucent.to_array()[3] - 0.6).abs() < 1e-6);
        assert!(parse_color("nope").is_none());
    }
}
