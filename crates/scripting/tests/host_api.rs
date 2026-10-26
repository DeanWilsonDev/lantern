//! The engine API as seen from Nyx: reads come from the snapshot, writes become commands,
//! and errors (analysis or run-time) come back as `ScriptError`s without crashing.

use glam::Vec3;
use lantern_scripting::{HookArgument, ScriptCommand, ScriptEntity, ScriptHook, ScriptHost, ScriptSnapshot};

fn snapshot() -> ScriptSnapshot {
    let mut snapshot = ScriptSnapshot { time_seconds: 2.5, ..Default::default() };
    snapshot.entity_ids_by_name.insert("platform".to_string(), 7);
    snapshot.entities.insert(7, ScriptEntity { position: Vec3::new(1.0, 2.0, 3.0), visible: true });
    snapshot.pressed_actions.insert("jump".to_string());
    snapshot
}

const API_SCRIPT: &str = r#"
Probe :: class : Application {
public:
    steps~ i32 = 0;
    Run :: () -> void { }
    OnStart~: () -> void {
        Game:::Log("started");
        Game:::SetStatus(`time ${Game:::Time()}`);
    }
    OnFixedUpdate~: (dt: f32) -> void {
        steps += 1;
        platform := Entity:::Find("platform");
        start := Entity:::Position(platform);
        Entity:::SetPosition(platform, Vector:::New(start.X(), start.Y() + dt, start.Z()));
        moved := Entity:::Position(platform);
        if Input:::IsPressed("jump") {
            Sound:::Play("jump", moved.Y());
        }
        Entity:::SetVisible(platform, false);
    }
    OnTriggerEnter~: (entity: i64) -> void {
        Game:::ShowMessage(`touched ${entity:d}`, 1.5f);
        Game:::ResetPlayer();
    }
}
"#;

#[test]
fn hooks_read_the_snapshot_and_write_commands() {
    let mut host = ScriptHost::load_source(API_SCRIPT, "probe.nyx", "Probe").unwrap();
    assert!(host.defines(ScriptHook::OnStart) && !host.defines(ScriptHook::OnExit));

    let (commands, result) = host.call(ScriptHook::OnStart, snapshot(), &[]);
    result.unwrap();
    assert_eq!(commands[0], ScriptCommand::Log { text: "started".to_string() });
    assert!(matches!(&commands[1], ScriptCommand::SetStatus { text } if text.starts_with("time 2.5")), "{commands:?}");

    let (commands, result) = host.call(ScriptHook::OnFixedUpdate, snapshot(), &[HookArgument::F32(0.5)]);
    result.unwrap();
    assert_eq!(
        commands,
        vec![
            ScriptCommand::SetEntityPosition { entity: 7, position: Vec3::new(1.0, 2.5, 3.0) },
            // The read after the write sees the write.
            ScriptCommand::PlaySound { name: "jump".to_string(), volume: 2.5 },
            ScriptCommand::SetEntityVisible { entity: 7, visible: false },
        ]
    );

    let (commands, result) = host.call(ScriptHook::OnTriggerEnter, snapshot(), &[HookArgument::I64(7)]);
    result.unwrap();
    assert_eq!(
        commands,
        vec![ScriptCommand::ShowMessage { text: "touched 7".to_string(), seconds: 1.5 }, ScriptCommand::ResetPlayer]
    );

    // Missing hooks are skipped quietly.
    let (commands, result) = host.call(ScriptHook::OnExit, snapshot(), &[]);
    assert!(commands.is_empty() && result.is_ok());
}

#[test]
fn unknown_entities_throw_in_the_script_and_the_host_reports_it() {
    const SCRIPT: &str = r#"
Broken :: class : Application {
public:
    Run :: () -> void { }
    OnFixedUpdate~: (dt: f32) -> void {
        Game:::Log("before");
        missing := Entity:::Find("no_such_entity");
        Entity:::SetVisible(missing, false);
        Game:::Log("after");
    }
}
"#;
    let mut host = ScriptHost::load_source(SCRIPT, "broken.nyx", "Broken").unwrap();
    for _ in 0..3 {
        let (commands, result) = host.call(ScriptHook::OnFixedUpdate, snapshot(), &[HookArgument::F32(0.1)]);
        let error = result.expect_err("the hook should fail");
        assert!(error.message.contains("no_such_entity"), "{}", error.message);
        assert_eq!(commands, vec![ScriptCommand::Log { text: "before".to_string() }]);
    }
}

#[test]
fn analysis_errors_are_rendered_and_nothing_runs() {
    const SCRIPT: &str = r#"
Typo :: class : Application {
public:
    Run :: () -> void { }
    OnStart~: () -> void {
        count: i32 = 1.5f;
    }
}
"#;
    let error = ScriptHost::load_source(SCRIPT, "typo.nyx", "Typo").err().expect("analysis should fail");
    assert!(error.message.contains("typo.nyx"), "{}", error.message);
    assert!(error.message.contains("NYX"), "{}", error.message);
}

/// The demo's own script analyses cleanly and runs its hooks against a small fake level.
#[test]
fn the_platformer_script_loads_and_runs() {
    let path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../apps/platformer-game/scripts/platformer.nyx");
    let mut host = match ScriptHost::load(&path, "Platformer") {
        Ok(host) => host,
        Err(error) => panic!("{error}"),
    };
    let mut level = ScriptSnapshot::default();
    for (index, name) in ["gem_0", "gem_1", "moving_platform"].iter().enumerate() {
        level.entity_ids_by_name.insert(name.to_string(), index as i64 + 100);
        level.entities.insert(index as i64 + 100, ScriptEntity { position: Vec3::new(0.0, 1.8, -21.0), visible: true });
    }
    let (commands, result) = host.call(ScriptHook::OnStart, level.clone(), &[]);
    result.unwrap();
    assert!(commands.contains(&ScriptCommand::SetStatus { text: "Gems 0 / 2".to_string() }), "{commands:?}");

    let mut halfway = level.clone();
    halfway.time_seconds = 3.0;
    let (commands, result) = host.call(ScriptHook::OnFixedUpdate, halfway, &[HookArgument::F32(1.0 / 60.0)]);
    result.unwrap();
    match &commands[..] {
        [ScriptCommand::SetEntityPosition { entity: 102, position }] => {
            assert!((position.z - -27.0).abs() < 1e-4, "{position:?}");
        }
        other => panic!("{other:?}"),
    }

    let (commands, result) = host.call(ScriptHook::OnTriggerEnter, level.clone(), &[HookArgument::I64(100)]);
    result.unwrap();
    assert!(commands.contains(&ScriptCommand::SetStatus { text: "Gems 1 / 2".to_string() }), "{commands:?}");
    // The platform is not a gem: nothing happens.
    let (commands, result) = host.call(ScriptHook::OnTriggerEnter, level.clone(), &[HookArgument::I64(102)]);
    assert!(commands.is_empty() && result.is_ok());
    let (commands, _) = host.call(ScriptHook::OnLand, level.clone(), &[HookArgument::F32(2.0)]);
    assert!(commands.is_empty(), "soft landings are silent");
    let (commands, _) = host.call(ScriptHook::OnLand, level, &[HookArgument::F32(30.0)]);
    assert_eq!(commands, vec![ScriptCommand::PlaySound { name: "land".to_string(), volume: 1.0 }]);
}
