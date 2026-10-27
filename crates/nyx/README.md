# nyx

A safe Rust wrapper over `nyx-capi`, the C ABI of the Nyx prototype interpreter
(`nyx-scripting-language`, branch `proto`). It builds on the raw bindings in `nyx-sys`.

```rust,ignore
use nyx::{Receiver, Runtime};

let runtime = Runtime::new()?;
runtime.register_function("Log", "(line: string) -> void", |_context, arguments| {
    println!("{}", arguments[0].as_str()?);
    Ok(None)
})?;
let program = runtime.mount_source(source, "game.nyx", "Game")?;
program.call_application_method("OnUpdate", &[runtime.make_f32(0.016)?])?;
```

## Ownership

- `Runtime`, `Program` and `Value` share the runtime through an `Rc`. The C runtime is destroyed
  when the last of them is dropped, so a program or value can outlive the `Runtime` you made.
  Nothing here is `Send`: a runtime stays on the thread that made it.
- A closure registered as a host function, method or constructor is owned by the runtime and
  dropped with it. A closure that captures a `Runtime`, `Value`, `Program` or `HostType` keeps the
  runtime alive, so it's never dropped. Capture plain Rust state (`Rc<RefCell<…>>`) instead, and
  reach the runtime through the `CallContext` each call gets.
- A host type's instances are boxed and owned by Nyx. A method gets `&T`, because Nyx can call
  back into the same instance while a method runs. Use `Cell` or `RefCell` for state a method
  changes.
- A static method gets the type's `HostType<T>`, so it can wrap new instances.

## Value types

`runtime.register_value_type::<T>(name, defaults)` registers a `Copy` type that Nyx stores by
value. `.field("x", |v| &mut v.x)` names a `bool`, `i32`, `i64`, `f32` or `f64` field; the
projection gives the field's offset, so no `unsafe` is needed. Scripts build one with braces
(`Vec3 { x: 1.0f }`, missing fields take `defaults`), read and write its fields, and get a copy
at every binding, argument and return. `HostType::wrap` copies the value in.

A value type can declare operators: `.operator("+", "(other: Vec3) -> Vec3", |context, own_type,
left, operands| …)`. The symbols are `+`, `-`, `*` and `/` with one operand, and `-` with none
for negation. The value type is always the left operand (`v * 2.0f`, not `2.0f * v`), and a
symbol can be declared once per operand type.

## Hooks

`program.defines_method("OnUpdate")` asks whether the application class (or a base) defines a
method, and `defines_method_on(&value, name)` asks the same of another instance, so a host can
skip a hook a game leaves out.

## Errors

- Every call returns `nyx::Result`. `Error::status` mirrors `nyx_status` and `Error::message` is
  the runtime's last error.
- A Nyx failure (`NYX_ERROR_FAILURE`) carries its error value in `Error::failure`.
- `Error::stack` lists the Nyx calls the error came through, innermost first, each with its file,
  line and column; `Error::location()` is the innermost one. `Value::error_stack()` reads the
  same from an error value a script returned.
- A callback returns `Ok(Some(value))`, `Ok(None)` for `void`, or a `CallbackError`:
  - `Failure(value)` is a Nyx failure the script can match with `when`. Build one with
    `context.failure("Error::Unknown", "message")`.
  - `Refusal(message)` stops the script's call. The host sees `Status::Runtime` with the message.
- A callback that panics becomes a failure with the variant `PANIC_VARIANT` and the panic's
  message, and the runtime stays usable.
