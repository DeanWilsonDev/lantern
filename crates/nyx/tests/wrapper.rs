use std::cell::{Cell, RefCell};
use std::rc::Rc;

use nyx::{CallbackError, Kind, Receiver, Runtime, Severity, Status, Value, PANIC_VARIANT};

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
fn host_failure_reaches_the_host_as_a_nyx_failure() {
    let runtime = Runtime::new().unwrap();
    runtime
        .register_function("Fuel", "() -> i32", |context, _arguments| Err(context.failure("Error::Unknown", "out of fuel")))
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
        .register_function("Fuel", "() -> i32", |context, _arguments| Err(context.failure("Error::Unknown", "out of fuel")))
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
