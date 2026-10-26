//! The game rules in Nyx: `scripts/platformer.nyx`, hosted by `lantern-scripting`.
//!
//! This adapter turns a `RulesView` into the script's per-step snapshot, maps rule events to
//! script hooks, and maps the script's commands back to `RuleCommand`s. Entity ids cross the
//! boundary as the 64-bit form of their generational handles; ids that no longer name a live
//! entity are dropped with a warning.

use std::path::{Path, PathBuf};

use lantern_core::handle::Handle;
use lantern_engine::world::EntityId;
use lantern_scripting::{HookArgument, ScriptCommand, ScriptEntity, ScriptHook, ScriptHost, ScriptSnapshot};

use crate::rules::{GameRules, RuleCommand, RuleEvent, RulesError, RulesView};

pub const SCRIPT_FILE: &str = "platformer.nyx";
pub const APPLICATION_CLASS: &str = "Platformer";

/// Finds the scripts directory like the assets one: `LANTERN_SCRIPTS`, then `scripts/` beside
/// the executable, then the app's source `scripts/` directory.
pub fn find_script() -> PathBuf {
    let mut candidates = Vec::new();
    if let Some(explicit) = std::env::var_os("LANTERN_SCRIPTS") {
        candidates.push(PathBuf::from(explicit));
    }
    if let Some(directory) = std::env::current_exe().ok().and_then(|path| path.parent().map(Path::to_path_buf)) {
        candidates.push(directory.join("scripts"));
    }
    candidates.push(Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts"));
    candidates
        .iter()
        .map(|directory| directory.join(SCRIPT_FILE))
        .find(|path| path.is_file())
        .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts").join(SCRIPT_FILE))
}

pub struct ScriptedRules {
    host: ScriptHost,
}

fn entity_to_script(entity: EntityId) -> i64 {
    entity.to_bits() as i64
}

fn entity_from_script(id: i64) -> EntityId {
    Handle::from_bits(id as u64)
}

fn snapshot(view: &RulesView) -> ScriptSnapshot {
    let mut snapshot = ScriptSnapshot { time_seconds: view.time_seconds, ..Default::default() };
    for (entity, data) in view.world.iter() {
        let id = entity_to_script(entity);
        snapshot.entity_ids_by_name.insert(data.name.clone(), id);
        let visible = data.mesh_renderer.is_none_or(|renderer| renderer.visible);
        snapshot.entities.insert(id, ScriptEntity { position: data.local.translation, visible });
    }
    for definition in view.action_set.all() {
        if view.action_pressed(&definition.name) {
            snapshot.pressed_actions.insert(definition.name.clone());
        }
        if view.action_held(&definition.name) {
            snapshot.held_actions.insert(definition.name.clone());
        }
    }
    snapshot
}

impl ScriptedRules {
    pub fn load(path: &Path) -> Result<ScriptedRules, String> {
        ScriptHost::load(path, APPLICATION_CLASS).map(|host| ScriptedRules { host }).map_err(|error| error.message)
    }

    fn call(
        &mut self,
        hook: ScriptHook,
        view: &RulesView,
        arguments: &[HookArgument],
        commands: &mut Vec<RuleCommand>,
    ) -> Result<(), RulesError> {
        let (script_commands, result) = self.host.call(hook, snapshot(view), arguments);
        for command in script_commands {
            if let Some(command) = translate(command, view) {
                commands.push(command);
            }
        }
        result.map_err(|error| RulesError { hook: hook.method_name(), message: error.message })
    }
}

fn translate(command: ScriptCommand, view: &RulesView) -> Option<RuleCommand> {
    let live_entity = |id: i64| {
        let entity = entity_from_script(id);
        let alive = view.world.contains(entity);
        if !alive {
            log::warn!(target: "rules", "script used a stale entity id {id}");
        }
        alive.then_some(entity)
    };
    Some(match command {
        ScriptCommand::Log { text } => RuleCommand::Log { text },
        ScriptCommand::ResetPlayer => RuleCommand::ResetPlayer,
        ScriptCommand::SetStatus { text } => RuleCommand::SetStatus { text },
        ScriptCommand::ShowMessage { text, seconds } => RuleCommand::ShowMessage { text, seconds },
        ScriptCommand::SetEntityPosition { entity, position } => {
            RuleCommand::SetEntityPosition { entity: live_entity(entity)?, position }
        }
        ScriptCommand::SetEntityVisible { entity, visible } => {
            RuleCommand::SetEntityVisible { entity: live_entity(entity)?, visible }
        }
        ScriptCommand::PlaySound { name, volume } => RuleCommand::PlaySound { name, volume },
    })
}

impl GameRules for ScriptedRules {
    fn description(&self) -> String {
        format!(
            "Nyx: {}",
            self.host.source_path().file_name().map_or_else(String::new, |name| name.to_string_lossy().into_owned())
        )
    }

    fn on_start(&mut self, view: &RulesView, commands: &mut Vec<RuleCommand>) -> Result<(), RulesError> {
        self.call(ScriptHook::OnStart, view, &[], commands)
    }

    fn on_fixed_update(
        &mut self,
        view: &RulesView,
        dt: f32,
        commands: &mut Vec<RuleCommand>,
    ) -> Result<(), RulesError> {
        self.call(ScriptHook::OnFixedUpdate, view, &[HookArgument::F32(dt)], commands)
    }

    fn on_event(
        &mut self,
        view: &RulesView,
        event: RuleEvent,
        commands: &mut Vec<RuleCommand>,
    ) -> Result<(), RulesError> {
        match event {
            RuleEvent::TriggerEntered { trigger } => {
                self.call(ScriptHook::OnTriggerEnter, view, &[HookArgument::I64(entity_to_script(trigger))], commands)
            }
            RuleEvent::PlayerFell => self.call(ScriptHook::OnPlayerFell, view, &[], commands),
            RuleEvent::PlayerJumped => self.call(ScriptHook::OnJump, view, &[], commands),
            RuleEvent::PlayerLanded { speed } => {
                self.call(ScriptHook::OnLand, view, &[HookArgument::F32(speed)], commands)
            }
            RuleEvent::ResetPressed => self.call(ScriptHook::OnResetPressed, view, &[], commands),
        }
    }

    fn on_exit(&mut self, view: &RulesView, commands: &mut Vec<RuleCommand>) -> Result<(), RulesError> {
        self.call(ScriptHook::OnExit, view, &[], commands)
    }

    fn reload(&mut self) -> Result<bool, String> {
        let path = self.host.source_path().to_path_buf();
        self.host = ScriptHost::load(&path, APPLICATION_CLASS).map_err(|error| error.message)?;
        log::info!(target: "rules", "reloaded {}", path.display());
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::NativeRules;
    use glam::Vec3;
    use lantern_core::input::{ActionSet, InputSnapshot};
    use lantern_engine::world::{MeshRenderer, SceneId, Transform, World};

    /// A tiny level: three gems and the moving platform. Built the same way twice, so the
    /// entity ids match between the two worlds.
    fn small_level() -> (World, Vec<EntityId>, EntityId) {
        let mut world = World::new();
        let mesh = Handle::from_bits(0);
        let renderer = MeshRenderer::new(mesh, None, Default::default());
        let gems = (0..3)
            .map(|index| {
                let transform = Transform::from_translation(Vec3::new(index as f32, 1.0, 0.0));
                world.spawn_mesh(&format!("gem_{index}"), SceneId(1), transform, renderer)
            })
            .collect();
        let platform_transform = Transform::from_translation(Vec3::new(0.0, 1.8, -21.0));
        let platform = world.spawn_mesh("moving_platform", SceneId(1), platform_transform, renderer);
        (world, gems, platform)
    }

    enum Step {
        Start,
        FixedUpdate { time: f32 },
        Event(RuleEvent),
    }

    /// Runs one step and applies visibility changes, as the game does, so later steps see them.
    fn run_step(rules: &mut dyn GameRules, world: &mut World, step: &Step) -> Vec<RuleCommand> {
        let input = InputSnapshot::default();
        let actions = ActionSet::default();
        let time_seconds = match step {
            Step::FixedUpdate { time } => *time,
            _ => 5.0,
        };
        let mut commands = Vec::new();
        let view = RulesView { world, input: &input, action_set: &actions, time_seconds };
        match step {
            Step::Start => rules.on_start(&view, &mut commands),
            Step::FixedUpdate { .. } => rules.on_fixed_update(&view, 1.0 / 60.0, &mut commands),
            Step::Event(event) => rules.on_event(&view, *event, &mut commands),
        }
        .unwrap();
        for command in &commands {
            if let RuleCommand::SetEntityVisible { entity, visible } = command
                && let Some(renderer) = world.get_mut(*entity).and_then(|data| data.mesh_renderer.as_mut())
            {
                renderer.visible = *visible;
            }
        }
        commands.retain(|command| !matches!(command, RuleCommand::Log { .. }));
        commands
    }

    /// The Nyx rules and the Rust rules must behave the same: feed both the same world and
    /// events and compare every command they produce (log lines aside).
    #[test]
    fn nyx_rules_match_the_native_rules() {
        let (mut native_world, gems, platform) = small_level();
        let (mut scripted_world, _, _) = small_level();
        let mut native = NativeRules::default();
        let mut scripted = ScriptedRules::load(&find_script()).expect("the script loads");

        let mut steps = vec![Step::Start];
        steps.extend([0.5f32, 1.5, 3.0, 4.75].map(|time| Step::FixedUpdate { time }));
        steps.extend(
            [
                RuleEvent::TriggerEntered { trigger: gems[1] },
                RuleEvent::TriggerEntered { trigger: gems[1] },
                RuleEvent::TriggerEntered { trigger: platform },
                RuleEvent::PlayerJumped,
                RuleEvent::PlayerLanded { speed: 2.0 },
                RuleEvent::PlayerLanded { speed: 9.0 },
                RuleEvent::TriggerEntered { trigger: gems[0] },
                RuleEvent::TriggerEntered { trigger: gems[2] },
                RuleEvent::PlayerFell,
                RuleEvent::ResetPressed,
            ]
            .map(Step::Event),
        );
        for (index, step) in steps.iter().enumerate() {
            let native_commands = run_step(&mut native, &mut native_world, step);
            let scripted_commands = run_step(&mut scripted, &mut scripted_world, step);
            assert_eq!(
                native_commands.len(),
                scripted_commands.len(),
                "step {index}: {native_commands:?} vs {scripted_commands:?}"
            );
            for (native_command, scripted_command) in native_commands.iter().zip(&scripted_commands) {
                match (native_command, scripted_command) {
                    (
                        RuleCommand::SetEntityPosition { entity: a, position: p },
                        RuleCommand::SetEntityPosition { entity: b, position: q },
                    ) => assert!(a == b && p.abs_diff_eq(*q, 1e-4), "step {index}: {p:?} vs {q:?}"),
                    _ => assert_eq!(native_command, scripted_command, "step {index}"),
                }
            }
        }
    }
}
