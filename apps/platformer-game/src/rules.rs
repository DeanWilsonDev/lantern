//! The seam between the engine-side game loop and the game's rules.
//!
//! Rules decide *what happens*: platforms follow paths, touching a gem collects it, falling
//! out of the world resets the player. The Rust side decides *how things move*: the character
//! controller, collision and transforms. Rules talk to the game only through this module:
//!
//! - they read a `RulesView` (time, input, entity positions);
//! - they write `RuleCommand`s, which the game applies in order right after the hook returns;
//! - the game tells them about `RuleEvent`s (pickups, falls, jumps) after physics.
//!
//! Two implementations exist: `NativeRules` below (plain Rust, used when scripting is
//! disabled, fails to load, or `LANTERN_RULES=native`) and the Nyx-scripted rules in
//! `scripted_rules.rs` (the default). Both see exactly the same view
//! and produce the same commands, which is what makes the scripting layer swappable.

use glam::Vec3;
use lantern_core::input::{ActionSet, InputSnapshot};
use lantern_engine::world::{EntityId, World};

/// What rules may ask the game to do. Applied in order after each hook.
#[derive(Clone, Debug, PartialEq)]
pub enum RuleCommand {
    SetEntityPosition {
        entity: EntityId,
        position: Vec3,
    },
    SetEntityVisible {
        entity: EntityId,
        visible: bool,
    },
    PlaySound {
        name: String,
        volume: f32,
    },
    /// Puts the player back at the spawn point, at rest.
    ResetPlayer,
    /// The short status line at the top of the screen (for example "Gems 3 / 6").
    SetStatus {
        text: String,
    },
    /// A large message in the middle of the screen for a few seconds.
    ShowMessage {
        text: String,
        seconds: f32,
    },
    #[cfg_attr(not(feature = "nyx-scripting"), expect(dead_code, reason = "only scripts log"))]
    Log {
        text: String,
    },
}

/// Things that happened in Rust that rules may react to, delivered after physics.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RuleEvent {
    /// The player entered a trigger (gems are triggers).
    TriggerEntered {
        trigger: EntityId,
    },
    /// The player fell below the level's kill height.
    PlayerFell,
    PlayerJumped,
    PlayerLanded {
        speed: f32,
    },
    /// The reset action was pressed.
    ResetPressed,
}

/// Read-only state rules can query during a hook.
#[cfg_attr(not(feature = "nyx-scripting"), expect(dead_code, reason = "the native rules need less than scripts"))]
pub struct RulesView<'a> {
    pub world: &'a World,
    pub input: &'a InputSnapshot,
    pub action_set: &'a ActionSet,
    /// Simulation time since the level started (fixed steps × step length).
    pub time_seconds: f32,
}

#[cfg_attr(not(feature = "nyx-scripting"), expect(dead_code, reason = "the native rules need less than scripts"))]
impl RulesView<'_> {
    pub fn find_entity(&self, name: &str) -> Option<EntityId> {
        self.world.find_by_name(name)
    }

    pub fn entity_position(&self, entity: EntityId) -> Option<Vec3> {
        self.world.get(entity).map(|data| data.local.translation)
    }

    /// Whether the named action was pressed this step. Unknown names are `false`.
    pub fn action_pressed(&self, name: &str) -> bool {
        self.action_set.find(name).is_some_and(|action| self.input.pressed(action))
    }

    pub fn action_held(&self, name: &str) -> bool {
        self.action_set.find(name).is_some_and(|action| self.input.held(action))
    }
}

/// A rules error: which hook failed and why (with file and line for scripts).
#[derive(Clone, Debug)]
pub struct RulesError {
    pub hook: &'static str,
    pub message: String,
}

impl std::fmt::Display for RulesError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}: {}", self.hook, self.message)
    }
}

/// The hooks the game calls. A failing hook is reported and the game keeps running.
pub trait GameRules {
    /// A short name for the HUD ("native" or the script file).
    fn description(&self) -> String;
    /// When the level starts (and after a full restart).
    fn on_start(&mut self, view: &RulesView, commands: &mut Vec<RuleCommand>) -> Result<(), RulesError>;
    /// Every fixed step, before physics.
    fn on_fixed_update(&mut self, view: &RulesView, dt: f32, commands: &mut Vec<RuleCommand>)
    -> Result<(), RulesError>;
    /// After physics, once per event, in the order they happened.
    fn on_event(
        &mut self,
        view: &RulesView,
        event: RuleEvent,
        commands: &mut Vec<RuleCommand>,
    ) -> Result<(), RulesError>;
    /// When the level is left.
    fn on_exit(&mut self, view: &RulesView, commands: &mut Vec<RuleCommand>) -> Result<(), RulesError>;
    /// Re-reads the rules from disk (F5). `Ok(true)` if they changed; the game then calls
    /// `on_start` again. On error the old rules stay in place.
    fn reload(&mut self) -> Result<bool, String> {
        Ok(false)
    }
}

/// The moving platform's path: back and forth along -Z.
pub const PLATFORM_TRAVEL: f32 = 6.0;
pub const PLATFORM_PERIOD_SECONDS: f32 = 6.0;

/// Gem count and platform path in Rust. `scripts/platformer.nyx` is the same logic in Nyx.
#[derive(Default)]
pub struct NativeRules {
    gems: Vec<EntityId>,
    platform: Option<(EntityId, Vec3)>,
    collected: u32,
}

impl NativeRules {
    fn status(&self) -> RuleCommand {
        RuleCommand::SetStatus { text: format!("Gems {} / {}", self.collected, self.gems.len()) }
    }

    fn restart(&mut self, commands: &mut Vec<RuleCommand>) {
        self.collected = 0;
        for &gem in &self.gems {
            commands.push(RuleCommand::SetEntityVisible { entity: gem, visible: true });
        }
        commands.push(self.status());
    }
}

/// Smooth back-and-forth between 0 and 1 with the given period (a cosine ease).
pub fn ping_pong(time_seconds: f32, period_seconds: f32) -> f32 {
    0.5 - 0.5 * (time_seconds / period_seconds * std::f32::consts::TAU).cos()
}

impl GameRules for NativeRules {
    fn description(&self) -> String {
        "native (Rust)".to_string()
    }

    fn on_start(&mut self, view: &RulesView, commands: &mut Vec<RuleCommand>) -> Result<(), RulesError> {
        self.gems = (0..).map_while(|index| view.find_entity(&format!("gem_{index}"))).collect();
        self.platform = view
            .find_entity("moving_platform")
            .and_then(|entity| view.entity_position(entity).map(|start| (entity, start)));
        self.restart(commands);
        commands.push(RuleCommand::ShowMessage { text: "Collect every gem!".to_string(), seconds: 3.0 });
        Ok(())
    }

    fn on_fixed_update(
        &mut self,
        view: &RulesView,
        _dt: f32,
        commands: &mut Vec<RuleCommand>,
    ) -> Result<(), RulesError> {
        if let Some((platform, start)) = self.platform {
            let offset = ping_pong(view.time_seconds, PLATFORM_PERIOD_SECONDS) * PLATFORM_TRAVEL;
            commands.push(RuleCommand::SetEntityPosition { entity: platform, position: start - Vec3::Z * offset });
        }
        Ok(())
    }

    fn on_event(
        &mut self,
        view: &RulesView,
        event: RuleEvent,
        commands: &mut Vec<RuleCommand>,
    ) -> Result<(), RulesError> {
        match event {
            RuleEvent::TriggerEntered { trigger } if self.gems.contains(&trigger) => {
                let visible =
                    view.world.get(trigger).and_then(|data| data.mesh_renderer).is_some_and(|mesh| mesh.visible);
                if !visible {
                    return Ok(());
                }
                self.collected += 1;
                commands.push(RuleCommand::SetEntityVisible { entity: trigger, visible: false });
                commands.push(RuleCommand::PlaySound { name: "pickup".to_string(), volume: 1.0 });
                commands.push(self.status());
                if self.collected as usize == self.gems.len() {
                    commands.push(RuleCommand::ShowMessage { text: "All gems collected!".to_string(), seconds: 4.0 });
                }
            }
            RuleEvent::PlayerFell => {
                commands.push(RuleCommand::PlaySound { name: "reset".to_string(), volume: 1.0 });
                commands.push(RuleCommand::ResetPlayer);
            }
            RuleEvent::ResetPressed => {
                commands.push(RuleCommand::PlaySound { name: "reset".to_string(), volume: 1.0 });
                commands.push(RuleCommand::ResetPlayer);
                self.restart(commands);
            }
            RuleEvent::PlayerJumped => commands.push(RuleCommand::PlaySound { name: "jump".to_string(), volume: 0.8 }),
            RuleEvent::PlayerLanded { speed } if speed > 4.0 => {
                commands.push(RuleCommand::PlaySound { name: "land".to_string(), volume: (speed / 15.0).min(1.0) });
            }
            _ => {}
        }
        Ok(())
    }

    fn on_exit(&mut self, _view: &RulesView, _commands: &mut Vec<RuleCommand>) -> Result<(), RulesError> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ping_pong_starts_and_ends_at_zero_and_peaks_halfway() {
        assert!(ping_pong(0.0, 6.0).abs() < 1e-6);
        assert!((ping_pong(3.0, 6.0) - 1.0).abs() < 1e-6);
        assert!(ping_pong(6.0, 6.0).abs() < 1e-5);
    }
}
