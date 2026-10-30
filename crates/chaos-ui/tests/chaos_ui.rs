//! The bridge end to end on a real Nyx runtime: a game object passed to a component, a signal
//! written by game code re-rendering it, Cosmos styles, layout, and menu events writing back.

use std::path::{Path, PathBuf};

use glam::Vec2;
use lantern_chaos_ui::{ChaosUi, Navigation, NodeKind, Pointer};
use lantern_core::color::Color;
use lantern_renderer_3d::render_world::Overlay;

struct TempGame {
    root: PathBuf,
}

impl TempGame {
    fn new(name: &str) -> TempGame {
        let root = std::env::temp_dir().join(format!("lantern-chaos-ui-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("ui")).unwrap();
        TempGame { root }
    }

    fn write(&self, relative: &str, text: &str) -> &TempGame {
        std::fs::write(self.root.join(relative), text).unwrap();
        self
    }

    fn path(&self) -> &Path {
        &self.root
    }
}

impl Drop for TempGame {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

const MODEL: &str = r#"
MenuModel :: class {
public:
    @signal title~ string = "LANTERN";
    @signal chosen~ string = "";
    @signal moves~ i32 = 0;
}
"#;

const GAME: &str = r#"
MenuModel :: import MenuModel;

Game :: class : Application {
public:
    model~ MenuModel = new MenuModel();
    Run :: () -> void { }
    Model~: () -> MenuModel { return model; }
    Rename~: (title: string) -> void { model.title = title; }
    Chosen~: () -> string { return model.chosen; }
    Moves~: () -> i32 { return model.moves; }
}
"#;

const MENU: &str = r#"
MenuModel :: import MenuModel;

Menu :: (model~ MenuModel) -> lantern:::ui:::Element {
    return (
        <frame class="menu">
            <slot>
                !{(() -> lantern:::ui:::Element {
                    return (<text class="title">{model.title}</text>);
                })()}
            </slot>
            <frame class="button" onFocus={() -> void { model.moves += 1; }} onRelease={() -> void { model.chosen = "play"; }}>
                <text>Play</text>
            </frame>
            <frame class="button" onFocus={() -> void { model.moves += 1; }} onRelease={() -> void { model.chosen = "quit"; }}>
                <text>Quit</text>
            </frame>
        </frame>
    );
}
"#;

const STYLES: &str = r#"
.menu {
    display: stack;
    flex-direction: column;
    padding: 10px;
    gap: 4px;
    background-color: #102030;
}

.title {
    font-size: 32px;
    color: #FFD24D;
}

.button {
    padding: 2px 6px;
    background-color: #00000080;
}

.button:hover {
    background-color: #FFD24D;
}
"#;

fn setup(game: &TempGame) -> (nyx::Runtime, ChaosUi, nyx::Program) {
    game.write("ui/menu-model.nyx", MODEL)
        .write("main.nyx", GAME)
        .write("ui/Menu.chaos", MENU)
        .write("ui/menu.cosmos", STYLES);
    let runtime = nyx::Runtime::new().unwrap();
    runtime.add_import_search_path(game.path()).unwrap();
    runtime.add_import_search_path(game.path().join("ui")).unwrap();
    let ui = ChaosUi::new(&runtime, game.path(), &game.path().join("ui")).unwrap();
    let program = runtime.mount_file(game.path().join("main.nyx"), "Game").unwrap();
    (runtime, ui, program)
}

fn model_of(program: &nyx::Program) -> nyx::Value {
    program.call_application_method("Model", &[]).unwrap().expect("Model returns the menu model")
}

fn call(program: &nyx::Program, method: &str, args: &[nyx::Value]) -> Option<nyx::Value> {
    program.call_application_method(method, args).unwrap()
}

fn texts(ui: &ChaosUi) -> Vec<String> {
    ui.nodes().into_iter().filter(|node| node.kind == NodeKind::Text).map(|node| node.text).collect()
}

#[test]
fn a_component_mounts_on_the_game_runtime_and_follows_its_signals() {
    let game = TempGame::new("signals");
    let (runtime, ui, program) = setup(&game);
    ui.mount("ui/Menu.chaos", "Menu", &[model_of(&program)]).unwrap();
    ui.update();
    assert_eq!(texts(&ui), ["LANTERN", "Play", "Quit"], "messages: {:?}", ui.take_messages());

    // Game code writes the signal; the slot re-renders on the next update.
    call(&program, "Rename", &[runtime.make_string("PAUSED").unwrap()]);
    ui.update();
    assert_eq!(texts(&ui), ["PAUSED", "Play", "Quit"]);
    assert!(ui.take_messages().is_empty());
}

#[test]
fn cosmos_styles_reach_the_nodes_and_the_layout() {
    let game = TempGame::new("styles");
    let (_runtime, ui, program) = setup(&game);
    ui.mount("ui/Menu.chaos", "Menu", &[model_of(&program)]).unwrap();
    ui.update();
    let nodes = ui.nodes();
    let menu = &nodes[0];
    assert_eq!(
        menu.style.background,
        Some(Color::rgba(0x10 as f32 / 255.0, 0x20 as f32 / 255.0, 0x30 as f32 / 255.0, 1.0))
    );
    let title = nodes.iter().find(|node| node.text == "LANTERN").unwrap();
    assert_eq!(title.style.font_size, Some(32.0));
    let button = nodes.iter().find(|node| node.focusable).unwrap();
    assert!(button.hover.is_some(), "the :hover block resolves");
    // `color` inherits: "Play" has no color of its own.
    let play = nodes.iter().find(|node| node.text == "Play").unwrap();
    assert_eq!(play.style.text_color, None);

    let mut overlay = Overlay::default();
    let rects = ui.draw(&mut overlay, Vec2::new(800.0, 600.0), 1.0);
    assert!(!overlay.vertices.is_empty());
    // Padding 10 around a column: the title starts 10 px in; it is 32 px tall (font scale 4).
    let title_index = nodes.iter().position(|node| node.text == "LANTERN").unwrap();
    assert_eq!(rects[title_index].min, Vec2::new(10.0, 10.0));
    assert_eq!(rects[title_index].size().y, 32.0);
    // The buttons stretch across the column (align-items defaults to stretch).
    let buttons: Vec<usize> = (0..nodes.len()).filter(|&index| nodes[index].focusable).collect();
    assert_eq!(rects[buttons[0]].size().x, rects[0].size().x - 20.0);
    // 4 px gap between the title and the first button.
    assert_eq!(rects[buttons[0]].min.y, rects[title_index].max.y + 4.0);
}

#[test]
fn navigation_focuses_buttons_and_confirm_calls_on_release() {
    let game = TempGame::new("events");
    let (_runtime, ui, program) = setup(&game);
    ui.mount("ui/Menu.chaos", "Menu", &[model_of(&program)]).unwrap();
    ui.update();

    // First frame: focus lands on the first button without an onFocus.
    ui.navigate(Navigation::default());
    let first = ui.focused().unwrap();
    assert_eq!(call(&program, "Moves", &[]).unwrap().as_i32().unwrap(), 0);

    ui.navigate(Navigation { next: true, ..Navigation::default() });
    assert_ne!(ui.focused(), Some(first));
    assert_eq!(call(&program, "Moves", &[]).unwrap().as_i32().unwrap(), 1);

    ui.navigate(Navigation { confirm: true, ..Navigation::default() });
    assert_eq!(call(&program, "Chosen", &[]).unwrap().as_str().unwrap(), "quit");

    // Wraps around.
    ui.navigate(Navigation { next: true, ..Navigation::default() });
    assert_eq!(ui.focused(), Some(first));
    assert!(ui.take_messages().is_empty());
}

#[test]
fn a_broken_component_reports_why() {
    let game = TempGame::new("broken");
    let (_runtime, ui, program) = setup(&game);
    game.write("ui/Broken.chaos", "Broken :: () -> lantern:::ui:::Element {\n    return (<frame>);\n}\n");
    let error = ui.mount("ui/Broken.chaos", "Broken", &[]).unwrap_err();
    assert!(error.contains("Broken"), "{error}");
    let _ = program;
}

fn pointer_at(position: Vec2, pressed: bool, released: bool, held: bool) -> Navigation {
    Navigation { pointer: Some(Pointer { position, pressed, released, held }), ..Navigation::default() }
}

fn button_centres(ui: &ChaosUi) -> Vec<(u64, Vec2)> {
    let mut overlay = Overlay::default();
    let rects = ui.draw(&mut overlay, Vec2::new(800.0, 600.0), 1.0);
    ui.nodes()
        .iter()
        .zip(&rects)
        .filter(|(node, _)| node.focusable)
        .map(|(node, rect)| (node.id, (rect.min + rect.max) * 0.5))
        .collect()
}

#[test]
fn the_mouse_focuses_a_button_it_moves_over_and_clicks_it() {
    let game = TempGame::new("mouse-click");
    let (_runtime, ui, program) = setup(&game);
    ui.mount("ui/Menu.chaos", "Menu", &[model_of(&program)]).unwrap();
    ui.update();
    let buttons = button_centres(&ui);
    let (play, quit) = (buttons[0], buttons[1]);

    ui.navigate(pointer_at(quit.1, false, false, false));
    assert_eq!(ui.focused(), Some(quit.0));
    assert_eq!(call(&program, "Moves", &[]).unwrap().as_i32().unwrap(), 1);

    // A quick click (down and up in one frame) on Play focuses and chooses it.
    ui.navigate(pointer_at(play.1, true, true, false));
    assert_eq!(ui.focused(), Some(play.0));
    assert_eq!(call(&program, "Chosen", &[]).unwrap().as_str().unwrap(), "play");
    assert!(ui.take_messages().is_empty());
}

#[test]
fn releasing_the_mouse_off_the_pressed_button_cancels_the_click() {
    let game = TempGame::new("mouse-cancel");
    let (_runtime, ui, program) = setup(&game);
    ui.mount("ui/Menu.chaos", "Menu", &[model_of(&program)]).unwrap();
    ui.update();
    let buttons = button_centres(&ui);
    let (play, quit) = (buttons[0], buttons[1]);

    ui.navigate(pointer_at(play.1, true, false, true));
    ui.navigate(pointer_at(quit.1, false, true, false));
    assert_eq!(call(&program, "Chosen", &[]).unwrap().as_str().unwrap(), "");

    // Down and up on the same button, over two frames, clicks it.
    ui.navigate(pointer_at(quit.1, true, false, true));
    ui.navigate(pointer_at(quit.1, false, false, true));
    assert_eq!(call(&program, "Chosen", &[]).unwrap().as_str().unwrap(), "");
    ui.navigate(pointer_at(quit.1, false, true, false));
    assert_eq!(call(&program, "Chosen", &[]).unwrap().as_str().unwrap(), "quit");
}

#[test]
fn a_still_mouse_leaves_keyboard_focus_alone() {
    let game = TempGame::new("mouse-still");
    let (_runtime, ui, program) = setup(&game);
    ui.mount("ui/Menu.chaos", "Menu", &[model_of(&program)]).unwrap();
    ui.update();
    let buttons = button_centres(&ui);
    let (play, quit) = (buttons[0], buttons[1]);

    ui.navigate(pointer_at(play.1, false, false, false));
    assert_eq!(ui.focused(), Some(play.0));
    ui.navigate(Navigation { next: true, ..pointer_at(play.1, false, false, false) });
    assert_eq!(ui.focused(), Some(quit.0), "the keyboard moved focus and the mouse didn't move");
    ui.navigate(pointer_at(play.1, false, false, false));
    assert_eq!(ui.focused(), Some(quit.0));
}
