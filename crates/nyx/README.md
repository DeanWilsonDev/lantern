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

## Errors

- Every call returns `nyx::Result`. `Error::status` mirrors `nyx_status` and `Error::message` is
  the runtime's last error.
- A Nyx failure (`NYX_ERROR_FAILURE`) carries its error value in `Error::failure`.
- A callback returns `Ok(Some(value))`, `Ok(None)` for `void`, or a `CallbackError`:
  - `Failure(value)` is a Nyx failure the script can match with `when`. Build one with
    `context.failure("Error::Unknown", "message")`.
  - `Refusal(message)` stops the script's call. The host sees `Status::Runtime` with the message.
- A callback that panics becomes a failure with the variant `PANIC_VARIANT` and the panic's
  message, and the runtime stays usable.
