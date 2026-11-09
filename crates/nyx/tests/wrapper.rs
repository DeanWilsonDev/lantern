use std::cell::{Cell, RefCell};
use std::rc::Rc;

use nyx::{CallbackError, Documentation, Kind, Receiver, Runtime, Severity, Status, Value, PANIC_VARIANT};

const EMPTY_APPLICATION: &str = r#"
Game :: class : Application {
public:
    Run :: () -> void { }
}
"#;

fn mount(runtime: &Runtime, functions: &str) -> nyx::Program {
    let source = format!("{functions}\n{EMPTY_APPLICATION}");
    match runtime.mount_source(&source, "test.nyx", "Game") {
        Ok(program) => program,
        Err(error) => panic!("{error}\n{}", runtime.render_diagnostics().unwrap_or_default()),
    }
}

fn call_i32(program: &nyx::Program, name: &str, args: &[Value]) -> nyx::Result<i32> {
    program.call_function(name, args)?.expect("the call returned a value").as_i32()
}

#[test]
fn reports_the_header_and_library_versions() {
    assert_eq!(nyx::capi_version(), nyx::HEADER_VERSION);
}

#[test]
fn primitive_values_round_trip() {
    let runtime = Runtime::new().unwrap();
    assert!(runtime.make_bool(true).unwrap().as_bool().unwrap());
    assert_eq!(runtime.make_i32(-7).unwrap().as_i32().unwrap(), -7);
    assert_eq!(runtime.make_i64(1 << 40).unwrap().as_i64().unwrap(), 1 << 40);
    assert_eq!(runtime.make_f32(2.5).unwrap().as_f32().unwrap(), 2.5);
    assert_eq!(runtime.make_f64(0.125).unwrap().as_f64().unwrap(), 0.125);
    assert_eq!(runtime.make_string("héllo").unwrap().as_str().unwrap(), "héllo");
    assert_eq!(runtime.make_string("").unwrap().as_str().unwrap(), "");
    assert!(runtime.make_null().unwrap().is_null());
}

#[test]
fn reading_the_wrong_kind_is_a_type_mismatch() {
    let runtime = Runtime::new().unwrap();
    let number = runtime.make_i32(3).unwrap();
    let error = number.array_length().unwrap_err();
    assert_eq!(error.status, Status::TypeMismatch);
    assert_eq!(error.message, "the value is i32, not array");
}

#[test]
fn arrays_built_in_rust_are_read_by_nyx_and_back() {
    let runtime = Runtime::new().unwrap();
    let program = mount(
        &runtime,
        "Total :: (items: Array<i32>) -> i32 { return items.reduce((total, item) -> total + item, 0); }\n\
         Doubled :: (items: Array<i32>) -> Array<i32> { return items.map((item) -> item * 2); }",
    );
    let items: Vec<Value> = [1, 2, 3, 4].iter().map(|number| runtime.make_i32(*number).unwrap()).collect();
    let array = runtime.make_array(&items).unwrap();
    assert_eq!(array.kind(), Kind::Array);
    assert_eq!(call_i32(&program, "Total", std::slice::from_ref(&array)).unwrap(), 10);

    let doubled = program.call_function("Doubled", &[array]).unwrap().unwrap();
    let numbers: Vec<i32> = doubled.array_items().unwrap().iter().map(|item| item.as_i32().unwrap()).collect();
    assert_eq!(numbers, [2, 4, 6, 8]);
    assert_eq!(doubled.array_get(9).unwrap_err().status, Status::OutOfRange);
}

#[test]
fn error_values_carry_their_variant_and_message() {
    let runtime = Runtime::new().unwrap();
    let error = runtime.make_error("Error::Unknown", "despawned").unwrap();
    assert_eq!(error.error_variant().unwrap(), "Error::Unknown");
    assert_eq!(error.error_message().unwrap(), "despawned");
}

#[test]
fn a_refused_script_reports_its_diagnostics() {
    let runtime = Runtime::new().unwrap();
    let error = runtime
        .mount_source("Broken :: () -> i32 { return missing; }\n", "broken.nyx", "Game")
        .err()
        .expect("the script is refused");
    assert_eq!(error.status, Status::Refused);
    let diagnostics = runtime.diagnostics();
    assert!(!diagnostics.is_empty());
    let first = &diagnostics[0];
    assert_eq!(first.severity, Severity::Error);
    assert!(first.code.starts_with("NYX"), "code was {}", first.code);
    assert_eq!(first.path, "broken.nyx");
    assert_eq!(first.start_line, 1);
    assert!(runtime.render_diagnostics().unwrap().contains(&first.code));
}

#[test]
fn documented_host_function_reaches_the_host_surface_and_still_runs() {
    let runtime = Runtime::new().unwrap();
    let documentation = Documentation::new("Deals damage to the target.")
        .parameter("amount", "the damage to deal")
        .returns("the damage that landed")
        .failure("Unknown", "the target despawned")
        .example("Damage(3);");
    runtime
        .register_documented_function("Damage", "(amount: i32) -> i32", &documentation, |context, arguments| {
            Ok(Some(context.make_i32(arguments[0].as_i32()? + 1)?))
        })
        .unwrap();
    runtime.register_function("Plain", "() -> void", |_context, _arguments| Ok(None)).unwrap();
    let manifest = runtime.write_host_surface().unwrap();
    for expected in [
        "Deals damage to the target.",
        "the damage to deal",
        "the damage that landed",
        "the target despawned",
        "Damage(3);",
    ] {
        assert!(manifest.contains(expected), "the manifest should hold {expected:?}: {manifest}");
    }
    let program = mount(&runtime, "Run :: () -> i32 { return Damage(4); }");
    assert_eq!(call_i32(&program, "Run", &[]).unwrap(), 5);
}

#[test]
fn host_function_is_called_from_nyx() {
    let runtime = Runtime::new().unwrap();
    let lines = Rc::new(RefCell::new(Vec::new()));
    let sink = lines.clone();
    runtime
        .register_function("Log", "(line: string) -> void", move |_context, arguments| {
            sink.borrow_mut().push(arguments[0].as_str()?.to_string());
            Ok(None)
        })
        .unwrap();
    runtime
        .register_function("Scale", "(value: i32, factor: i32) -> i32", |context, arguments| {
            Ok(Some(context.make_i32(arguments[0].as_i32()? * arguments[1].as_i32()?)?))
        })
        .unwrap();
    let program = mount(&runtime, "Run :: () -> i32 { Log(\"scaling\"); return Scale(6, 7); }");
    assert_eq!(call_i32(&program, "Run", &[]).unwrap(), 42);
    assert_eq!(*lines.borrow(), ["scaling"]);
}

#[test]
fn the_host_surface_lists_registered_functions() {
    let runtime = Runtime::new().unwrap();
    runtime
        .register_function("Scale", "(value: i32, factor: i32) -> i32", |context, arguments| {
            Ok(Some(context.make_i32(arguments[0].as_i32()? * arguments[1].as_i32()?)?))
        })
        .unwrap();
    let manifest = runtime.write_host_surface().unwrap();
    assert!(manifest.contains("\"name\": \"Scale\""), "{manifest}");
    assert!(manifest.contains("\"signature\": \"(value: i32, factor: i32) -> i32\""), "{manifest}");
}

#[test]
fn a_removed_standard_namespace_refuses_its_import() {
    let runtime = Runtime::new().unwrap();
    let source = "Math :: import standard:::Math;\nHalf :: () -> f32 { return Math:::sqrt(0.25f); }\n";
    let program = mount(&runtime, source);
    assert!(program.call_function("Half", &[]).unwrap().is_some());

    runtime.remove_standard_namespace("Math").unwrap();
    let error = runtime
        .mount_source(&format!("{source}\n{EMPTY_APPLICATION}"), "removed.nyx", "Game")
        .err()
        .expect("the import is refused");
    assert_eq!(error.status, Status::Refused);
    assert_eq!(runtime.diagnostics()[0].code, "NYX0493");
    assert_eq!(runtime.remove_standard_namespace("Teleport").unwrap_err().status, Status::InvalidArgument);
}

#[test]
fn host_failure_reaches_the_host_as_a_nyx_failure() {
    let runtime = Runtime::new().unwrap();
    runtime
        .register_function("Fuel", "() -> i32", |context, _arguments| {
            Err(context.failure("Error::Unknown", "out of fuel"))
        })
        .unwrap();
    let program = mount(&runtime, "Burn :: () -> i32 { left := Fuel(); return left + 1; }");
    let error = program.call_function("Burn", &[]).unwrap_err();
    assert!(error.is_failure());
    let failure = error.failure.expect("a failure carries its error value");
    assert_eq!(failure.error_variant().unwrap(), "Error::Unknown");
    assert_eq!(failure.error_message().unwrap(), "out of fuel");
}

#[test]
fn a_failure_can_be_matched_inside_nyx() {
    let runtime = Runtime::new().unwrap();
    runtime
        .register_function("Fuel", "() -> i32", |context, _arguments| {
            Err(context.failure("Error::Unknown", "out of fuel"))
        })
        .unwrap();
    let program = mount(
        &runtime,
        "Gauge :: () -> string {\n\
             left := Fuel();\n\
             return when left {\n\
                 amount: i32 -> \"fuelled\",\n\
                 problem: Error -> \"empty\"\n\
             };\n\
         }",
    );
    let gauge = program.call_function("Gauge", &[]).unwrap().unwrap();
    assert_eq!(gauge.as_str().unwrap(), "empty");
}

#[test]
fn a_refusal_stops_the_call_with_its_message() {
    let runtime = Runtime::new().unwrap();
    runtime
        .register_function("Spawn", "() -> i32", |context, _arguments| Err(context.refusal("the world is full")))
        .unwrap();
    let program = mount(&runtime, "Run :: () -> i32 { return Spawn(); }");
    let error = program.call_function("Run", &[]).unwrap_err();
    assert_eq!(error.status, Status::Runtime);
    assert!(error.message.contains("the world is full"), "message was {}", error.message);
}

#[test]
fn a_panicking_callback_becomes_a_failure_and_the_runtime_stays_usable() {
    let runtime = Runtime::new().unwrap();
    let calls = Rc::new(Cell::new(0));
    let counter = calls.clone();
    runtime
        .register_function("Risky", "(explode: bool) -> i32", move |context, arguments| {
            counter.set(counter.get() + 1);
            if arguments[0].as_bool()? {
                panic!("the reactor melted");
            }
            Ok(Some(context.make_i32(5)?))
        })
        .unwrap();
    let program = mount(&runtime, "Run :: (explode: bool) -> i32 { return Risky(explode); }");

    let previous_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let error = program.call_function("Run", &[runtime.make_bool(true).unwrap()]).unwrap_err();
    std::panic::set_hook(previous_hook);

    assert!(error.is_failure());
    let failure = error.failure.unwrap();
    assert_eq!(failure.error_variant().unwrap(), PANIC_VARIANT);
    assert_eq!(failure.error_message().unwrap(), "the reactor melted");

    assert_eq!(call_i32(&program, "Run", &[runtime.make_bool(false).unwrap()]).unwrap(), 5);
    assert_eq!(calls.get(), 2);
}

#[test]
fn an_unreadable_signature_is_refused_and_its_closure_dropped() {
    let runtime = Runtime::new().unwrap();
    let witness = Rc::new(());
    let captured = witness.clone();
    let error = runtime
        .register_function("Damage", "(amount: ) ->", move |_context, _arguments| {
            let _ = &captured;
            Ok(None)
        })
        .unwrap_err();
    assert_eq!(error.status, Status::InvalidArgument);
    assert_eq!(error.message, "'Damage' has an unreadable signature '(amount: ) ->'");
    assert_eq!(Rc::strong_count(&witness), 1);
}

#[test]
fn closures_are_dropped_when_the_runtime_goes() {
    let witness = Rc::new(());
    {
        let runtime = Runtime::new().unwrap();
        let captured = witness.clone();
        runtime
            .register_function("Ping", "() -> void", move |_context, _arguments| {
                let _ = &captured;
                Ok(None)
            })
            .unwrap();
        assert_eq!(Rc::strong_count(&witness), 2);
    }
    assert_eq!(Rc::strong_count(&witness), 1);
}

#[test]
fn values_from_another_runtime_are_refused() {
    let first = Runtime::new().unwrap();
    let second = Runtime::new().unwrap();
    let program = mount(&first, "Echo :: (value: i32) -> i32 { return value; }");
    let foreign = second.make_i32(1).unwrap();
    let error = program.call_function("Echo", &[foreign]).unwrap_err();
    assert_eq!(error.status, Status::InvalidArgument);
}

#[test]
fn values_and_programs_outlive_the_runtime_handle() {
    let (program, value) = {
        let runtime = Runtime::new().unwrap();
        let program = mount(&runtime, "Seven :: () -> i32 { return 7; }");
        let value = runtime.make_string("kept").unwrap();
        (program, value)
    };
    assert_eq!(call_i32(&program, "Seven", &[]).unwrap(), 7);
    assert_eq!(value.as_str().unwrap(), "kept");
}

struct Counter {
    count: Cell<i32>,
    drops: Rc<Cell<usize>>,
}

impl Drop for Counter {
    fn drop(&mut self) {
        self.drops.set(self.drops.get() + 1);
    }
}

fn register_counter(runtime: &Runtime, drops: Rc<Cell<usize>>) -> nyx::HostType<Counter> {
    let constructed_drops = drops.clone();
    runtime
        .register_type::<Counter>("Counter")
        .constructor("(start: i32) -> void", move |_context, arguments| {
            Ok(Counter { count: Cell::new(arguments[0].as_i32()?), drops: constructed_drops.clone() })
        })
        .method("Add", "(amount: i32) -> void", Receiver::Mutable, |_context, counter, arguments| {
            counter.count.set(counter.count.get() + arguments[0].as_i32()?);
            Ok(None)
        })
        .method("Count", "() -> i32", Receiver::Immutable, |context, counter, _arguments| {
            Ok(Some(context.make_i32(counter.count.get())?))
        })
        .static_method("Zero", "() -> Counter", move |_context, counter_type, _arguments| {
            Ok(Some(counter_type.wrap(Counter { count: Cell::new(0), drops: drops.clone() })?))
        })
        .register()
        .unwrap()
}

const COUNTER_SCRIPT: &str = "Counters :: namespace {\n\
    Built :: () -> i32 { counter ~= new Counter(10); counter.Add(5); return counter.Count(); }\n\
    FromZero :: () -> i32 { counter ~= Counter:::Zero(); counter.Add(3); return counter.Count(); }\n\
    Bump :: (counter~ Counter) -> :Counter { counter.Add(1); return counter; }\n\
    Make :: (start: i32) -> Counter { return new Counter(start); }\n\
}";

#[test]
fn a_rust_type_is_constructed_and_called_from_nyx() {
    let runtime = Runtime::new().unwrap();
    let drops = Rc::new(Cell::new(0));
    let _counter_type = register_counter(&runtime, drops.clone());
    let program = mount(&runtime, COUNTER_SCRIPT);
    assert_eq!(call_i32(&program, "Counters:::Built", &[]).unwrap(), 15);
    assert_eq!(call_i32(&program, "Counters:::FromZero", &[]).unwrap(), 3);
    assert_eq!(drops.get(), 2);
}

#[test]
fn a_rust_instance_is_passed_in_and_read_back() {
    let runtime = Runtime::new().unwrap();
    let drops = Rc::new(Cell::new(0));
    let counter_type = register_counter(&runtime, drops.clone());
    let program = mount(&runtime, COUNTER_SCRIPT);

    let hero = counter_type.wrap(Counter { count: Cell::new(41), drops: drops.clone() }).unwrap();
    assert_eq!(hero.kind(), Kind::HostObject);
    let bumped = program.call_function("Counters:::Bump", std::slice::from_ref(&hero)).unwrap().unwrap();
    assert_eq!(counter_type.get(&bumped).unwrap().count.get(), 42);
    assert!(std::ptr::eq(counter_type.get(&bumped).unwrap(), counter_type.get(&hero).unwrap()));

    let made = program.call_function("Counters:::Make", &[runtime.make_i32(9).unwrap()]).unwrap().unwrap();
    assert_eq!(counter_type.get(&made).unwrap().count.get(), 9);

    assert_eq!(drops.get(), 0);
    drop(bumped);
    drop(hero);
    assert_eq!(drops.get(), 1);
    drop(made);
    assert_eq!(drops.get(), 2);
}

#[test]
fn a_host_object_of_another_type_is_a_mismatch() {
    let runtime = Runtime::new().unwrap();
    let drops = Rc::new(Cell::new(0));
    let counter_type = register_counter(&runtime, drops.clone());
    let other_type = runtime.register_type::<String>("Label").register().unwrap();
    let label = other_type.wrap("banner".to_string()).unwrap();
    let error = counter_type.get(&label).err().expect("a Label isn't a Counter");
    assert_eq!(error.status, Status::TypeMismatch);
    assert_eq!(other_type.get(&label).unwrap(), "banner");
}

#[test]
fn instances_still_alive_are_dropped_with_the_runtime() {
    let drops = Rc::new(Cell::new(0));
    {
        let runtime = Runtime::new().unwrap();
        let _counter_type = register_counter(&runtime, drops.clone());
        let program = runtime
            .mount_source(
                "Keeper :: class : Application {\npublic:\n    kept~ Counter? = null;\n    Run :: () -> void { }\n    Keep~: () -> void { kept = new Counter(1); }\n}\n",
                "keeper.nyx",
                "Keeper",
            )
            .unwrap();
        program.call_application_method("Keep", &[]).unwrap();
        assert_eq!(drops.get(), 0);
    }
    assert_eq!(drops.get(), 1);
}

#[test]
fn a_constructor_can_refuse() {
    let runtime = Runtime::new().unwrap();
    runtime
        .register_type::<Counter>("Counter")
        .constructor("(start: i32) -> void", |context, _arguments| -> Result<Counter, CallbackError> {
            Err(context.refusal("counters are disabled"))
        })
        .register()
        .unwrap();
    let program = mount(&runtime, "Make :: () -> i32 { counter := new Counter(1); return 0; }");
    let error = program.call_function("Make", &[]).unwrap_err();
    assert_eq!(error.status, Status::Runtime);
    assert!(error.message.contains("counters are disabled"), "message was {}", error.message);
}

#[test]
fn asks_whether_the_application_defines_a_hook() {
    let runtime = Runtime::new().unwrap();
    let program = runtime
        .mount_source(
            "BaseGame :: class : Application {\npublic:\n    Run :: () -> void { }\n    OnRender :: () -> void { }\n}\n\
             Game :: class : BaseGame {\npublic:\n    OnFixedUpdate~: (dt: f32) -> void { }\n}\n",
            "hooks.nyx",
            "Game",
        )
        .unwrap();
    assert!(program.defines_method("OnFixedUpdate").unwrap());
    assert!(program.defines_method("OnRender").unwrap());
    assert!(!program.defines_method("OnStart").unwrap());
    let text = runtime.make_string("not a game").unwrap();
    assert_eq!(program.defines_method_on(&text, "OnRender").unwrap_err().status, Status::InvalidArgument);
}

#[test]
fn a_failure_and_a_refusal_carry_the_nyx_call_stack() {
    let runtime = Runtime::new().unwrap();
    runtime
        .register_function("RefuseNow", "() -> void", |context, _| Err(context.refusal("the engine refused")))
        .unwrap();
    let program = runtime
        .mount_source(
            "Game :: class : Application {\npublic:\n    Run :: () -> void { }\n\
             \x20   Step :: () -> i32 {\n        return Check() + 1;\n    }\n\
             \x20   Check :: () -> i32 {\n        throw Error:::Unknown(\"broken\");\n    }\n\
             \x20   Refuse :: () -> void {\n        RefuseNow();\n    }\n}\n",
            "stack.nyx",
            "Game",
        )
        .unwrap();
    let failure = program.call_application_method("Step", &[]).unwrap_err();
    assert_eq!(failure.status, Status::Failure);
    let functions: Vec<&str> = failure.stack.iter().map(|frame| frame.function.as_str()).collect();
    assert_eq!(functions, ["Game.Check", "Game.Step"]);
    let location = failure.location().unwrap();
    assert_eq!((location.path.as_str(), location.line, location.column), ("stack.nyx", 8, 9));
    assert!(failure.to_string().ends_with("at Game.Check (stack.nyx:8:9)"), "{failure}");

    let refusal = program.call_application_method("Refuse", &[]).unwrap_err();
    assert_eq!(refusal.status, Status::Runtime);
    assert_eq!(refusal.location().map(|frame| (frame.function.as_str(), frame.line)), Some(("Game.Refuse", 11)));
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
struct Vector3 {
    x: f32,
    y: f32,
    z: f32,
}

#[test]
fn a_value_type_has_fields_braces_and_copies() {
    let runtime = Runtime::new().unwrap();
    let vectors = runtime
        .register_value_type::<Vector3>("Vec3", Vector3 { x: 0.0, y: 0.0, z: 12.0 })
        .field("x", |v| &mut v.x)
        .field("y", |v| &mut v.y)
        .field("z", |v| &mut v.z)
        .method("LengthSquared", "() -> f32", Receiver::Immutable, |context, v, _| {
            Ok(Some(context.make_f32(v.x * v.x + v.y * v.y + v.z * v.z)?))
        })
        .static_method("Up", "() -> Vec3", |_context, own_type, _| {
            Ok(Some(own_type.wrap(Vector3 { x: 0.0, y: 1.0, z: 0.0 })?))
        })
        .register()
        .unwrap();
    assert!(vectors.is_value_type());
    let program = runtime
        .mount_source(
            "Game :: class : Application {\npublic:\n    spawn~ Vec3 = Vec3 { x: 1.0f };\n    Run :: () -> void { }\n\
             \x20   Built :: () -> f32 { v := Vec3 { x: 3.0f, y: 4.0f }; return v.LengthSquared(); }\n\
             \x20   Store~: (at: Vec3) -> void { spawn = at; }\n\
             \x20   Spawn :: () -> Vec3 { return spawn; }\n\
             \x20   Up :: () -> Vec3 { return Vec3:::Up(); }\n}\n",
            "vectors.nyx",
            "Game",
        )
        .unwrap();
    let built = program.call_application_method("Built", &[]).unwrap().unwrap();
    assert_eq!(built.as_f32().unwrap(), 169.0);
    let mut at = Vector3 { x: 7.0, y: 8.0, z: 9.0 };
    let argument = vectors.wrap(at).unwrap();
    at.x = -1.0;
    assert_eq!(vectors.get(&argument).unwrap().x, 7.0);
    assert_eq!(at.x, -1.0);
    program.call_application_method("Store", &[argument]).unwrap();
    let spawn = program.call_application_method("Spawn", &[]).unwrap().unwrap();
    assert_eq!(*vectors.get(&spawn).unwrap(), Vector3 { x: 7.0, y: 8.0, z: 9.0 });
    let up = program.call_application_method("Up", &[]).unwrap().unwrap();
    assert_eq!(*vectors.get(&up).unwrap(), Vector3 { x: 0.0, y: 1.0, z: 0.0 });
}

#[test]
fn a_value_type_declares_operators() {
    let runtime = Runtime::new().unwrap();
    let vectors = runtime
        .register_value_type::<Vector3>("Vec3", Vector3 { x: 0.0, y: 0.0, z: 0.0 })
        .field("x", |v| &mut v.x)
        .field("y", |v| &mut v.y)
        .field("z", |v| &mut v.z)
        .operator("+", "(other: Vec3) -> Vec3", |_context, own_type, left, operands| {
            let right = *own_type.get(&operands[0])?;
            Ok(Some(own_type.wrap(Vector3 { x: left.x + right.x, y: left.y + right.y, z: left.z + right.z })?))
        })
        .operator("*", "(scale: f32) -> Vec3", |_context, own_type, vector, operands| {
            let scale = operands[0].as_f32()?;
            Ok(Some(own_type.wrap(Vector3 { x: vector.x * scale, y: vector.y * scale, z: vector.z * scale })?))
        })
        .operator("-", "() -> Vec3", |_context, own_type, vector, _| {
            Ok(Some(own_type.wrap(Vector3 { x: -vector.x, y: -vector.y, z: -vector.z })?))
        })
        .register()
        .unwrap();
    let program = runtime
        .mount_source(
            "Game :: class : Application {\npublic:\n    Run :: () -> void { }\n\
             \x20   Step :: (at: Vec3) -> Vec3 { velocity := Vec3 { y: -2.0f }; return -(at + velocity * 0.5f); }\n}\n",
            "operators.nyx",
            "Game",
        )
        .unwrap();
    let at = vectors.wrap(Vector3 { x: 1.0, y: 2.0, z: 3.0 }).unwrap();
    let stepped = program.call_application_method("Step", &[at]).unwrap().unwrap();
    assert_eq!(*vectors.get(&stepped).unwrap(), Vector3 { x: -1.0, y: -1.0, z: -3.0 });

    let refused = runtime
        .register_value_type::<Vector3>("Vec4", Vector3 { x: 0.0, y: 0.0, z: 0.0 })
        .operator("%", "(other: Vec4) -> Vec4", |_context, _, _, _| Ok(None))
        .register()
        .err()
        .unwrap();
    assert_eq!(refused.message, "'Vec4' can't declare the operator '%': a host type declares '+', '-', '*' or '/'");
}

#[test]
fn a_callback_held_by_the_host_is_called_later() {
    let runtime = Runtime::new().unwrap();
    let remembered: Rc<RefCell<Option<Value>>> = Rc::new(RefCell::new(None));
    let sink = remembered.clone();
    runtime
        .register_function("Remember", "(callback: (value: i32) -> i32) -> void", move |_context, arguments| {
            *sink.borrow_mut() = Some(arguments[0].clone());
            Ok(None)
        })
        .unwrap();
    let source = "Game :: class : Application {\npublic:\n    Run :: () -> void { }\n    Start :: () -> void {\n        offset := 3;\n        Remember((value: i32) -> i32 { return value * 10 + offset; });\n    }\n}\n";
    let program = runtime.mount_source(source, "callbacks.nyx", "Game").unwrap();
    program.call_application_method("Start", &[]).unwrap();
    let callback = remembered.borrow_mut().take().expect("the game handed over its callback");
    assert_eq!(callback.kind(), Kind::Callable);
    let argument = runtime.make_i32(7).unwrap();
    assert_eq!(callback.call(std::slice::from_ref(&argument)).unwrap().unwrap().as_i32().unwrap(), 73);
    assert_eq!(callback.call(&[]).unwrap_err().status, Status::Arguments);
}

#[test]
fn dictionaries_built_in_rust_are_read_by_nyx_and_back() {
    let runtime = Runtime::new().unwrap();
    let program = mount(
        &runtime,
        "Total :: (scores: Dictionary<string, i32>) -> i32 {\n    total~ i32 = 0;\n    for key in scores.keys() { total += scores[key]; }\n    return total;\n}\n\
         Scores :: () -> Dictionary<string, i32> {\n    scores~ Dictionary<string, i32> = {};\n    scores[\"beta\"] = 2;\n    scores[\"alpha\"] = 1;\n    return scores;\n}",
    );
    let entries: Vec<(Value, Value)> = [("one", 10), ("two", 20)]
        .iter()
        .map(|(name, number)| (runtime.make_string(name).unwrap(), runtime.make_i32(*number).unwrap()))
        .collect();
    let dictionary = runtime.make_dictionary(&entries).unwrap();
    assert_eq!(dictionary.kind(), Kind::Dictionary);
    assert_eq!(call_i32(&program, "Total", &[dictionary]).unwrap(), 30);

    let scores = program.call_function("Scores", &[]).unwrap().unwrap();
    assert_eq!(scores.dictionary_length().unwrap(), 2);
    assert_eq!(scores.dictionary_get("beta").unwrap().as_i32().unwrap(), 2);
    assert_eq!(scores.dictionary_get("gamma").unwrap_err().status, Status::NotFound);
    let names: Vec<String> =
        scores.dictionary_entries().unwrap().iter().map(|(key, _)| key.as_str().unwrap().to_string()).collect();
    assert_eq!(names, ["alpha", "beta"]);
    let bad_key = runtime.make_f32(1.5).unwrap();
    let element = runtime.make_i32(1).unwrap();
    assert_eq!(runtime.make_dictionary(&[(bad_key, element)]).unwrap_err().status, Status::TypeMismatch);
}

#[test]
fn an_objects_fields_and_a_failure_s_suppressed_failures_are_read() {
    let runtime = Runtime::new().unwrap();
    let source = "LevelError :: enum : Error<i32> {\n    NotFound(path: string) { logged := 1; },\n    Locked(path: string) { logged := 2; }\n}\n\
        Game :: class : Application {\npublic:\n    label: string = \"hero\";\n    Run :: () -> void { }\n    Self :: () -> :Game { return this; }\n    Broken :: () -> i32 {\n        defer { throw LevelError:::Locked(\"save\"); }\n        throw LevelError:::NotFound(\"x.lvl\");\n    }\n}\n";
    let program = runtime.mount_source(source, "objects.nyx", "Game").unwrap();
    let game = program.call_application_method("Self", &[]).unwrap().unwrap();
    assert_eq!(game.kind(), Kind::Object);
    assert_eq!(game.object_type_name().unwrap(), "Game");
    assert_eq!(game.object_field("label").unwrap().as_str().unwrap(), "hero");
    assert_eq!(game.object_field("missing").unwrap_err().status, Status::NotFound);

    let error = program.call_application_method("Broken", &[]).unwrap_err();
    let failure = error.failure.expect("the failure's error value");
    let suppressed = failure.error_suppressed().unwrap();
    assert_eq!(suppressed.len(), 1);
    assert_eq!(suppressed[0].error_variant().unwrap(), "LevelError::Locked");
}

#[test]
fn a_patch_changes_the_next_call_and_a_bad_one_changes_nothing() {
    let runtime = Runtime::new().unwrap();
    let program = mount(&runtime, "Scale :: (value: i32) -> i32 { return value * 2; }");
    let four = runtime.make_i32(4).unwrap();
    assert_eq!(call_i32(&program, "Scale", std::slice::from_ref(&four)).unwrap(), 8);
    program.patch_function("Scale :: (value: i32) -> i32 { return value * 10; }", "Scale", "patch.nyx").unwrap();
    assert_eq!(call_i32(&program, "Scale", std::slice::from_ref(&four)).unwrap(), 40);

    let restart = program
        .patch_function("Scale :: (value: i32, extra: i32) -> i32 { return value * extra; }", "Scale", "patch.nyx")
        .unwrap_err();
    assert_eq!(restart.status, Status::RestartRequired);
    let refused = program
        .patch_function("Scale :: (value: i32) -> i32 { return value * missing; }", "Scale", "patch.nyx")
        .unwrap_err();
    assert_eq!(refused.status, Status::Refused);
    assert!(!runtime.diagnostics().is_empty());
    assert_eq!(call_i32(&program, "Scale", std::slice::from_ref(&four)).unwrap(), 40);
}
