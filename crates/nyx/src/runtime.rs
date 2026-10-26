use std::path::Path;
use std::ptr;
use std::rc::Rc;

use nyx_sys as sys;

use crate::diagnostic::Diagnostic;
use crate::error::{Error, Result, Status};
use crate::host::{self, CallContext, HostTypeBuilder};
use crate::program::Program;
use crate::raw::{nyx_str, owned_string, path_text};
use crate::value::Value;
use crate::CallbackResult;

pub(crate) struct RuntimeInner {
    pub(crate) raw: *mut sys::nyx_runtime,
}

impl RuntimeInner {
    pub(crate) fn last_error(&self) -> String {
        let mut message = sys::nyx_str { data: ptr::null(), length: 0 };
        unsafe {
            sys::nyx_runtime_last_error(self.raw, &mut message);
            owned_string(message)
        }
    }

    pub(crate) fn check(&self, status: sys::nyx_status) -> Result<()> {
        match Status::from_raw(status) {
            None => Ok(()),
            Some(status) => Err(Error::new(status, self.last_error())),
        }
    }
}

impl Drop for RuntimeInner {
    fn drop(&mut self) {
        unsafe {
            sys::nyx_runtime_destroy(self.raw);
        }
    }
}

#[derive(Clone)]
pub struct Runtime {
    pub(crate) inner: Rc<RuntimeInner>,
}

impl Runtime {
    pub fn new() -> Result<Runtime> {
        let mut raw = ptr::null_mut();
        let status = unsafe { sys::nyx_runtime_create(&mut raw) };
        if let Some(status) = Status::from_raw(status) {
            return Err(Error::new(status, "nyx_runtime_create failed"));
        }
        Ok(Runtime { inner: Rc::new(RuntimeInner { raw }) })
    }

    pub(crate) fn from_inner(inner: Rc<RuntimeInner>) -> Runtime {
        Runtime { inner }
    }

    pub(crate) fn raw(&self) -> *mut sys::nyx_runtime {
        self.inner.raw
    }

    pub(crate) fn check(&self, status: sys::nyx_status) -> Result<()> {
        self.inner.check(status)
    }

    pub fn add_import_search_path(&self, path: impl AsRef<Path>) -> Result<()> {
        let text = path_text(path.as_ref())?;
        self.check(unsafe { sys::nyx_runtime_add_import_search_path(self.raw(), nyx_str(text)) })
    }

    pub fn mount_source(&self, source: &str, name: &str, application_class: &str) -> Result<Program> {
        let mut raw = ptr::null_mut();
        self.check(unsafe {
            sys::nyx_runtime_mount_source(
                self.raw(),
                nyx_str(source),
                nyx_str(name),
                nyx_str(application_class),
                &mut raw,
            )
        })?;
        Ok(Program::from_raw(self.inner.clone(), raw))
    }

    pub fn mount_file(&self, path: impl AsRef<Path>, application_class: &str) -> Result<Program> {
        let text = path_text(path.as_ref())?;
        let mut raw = ptr::null_mut();
        self.check(unsafe {
            sys::nyx_runtime_mount_file(self.raw(), nyx_str(text), nyx_str(application_class), &mut raw)
        })?;
        Ok(Program::from_raw(self.inner.clone(), raw))
    }

    pub fn diagnostics(&self) -> Vec<Diagnostic> {
        let count = unsafe { sys::nyx_runtime_diagnostic_count(self.raw()) };
        let mut diagnostics = Vec::with_capacity(count);
        for index in 0..count {
            let mut raw = unsafe { std::mem::zeroed::<sys::nyx_diagnostic>() };
            if unsafe { sys::nyx_runtime_diagnostic(self.raw(), index, &mut raw) } == sys::NYX_OK {
                diagnostics.push(unsafe { Diagnostic::from_raw(&raw) });
            }
        }
        diagnostics
    }

    pub fn render_diagnostics(&self) -> Result<String> {
        let mut raw = ptr::null_mut();
        self.check(unsafe { sys::nyx_runtime_render_diagnostics(self.raw(), &mut raw) })?;
        match self.adopt(raw) {
            Some(text) => Ok(text.as_str()?.to_string()),
            None => Ok(String::new()),
        }
    }

    pub(crate) fn adopt(&self, raw: *mut sys::nyx_value) -> Option<Value> {
        Value::from_owned(self.inner.clone(), raw)
    }

    fn make(
        &self,
        create: impl FnOnce(*mut sys::nyx_runtime, *mut *mut sys::nyx_value) -> sys::nyx_status,
    ) -> Result<Value> {
        let mut raw = ptr::null_mut();
        self.check(create(self.raw(), &mut raw))?;
        self.adopt(raw).ok_or_else(|| Error::new(Status::Internal, "nyx-capi made a value but returned no handle"))
    }

    pub fn make_null(&self) -> Result<Value> {
        self.make(|runtime, out| unsafe { sys::nyx_value_new_null(runtime, out) })
    }

    pub fn make_bool(&self, value: bool) -> Result<Value> {
        self.make(|runtime, out| unsafe { sys::nyx_value_new_bool(runtime, value, out) })
    }

    pub fn make_i32(&self, value: i32) -> Result<Value> {
        self.make(|runtime, out| unsafe { sys::nyx_value_new_i32(runtime, value, out) })
    }

    pub fn make_i64(&self, value: i64) -> Result<Value> {
        self.make(|runtime, out| unsafe { sys::nyx_value_new_i64(runtime, value, out) })
    }

    pub fn make_f32(&self, value: f32) -> Result<Value> {
        self.make(|runtime, out| unsafe { sys::nyx_value_new_f32(runtime, value, out) })
    }

    pub fn make_f64(&self, value: f64) -> Result<Value> {
        self.make(|runtime, out| unsafe { sys::nyx_value_new_f64(runtime, value, out) })
    }

    pub fn make_string(&self, value: &str) -> Result<Value> {
        self.make(|runtime, out| unsafe { sys::nyx_value_new_string(runtime, nyx_str(value), out) })
    }

    pub fn make_array(&self, items: &[Value]) -> Result<Value> {
        let raw_items = self.raw_arguments(items)?;
        self.make(|runtime, out| unsafe { sys::nyx_value_new_array(runtime, raw_items.as_ptr(), raw_items.len(), out) })
    }

    pub fn make_error(&self, variant: &str, message: &str) -> Result<Value> {
        self.make(|runtime, out| unsafe { sys::nyx_value_new_error(runtime, nyx_str(variant), nyx_str(message), out) })
    }

    pub(crate) fn raw_arguments(&self, values: &[Value]) -> Result<Vec<*mut sys::nyx_value>> {
        values
            .iter()
            .map(|value| {
                if value.belongs_to(&self.inner) {
                    Ok(value.raw())
                } else {
                    Err(Error::new(Status::InvalidArgument, "a value from another runtime can't be passed to this one"))
                }
            })
            .collect()
    }

    pub fn register_function<F>(&self, name: &str, signature: &str, function: F) -> Result<()>
    where
        F: Fn(&CallContext, &[Value]) -> CallbackResult + 'static,
    {
        host::register_function(self, name, signature, function)
    }

    pub fn register_type<T: 'static>(&self, name: &str) -> HostTypeBuilder<T> {
        HostTypeBuilder::new(self.clone(), name)
    }
}
