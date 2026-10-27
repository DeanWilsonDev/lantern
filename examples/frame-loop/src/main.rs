#![forbid(unsafe_code)]

use std::cell::{Cell, RefCell};
use std::process::ExitCode;
use std::rc::Rc;

use nyx::{CallbackError, Receiver, Runtime, Value};

const FRAME_COUNT: usize = 20;
const FRAME_SECONDS: f32 = 0.1;
const RUNNER_SPEED: f32 = 5.0;

const RACER_SOURCE: &str = r#"
Racer :: class : Application {
public:
    runner~ Entity? = null;
    frames~ i32 = 0;
    lost~ i32 = 0;
    Run :: () -> void { }
    Start~: (speed: f32) -> void {
        runner = Entity:::Spawn(speed);
        Log("runner spawned");
    }
    OnUpdate~: (dt: f32) -> void {
        frames += 1;
        guard current := runner return;
        moved := current.Advance(dt);
        when moved {
            distance: f32 -> {
                if (distance >= 10.0f) {
                    current.Despawn();
                    runner = null;
                    Log("finished");
                }
            },
            problem: Error -> {
                lost += 1;
            }
        }
    }
    Frames :: () -> i32 { return frames; }
    Lost :: () -> i32 { return lost; }
}
"#;

struct Entity {
    speed: f32,
    position: Cell<f32>,
    alive: Cell<bool>,
}

fn first_f32(arguments: &[Value]) -> Result<f32, CallbackError> {
    match arguments.first() {
        Some(argument) => Ok(argument.as_f32()?),
        None => Err(CallbackError::Refusal("expected one f32 argument".to_string())),
    }
}

fn run() -> nyx::Result<()> {
    let runtime = Runtime::new()?;
    let log_lines = Rc::new(RefCell::new(Vec::<String>::new()));
    let finished_position = Rc::new(Cell::new(0.0f32));

    let log_sink = log_lines.clone();
    runtime.register_function("Log", "(line: string) -> void", move |_context, arguments| {
        let line = match arguments.first() {
            Some(argument) => argument.as_str()?.to_string(),
            None => String::new(),
        };
        println!("[script] {line}");
        log_sink.borrow_mut().push(line);
        Ok(None)
    })?;

    let despawn_position = finished_position.clone();
    runtime
        .register_type::<Entity>("Entity")
        .method("Advance", "(dt: f32) -> f32", Receiver::Immutable, |context, entity, arguments| {
            if !entity.alive.get() {
                return Err(context.failure("Error::Unknown", "despawned"));
            }
            let seconds = first_f32(arguments)?;
            entity.position.set(entity.position.get() + entity.speed * seconds);
            Ok(Some(context.make_f32(entity.position.get())?))
        })
        .method("Despawn", "() -> void", Receiver::Immutable, move |_context, entity, _arguments| {
            entity.alive.set(false);
            despawn_position.set(entity.position.get());
            Ok(None)
        })
        .static_method("Spawn", "(speed: f32) -> Entity", |_context, entity_type, arguments| {
            let speed = first_f32(arguments)?;
            let entity = Entity { speed, position: Cell::new(0.0), alive: Cell::new(true) };
            Ok(Some(entity_type.wrap(entity)?))
        })
        .register()?;

    let program = match runtime.mount_source(RACER_SOURCE, "racer.nyx", "Racer") {
        Ok(program) => program,
        Err(error) => {
            eprintln!("{}", runtime.render_diagnostics().unwrap_or_default());
            return Err(error);
        }
    };

    program.call_application_method("Start", &[runtime.make_f32(RUNNER_SPEED)?])?;
    for _frame in 0..FRAME_COUNT {
        program.call_application_method("OnUpdate", &[runtime.make_f32(FRAME_SECONDS)?])?;
    }

    let frames = read_i32(&program, "Frames")?;
    let lost = read_i32(&program, "Lost")?;
    println!("ran {frames} frames; the runner finished at {:.2}", finished_position.get());

    let lines = log_lines.borrow();
    let expected_lines = ["runner spawned", "finished"];
    if frames != FRAME_COUNT as i32 || lost != 0 || finished_position.get() < 10.0 || *lines != expected_lines {
        return Err(nyx::Error {
            status: nyx::Status::Runtime,
            message: format!("unexpected run: frames {frames}, lost {lost}, log {lines:?}"),
            failure: None,
        });
    }
    Ok(())
}

fn read_i32(program: &nyx::Program, method: &str) -> nyx::Result<i32> {
    match program.call_application_method(method, &[])? {
        Some(value) => value.as_i32(),
        None => Ok(0),
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("frame-loop failed: {error}");
            ExitCode::FAILURE
        }
    }
}
