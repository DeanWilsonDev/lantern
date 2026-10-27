use std::ptr::{self, NonNull};
use std::rc::Rc;

use nyx_sys as sys;

use crate::error::{Error, Result, Status};
use crate::raw::nyx_str;
use crate::runtime::{Runtime, RuntimeInner};
use crate::value::Value;

pub struct Program {
    raw: NonNull<sys::nyx_program>,
    runtime: Runtime,
}

impl Program {
    pub(crate) fn from_raw(runtime: Rc<RuntimeInner>, raw: *mut sys::nyx_program) -> Program {
        Program {
            raw: NonNull::new(raw).expect("nyx-capi mounted a program but returned no handle"),
            runtime: Runtime::from_inner(runtime),
        }
    }

    pub fn runtime(&self) -> &Runtime {
        &self.runtime
    }

    pub fn application(&self) -> Result<Value> {
        let mut raw = ptr::null_mut();
        self.runtime.check(unsafe { sys::nyx_program_application(self.raw.as_ptr(), &mut raw) })?;
        self.runtime.adopt(raw).ok_or_else(|| Error::new(Status::Internal, "nyx-capi returned no application instance"))
    }

    pub fn call_function(&self, qualified_name: &str, args: &[Value]) -> Result<Option<Value>> {
        let raw_args = self.runtime.raw_arguments(args)?;
        let mut result = ptr::null_mut();
        let status = unsafe {
            sys::nyx_program_call_function(
                self.raw.as_ptr(),
                nyx_str(qualified_name),
                raw_args.as_ptr(),
                raw_args.len(),
                &mut result,
            )
        };
        self.finish_call(status, result)
    }

    pub fn call_application_method(&self, method: &str, args: &[Value]) -> Result<Option<Value>> {
        self.call_method_on(ptr::null_mut(), method, args)
    }

    pub fn call_method(&self, receiver: &Value, method: &str, args: &[Value]) -> Result<Option<Value>> {
        let raw_receiver = self.runtime.raw_arguments(std::slice::from_ref(receiver))?[0];
        self.call_method_on(raw_receiver, method, args)
    }

    pub fn defines_method(&self, method: &str) -> Result<bool> {
        self.defines_method_on_raw(ptr::null_mut(), method)
    }

    pub fn defines_method_on(&self, receiver: &Value, method: &str) -> Result<bool> {
        let raw_receiver = self.runtime.raw_arguments(std::slice::from_ref(receiver))?[0];
        self.defines_method_on_raw(raw_receiver, method)
    }

    fn defines_method_on_raw(&self, receiver: *mut sys::nyx_value, method: &str) -> Result<bool> {
        let mut defined = false;
        self.runtime.check(unsafe {
            sys::nyx_program_defines_method(self.raw.as_ptr(), receiver, nyx_str(method), &mut defined)
        })?;
        Ok(defined)
    }

    fn call_method_on(&self, receiver: *mut sys::nyx_value, method: &str, args: &[Value]) -> Result<Option<Value>> {
        let raw_args = self.runtime.raw_arguments(args)?;
        let mut result = ptr::null_mut();
        let status = unsafe {
            sys::nyx_program_call_method(
                self.raw.as_ptr(),
                receiver,
                nyx_str(method),
                raw_args.as_ptr(),
                raw_args.len(),
                &mut result,
            )
        };
        self.finish_call(status, result)
    }

    fn finish_call(&self, status: sys::nyx_status, result: *mut sys::nyx_value) -> Result<Option<Value>> {
        let result = self.runtime.adopt(result);
        if status == sys::NYX_ERROR_FAILURE {
            let message = match &result {
                Some(failure) => format!(
                    "{}: {}",
                    failure.error_variant().unwrap_or("an error"),
                    failure.error_message().unwrap_or_default()
                ),
                None => "the call failed".to_string(),
            };
            let stack = result.as_ref().and_then(|failure| failure.error_stack().ok()).unwrap_or_default();
            return Err(Error { status: Status::Failure, message, failure: result, stack });
        }
        self.runtime.check(status)?;
        Ok(result)
    }
}

impl Drop for Program {
    fn drop(&mut self) {
        unsafe { sys::nyx_program_release(self.raw.as_ptr()) }
    }
}
