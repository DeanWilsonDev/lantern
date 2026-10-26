use std::fmt;
use std::ptr::{self, NonNull};
use std::rc::Rc;

use nyx_sys as sys;

use crate::error::{Error, Result, Status};
use crate::raw::borrowed_bytes;
use crate::runtime::{Runtime, RuntimeInner};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Kind {
    Null,
    Bool,
    I32,
    I64,
    F32,
    F64,
    String,
    Array,
    Dictionary,
    Object,
    HostObject,
    Callable,
    Module,
}

impl Kind {
    fn from_raw(raw: sys::nyx_kind) -> Kind {
        match raw {
            sys::NYX_KIND_BOOL => Kind::Bool,
            sys::NYX_KIND_I32 => Kind::I32,
            sys::NYX_KIND_I64 => Kind::I64,
            sys::NYX_KIND_F32 => Kind::F32,
            sys::NYX_KIND_F64 => Kind::F64,
            sys::NYX_KIND_STRING => Kind::String,
            sys::NYX_KIND_ARRAY => Kind::Array,
            sys::NYX_KIND_DICTIONARY => Kind::Dictionary,
            sys::NYX_KIND_OBJECT => Kind::Object,
            sys::NYX_KIND_HOST_OBJECT => Kind::HostObject,
            sys::NYX_KIND_CALLABLE => Kind::Callable,
            sys::NYX_KIND_MODULE => Kind::Module,
            _ => Kind::Null,
        }
    }
}

pub struct Value {
    raw: NonNull<sys::nyx_value>,
    runtime: Rc<RuntimeInner>,
}

impl Value {
    pub(crate) fn from_owned(runtime: Rc<RuntimeInner>, raw: *mut sys::nyx_value) -> Option<Value> {
        NonNull::new(raw).map(|raw| Value { raw, runtime })
    }

    pub(crate) unsafe fn from_borrowed(runtime: Rc<RuntimeInner>, raw: *mut sys::nyx_value) -> Option<Value> {
        NonNull::new(sys::nyx_value_retain(raw)).map(|raw| Value { raw, runtime })
    }

    pub(crate) fn raw(&self) -> *mut sys::nyx_value {
        self.raw.as_ptr()
    }

    pub(crate) fn into_raw(self) -> *mut sys::nyx_value {
        let value = std::mem::ManuallyDrop::new(self);
        let runtime = unsafe { ptr::read(&value.runtime) };
        drop(runtime);
        value.raw.as_ptr()
    }

    pub(crate) fn belongs_to(&self, runtime: &Rc<RuntimeInner>) -> bool {
        Rc::ptr_eq(&self.runtime, runtime)
    }

    pub fn runtime(&self) -> Runtime {
        Runtime::from_inner(self.runtime.clone())
    }

    pub fn kind(&self) -> Kind {
        Kind::from_raw(unsafe { sys::nyx_value_kind(self.raw()) })
    }

    pub fn is_null(&self) -> bool {
        self.kind() == Kind::Null
    }

    fn read<T: Default>(&self, read: unsafe extern "C" fn(*const sys::nyx_value, *mut T) -> sys::nyx_status) -> Result<T> {
        let mut out = T::default();
        self.runtime.check(unsafe { read(self.raw(), &mut out) })?;
        Ok(out)
    }

    pub fn as_bool(&self) -> Result<bool> {
        self.read(sys::nyx_value_get_bool)
    }

    pub fn as_i32(&self) -> Result<i32> {
        self.read(sys::nyx_value_get_i32)
    }

    pub fn as_i64(&self) -> Result<i64> {
        self.read(sys::nyx_value_get_i64)
    }

    pub fn as_f32(&self) -> Result<f32> {
        self.read(sys::nyx_value_get_f32)
    }

    pub fn as_f64(&self) -> Result<f64> {
        self.read(sys::nyx_value_get_f64)
    }

    fn read_text(
        &self,
        read: unsafe extern "C" fn(*const sys::nyx_value, *mut sys::nyx_str) -> sys::nyx_status,
    ) -> Result<&str> {
        let mut text = sys::nyx_str { data: ptr::null(), length: 0 };
        self.runtime.check(unsafe { read(self.raw(), &mut text) })?;
        let bytes = unsafe { borrowed_bytes(text) };
        std::str::from_utf8(bytes)
            .map_err(|_| Error::new(Status::TypeMismatch, "the Nyx string isn't valid UTF-8"))
    }

    pub fn as_str(&self) -> Result<&str> {
        self.read_text(sys::nyx_value_get_string)
    }

    pub fn array_length(&self) -> Result<usize> {
        self.read(sys::nyx_value_array_length)
    }

    pub fn array_get(&self, index: usize) -> Result<Value> {
        let mut element = ptr::null_mut();
        self.runtime.check(unsafe { sys::nyx_value_array_get(self.raw(), index, &mut element) })?;
        Value::from_owned(self.runtime.clone(), element)
            .ok_or_else(|| Error::new(Status::Internal, "nyx-capi returned no array element"))
    }

    pub fn array_items(&self) -> Result<Vec<Value>> {
        (0..self.array_length()?).map(|index| self.array_get(index)).collect()
    }

    pub fn error_variant(&self) -> Result<&str> {
        self.read_text(sys::nyx_value_error_variant)
    }

    pub fn error_message(&self) -> Result<&str> {
        self.read_text(sys::nyx_value_error_message)
    }
}

impl Clone for Value {
    fn clone(&self) -> Value {
        let raw = unsafe { sys::nyx_value_retain(self.raw()) };
        Value { raw: NonNull::new(raw).expect("nyx_value_retain returned NULL for a live value"), runtime: self.runtime.clone() }
    }
}

impl Drop for Value {
    fn drop(&mut self) {
        unsafe { sys::nyx_value_release(self.raw()) }
    }
}

impl fmt::Debug for Value {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let kind = self.kind();
        let mut tuple = formatter.debug_tuple("Value");
        match kind {
            Kind::Bool => tuple.field(&self.as_bool().unwrap_or_default()),
            Kind::I32 => tuple.field(&self.as_i32().unwrap_or_default()),
            Kind::I64 => tuple.field(&self.as_i64().unwrap_or_default()),
            Kind::F32 => tuple.field(&self.as_f32().unwrap_or_default()),
            Kind::F64 => tuple.field(&self.as_f64().unwrap_or_default()),
            Kind::String => tuple.field(&self.as_str().unwrap_or_default()),
            Kind::Object => match self.error_variant() {
                Ok(variant) => tuple.field(&variant).field(&self.error_message().unwrap_or_default()),
                Err(_) => tuple.field(&kind),
            },
            other => tuple.field(&other),
        };
        tuple.finish()
    }
}
