//! `GameUi`: the game's UI as Chaos components (`.chaos`) styled by Cosmos (`.cosmos`), drawn
//! by `lantern-chaos-ui` (decision D-043).
//!
//! The game mounts its root component once (`GameUi:::Mount`). The component gets the game's
//! `Application` object as its argument and reads the game's state from it; fields declared
//! `@signal` re-render the parts of the UI that read them when the game writes them. Buttons
//! (elements with `onPress` or `onRelease`) get keyboard and gamepad focus from the actions the game names in
//! `GameUi:::SetNavigation`; the engine calls a button's `onFocus` when focus moves to it and
//! its `onRelease` when it is confirmed. The engine draws the UI after `OnRender`, on top of the
//! game's own `Overlay` drawing.

use std::cell::RefCell;
use std::rc::Rc;

use lantern_chaos_ui::ChaosUi;
use nyx::{Runtime, Value};

use super::{invalid, string_argument};

/// The three actions that move menu focus and press the focused button.
#[derive(Clone, Debug)]
pub struct NavigationActions {
    pub previous: String,
    pub next: String,
    pub confirm: String,
}

/// What `GameUi` needs between calls. Emptied before the runtime is dropped: the Chaos UI and
/// the application value both keep the runtime alive.
#[derive(Default)]
pub struct GameUiState {
    pub chaos: Option<Rc<ChaosUi>>,
    /// The game's `Application` object, passed to every mounted component.
    pub application: Option<Value>,
    pub navigation: Option<NavigationActions>,
}

pub type SharedGameUi = Rc<RefCell<GameUiState>>;

struct GameUiApi;

pub fn register(runtime: &Runtime, game_ui: &SharedGameUi) -> nyx::Result<()> {
    let mount = game_ui.clone();
    let unmount = game_ui.clone();
    let navigation = game_ui.clone();
    runtime
        .register_type::<GameUiApi>("GameUi")
        // Mounts `component` from `file` (relative to the game directory) with the game's
        // Application object as its argument: `Name :: (game~ MyGame) -> lantern:::ui:::Element`.
        // Returns a number for Unmount. Call it from OnStart or later.
        .static_method("Mount", "(file: string, component: string) -> i32", move |context, _, arguments| {
            let file = string_argument(arguments, 0)?;
            let component = string_argument(arguments, 1)?;
            // Taken out of the shared state before mounting: mounting runs the component's Nyx,
            // which may call back into the engine.
            let (chaos, application) = {
                let state = mount.borrow();
                (state.chaos.clone(), state.application.clone())
            };
            let chaos = chaos.ok_or_else(|| invalid(context, "GameUi isn't available: Chaos didn't start"))?;
            let application = application
                .ok_or_else(|| invalid(context, "GameUi:::Mount needs the game to be loaded; call it from OnStart"))?;
            let number = chaos.mount(&file, &component, &[application]).map_err(|error| invalid(context, error))?;
            Ok(Some(context.make_i32(number as i32)?))
        })
        .static_method("Unmount", "(mount: i32) -> void", move |context, _, arguments| {
            let number = super::i32_argument(arguments, 0)?;
            let chaos = unmount.borrow().chaos.clone();
            let removed = chaos.is_some_and(|chaos| chaos.unmount(number as u32));
            if !removed {
                return Err(invalid(context, format!("GameUi:::Unmount: nothing is mounted as {number}")));
            }
            Ok(None)
        })
        // The actions that move focus to the previous and next button and press the focused one,
        // by name (as registered with Input:::DefineButton).
        .static_method(
            "SetNavigation",
            "(previous: string, next: string, confirm: string) -> void",
            move |_context, _, arguments| {
                navigation.borrow_mut().navigation = Some(NavigationActions {
                    previous: string_argument(arguments, 0)?,
                    next: string_argument(arguments, 1)?,
                    confirm: string_argument(arguments, 2)?,
                });
                Ok(None)
            },
        )
        .register()?;
    Ok(())
}
