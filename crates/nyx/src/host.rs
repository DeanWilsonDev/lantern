use std::any::Any;
use std::ffi::c_void;
use std::marker::PhantomData;
use std::ops::Deref;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::ptr;
use std::rc::{Rc, Weak};

use nyx_sys as sys;

use crate::error::{CallbackError, CallbackResult, Error, Result, Status};
use crate::raw::nyx_str;
use crate::runtime::{Runtime, RuntimeInner};
use crate::value::Value;

pub const PANIC_VARIANT: &str = "Error::Unknown";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Receiver {
    Mutable,
    Immutable,
}

impl Receiver {
    fn to_raw(self) -> sys::nyx_receiver {
        match self {
            Receiver::Mutable => sys::NYX_RECEIVER_MUTABLE,
            Receiver::Immutable => sys::NYX_RECEIVER_IMMUTABLE,
        }
    }
}

pub struct CallContext {
    runtime: Runtime,
    call: *mut sys::nyx_call,
}

impl CallContext {
    pub fn runtime(&self) -> &Runtime {
        &self.runtime
    }

    pub fn failure(&self, variant: &str, message: &str) -> CallbackError {
        match self.runtime.make_error(variant, message) {
            Ok(error) => CallbackError::Failure(error),
            Err(error) => CallbackError::Refusal(error.to_string()),
        }
    }

    pub fn refusal(&self, message: impl Into<String>) -> CallbackError {
        CallbackError::Refusal(message.into())
    }
}

impl Deref for CallContext {
    type Target = Runtime;

    fn deref(&self) -> &Runtime {
        &self.runtime
    }
}

fn panic_text(payload: &(dyn Any + Send)) -> String {
    if let Some(text) = payload.downcast_ref::<&str>() {
        return (*text).to_string();
    }
    if let Some(text) = payload.downcast_ref::<String>() {
        return text.clone();
    }
    "a Rust host callback panicked".to_string()
}

unsafe fn run_callback<Body>(
    runtime: &Weak<RuntimeInner>,
    call: *mut sys::nyx_call,
    args: *const *mut sys::nyx_value,
    count: usize,
    out_result: *mut *mut sys::nyx_value,
    body: Body,
) -> sys::nyx_status
where
    Body: FnOnce(&CallContext, &[Value]) -> CallbackResult,
{
    let Some(inner) = runtime.upgrade() else {
        return sys::NYX_ERROR_INVALID_STATE;
    };
    let context = CallContext { runtime: Runtime::from_inner(inner.clone()), call };
    let outcome = catch_unwind(AssertUnwindSafe(|| {
        let mut arguments = Vec::with_capacity(count);
        for index in 0..count {
            let raw = if args.is_null() { ptr::null_mut() } else { *args.add(index) };
            match Value::from_borrowed(inner.clone(), raw) {
                Some(argument) => arguments.push(argument),
                None => arguments.push(context.make_null()?),
            }
        }
        body(&context, &arguments)
    }));
    let outcome = match outcome {
        Ok(outcome) => outcome,
        Err(payload) => Err(context.failure(PANIC_VARIANT, &panic_text(payload.as_ref()))),
    };
    match outcome {
        Ok(None) => sys::NYX_OK,
        Ok(Some(result)) => hand_over(&context, result, out_result, sys::NYX_OK),
        Err(CallbackError::Failure(failure)) => hand_over(&context, failure, out_result, sys::NYX_ERROR_FAILURE),
        Err(CallbackError::Refusal(message)) => {
            sys::nyx_call_set_message(call, nyx_str(&message));
            sys::NYX_ERROR_REFUSED
        }
    }
}

unsafe fn hand_over(
    context: &CallContext,
    value: Value,
    out_result: *mut *mut sys::nyx_value,
    status: sys::nyx_status,
) -> sys::nyx_status {
    if !value.belongs_to(&context.runtime.inner) {
        let message = "a host callback returned a value from another runtime";
        sys::nyx_call_set_message(context.call, nyx_str(message));
        return sys::NYX_ERROR_INVALID_ARGUMENT;
    }
    *out_result = value.into_raw();
    status
}

unsafe extern "C" fn free_boxed<Boxed>(userdata: *mut c_void) {
    drop(Box::from_raw(userdata.cast::<Boxed>()));
}

type FunctionBody = dyn Fn(&CallContext, &[Value]) -> CallbackResult;

struct FunctionSlot {
    runtime: Weak<RuntimeInner>,
    function: Box<FunctionBody>,
}

unsafe extern "C" fn function_trampoline(
    call: *mut sys::nyx_call,
    args: *const *mut sys::nyx_value,
    count: usize,
    out_result: *mut *mut sys::nyx_value,
    userdata: *mut c_void,
) -> sys::nyx_status {
    let slot = &*userdata.cast::<FunctionSlot>();
    run_callback(&slot.runtime, call, args, count, out_result, |context, arguments| (slot.function)(context, arguments))
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Documentation {
    pub summary: String,
    pub parameters: Vec<(String, String)>,
    pub returns: String,
    pub failures: Vec<(String, String)>,
    pub example: String,
}

impl Documentation {
    pub fn new(summary: &str) -> Self {
        Documentation { summary: summary.to_owned(), ..Documentation::default() }
    }

    pub fn parameter(mut self, name: &str, description: &str) -> Self {
        self.parameters.push((name.to_owned(), description.to_owned()));
        self
    }

    pub fn returns(mut self, description: &str) -> Self {
        self.returns = description.to_owned();
        self
    }

    pub fn failure(mut self, variant: &str, condition: &str) -> Self {
        self.failures.push((variant.to_owned(), condition.to_owned()));
        self
    }

    pub fn example(mut self, example: &str) -> Self {
        self.example = example.to_owned();
        self
    }
}

pub(crate) fn register_function<F>(runtime: &Runtime, name: &str, signature: &str, function: F) -> Result<()>
where
    F: Fn(&CallContext, &[Value]) -> CallbackResult + 'static,
{
    register_documented_function(runtime, name, signature, None, function)
}

pub(crate) fn register_documented_function<F>(
    runtime: &Runtime,
    name: &str,
    signature: &str,
    documentation: Option<&Documentation>,
    function: F,
) -> Result<()>
where
    F: Fn(&CallContext, &[Value]) -> CallbackResult + 'static,
{
    let parameters: Vec<sys::nyx_documented_parameter> = documentation
        .map(|record| {
            record
                .parameters
                .iter()
                .map(|(parameter_name, description)| sys::nyx_documented_parameter {
                    name: nyx_str(parameter_name),
                    description: nyx_str(description),
                })
                .collect()
        })
        .unwrap_or_default();
    let failures: Vec<sys::nyx_documented_failure> = documentation
        .map(|record| {
            record
                .failures
                .iter()
                .map(|(variant, condition)| sys::nyx_documented_failure {
                    variant: nyx_str(variant),
                    condition: nyx_str(condition),
                })
                .collect()
        })
        .unwrap_or_default();
    let raw_documentation = documentation.map(|record| sys::nyx_documentation {
        summary: nyx_str(&record.summary),
        parameters: parameters.as_ptr(),
        parameter_count: parameters.len(),
        returns: nyx_str(&record.returns),
        failures: failures.as_ptr(),
        failure_count: failures.len(),
        example: nyx_str(&record.example),
    });
    let documentation_pointer =
        raw_documentation.as_ref().map_or(std::ptr::null(), |record| record as *const sys::nyx_documentation);
    let slot =
        Box::into_raw(Box::new(FunctionSlot { runtime: Rc::downgrade(&runtime.inner), function: Box::new(function) }));
    let status = unsafe {
        sys::nyx_runtime_register_function_documented(
            runtime.raw(),
            nyx_str(name),
            nyx_str(signature),
            Some(function_trampoline),
            slot.cast(),
            Some(free_boxed::<FunctionSlot>),
            documentation_pointer,
        )
    };
    if status != sys::NYX_OK {
        drop(unsafe { Box::from_raw(slot) });
    }
    runtime.check(status)
}

type MethodBody<T> = dyn Fn(&CallContext, &T, &[Value]) -> CallbackResult;
type ConstructorBody<T> = dyn Fn(&CallContext, &[Value]) -> std::result::Result<T, CallbackError>;

struct MethodSlot<T> {
    runtime: Weak<RuntimeInner>,
    name: String,
    signature: String,
    receiver: Receiver,
    function: Box<MethodBody<T>>,
}

type StaticBody<T> = dyn Fn(&CallContext, &HostType<T>, &[Value]) -> CallbackResult;

struct StaticSlot<T> {
    runtime: Weak<RuntimeInner>,
    name: String,
    signature: String,
    value_type: bool,
    function: Box<StaticBody<T>>,
}

type OperatorBody<T> = dyn Fn(&CallContext, &HostType<T>, &T, &[Value]) -> CallbackResult;

struct OperatorSlot<T> {
    runtime: Weak<RuntimeInner>,
    symbol: String,
    signature: String,
    function: Box<OperatorBody<T>>,
}

type GetterBody<T> = dyn Fn(&CallContext, &T) -> std::result::Result<Value, CallbackError>;
type SetterBody<T> = dyn Fn(&CallContext, &T, &Value) -> std::result::Result<(), CallbackError>;

struct PropertySlot<T> {
    runtime: Weak<RuntimeInner>,
    name: String,
    type_name: String,
    getter: Box<GetterBody<T>>,
    setter: Option<Box<SetterBody<T>>>,
}

struct ConstructorSlot<T> {
    runtime: Weak<RuntimeInner>,
    signature: String,
    value_type: bool,
    function: Box<ConstructorBody<T>>,
}

mod sealed {
    pub trait Sealed {}
}

pub trait ValueField: Copy + sealed::Sealed + 'static {
    #[doc(hidden)]
    const KIND: sys::nyx_kind;
}

macro_rules! value_field {
    ($($rust:ty => $kind:ident),* $(,)?) => {
        $(
            impl sealed::Sealed for $rust {}
            impl ValueField for $rust {
                const KIND: sys::nyx_kind = sys::$kind;
            }
        )*
    };
}

value_field!(bool => NYX_KIND_BOOL, i32 => NYX_KIND_I32, i64 => NYX_KIND_I64, f32 => NYX_KIND_F32, f64 => NYX_KIND_F64);

struct ValueFieldLayout {
    name: String,
    kind: sys::nyx_kind,
    offset: usize,
}

struct ValueLayout<T> {
    defaults: Box<T>,
    fields: Vec<ValueFieldLayout>,
    error: Option<String>,
}

struct TypeSlots<T> {
    methods: Vec<MethodSlot<T>>,
    static_methods: Vec<StaticSlot<T>>,
    operators: Vec<OperatorSlot<T>>,
    properties: Vec<PropertySlot<T>>,
    constructor: Option<Box<ConstructorSlot<T>>>,
}

unsafe fn instance_of<T>(call: *mut sys::nyx_call, value: &Value) -> std::result::Result<&T, CallbackError> {
    let mut instance = ptr::null_mut();
    let status = sys::nyx_value_get_host_object(value.raw(), sys::nyx_call_host_type(call), &mut instance);
    if status != sys::NYX_OK || instance.is_null() {
        let error = value.runtime().check(status).err();
        let message = error.map(|error| error.message).unwrap_or_else(|| "the receiver has no instance".to_string());
        return Err(CallbackError::Refusal(message));
    }
    Ok(&*instance.cast::<T>())
}

unsafe extern "C" fn method_trampoline<T: 'static>(
    call: *mut sys::nyx_call,
    args: *const *mut sys::nyx_value,
    count: usize,
    out_result: *mut *mut sys::nyx_value,
    userdata: *mut c_void,
) -> sys::nyx_status {
    let slot = &*userdata.cast::<MethodSlot<T>>();
    run_callback(&slot.runtime, call, args, count, out_result, |context, arguments| {
        let Some((receiver, rest)) = arguments.split_first() else {
            return Err(CallbackError::Refusal(format!("'{}' was called without a receiver", slot.name)));
        };
        let instance = instance_of::<T>(call, receiver)?;
        (slot.function)(context, instance, rest)
    })
}

unsafe extern "C" fn getter_trampoline<T: 'static>(
    call: *mut sys::nyx_call,
    instance: *mut c_void,
    out_value: *mut *mut sys::nyx_value,
    userdata: *mut c_void,
) -> sys::nyx_status {
    let slot = &*userdata.cast::<PropertySlot<T>>();
    run_callback(&slot.runtime, call, ptr::null(), 0, out_value, |context, _arguments| {
        (slot.getter)(context, &*instance.cast::<T>()).map(Some)
    })
}

unsafe extern "C" fn setter_trampoline<T: 'static>(
    call: *mut sys::nyx_call,
    instance: *mut c_void,
    value: *mut sys::nyx_value,
    userdata: *mut c_void,
) -> sys::nyx_status {
    let slot = &*userdata.cast::<PropertySlot<T>>();
    let Some(setter) = slot.setter.as_ref() else {
        return sys::NYX_ERROR_INVALID_STATE;
    };
    run_callback(&slot.runtime, call, &raw const value, 1, ptr::null_mut(), |context, arguments| {
        setter(context, &*instance.cast::<T>(), &arguments[0])?;
        Ok(None)
    })
}

unsafe extern "C" fn static_trampoline<T: 'static>(
    call: *mut sys::nyx_call,
    args: *const *mut sys::nyx_value,
    count: usize,
    out_result: *mut *mut sys::nyx_value,
    userdata: *mut c_void,
) -> sys::nyx_status {
    let slot = &*userdata.cast::<StaticSlot<T>>();
    run_callback(&slot.runtime, call, args, count, out_result, |context, arguments| {
        let own_type = HostType::<T> {
            raw: sys::nyx_call_host_type(call),
            runtime: context.runtime().clone(),
            value_type: slot.value_type,
            marker: PhantomData,
        };
        (slot.function)(context, &own_type, arguments)
    })
}

unsafe extern "C" fn operator_trampoline<T: 'static>(
    call: *mut sys::nyx_call,
    args: *const *mut sys::nyx_value,
    count: usize,
    out_result: *mut *mut sys::nyx_value,
    userdata: *mut c_void,
) -> sys::nyx_status {
    let slot = &*userdata.cast::<OperatorSlot<T>>();
    run_callback(&slot.runtime, call, args, count, out_result, |context, arguments| {
        let Some((receiver, operands)) = arguments.split_first() else {
            return Err(CallbackError::Refusal(format!("operator '{}' was called without an operand", slot.symbol)));
        };
        let instance = instance_of::<T>(call, receiver)?;
        let own_type = HostType::<T> {
            raw: sys::nyx_call_host_type(call),
            runtime: context.runtime().clone(),
            value_type: true,
            marker: PhantomData,
        };
        (slot.function)(context, &own_type, instance, operands)
    })
}

unsafe extern "C" fn constructor_trampoline<T: 'static>(
    call: *mut sys::nyx_call,
    args: *const *mut sys::nyx_value,
    count: usize,
    out_result: *mut *mut sys::nyx_value,
    userdata: *mut c_void,
) -> sys::nyx_status {
    let slot = &*userdata.cast::<ConstructorSlot<T>>();
    run_callback(&slot.runtime, call, args, count, out_result, |context, arguments| {
        let instance = (slot.function)(context, arguments)?;
        let wrapped = wrap_instance(context.runtime(), sys::nyx_call_host_type(call), instance, slot.value_type)?;
        Ok(Some(wrapped))
    })
}

unsafe extern "C" fn release_instance<T>(instance: *mut c_void, _type_userdata: *mut c_void) {
    let _ = catch_unwind(AssertUnwindSafe(|| drop(Box::from_raw(instance.cast::<T>()))));
}

fn wrap_instance<T>(
    runtime: &Runtime,
    host_type: *mut sys::nyx_host_type,
    instance: T,
    value_type: bool,
) -> Result<Value> {
    if value_type {
        let mut raw = ptr::null_mut();
        let status =
            unsafe { sys::nyx_value_new_host_object(host_type, ptr::from_ref(&instance).cast_mut().cast(), &mut raw) };
        runtime.check(status)?;
        return runtime
            .adopt(raw)
            .ok_or_else(|| Error::new(Status::Internal, "nyx-capi made a value but returned no handle"));
    }
    let boxed = Box::into_raw(Box::new(instance));
    let mut raw = ptr::null_mut();
    let status = unsafe { sys::nyx_value_new_host_object(host_type, boxed.cast(), &mut raw) };
    if status != sys::NYX_OK {
        drop(unsafe { Box::from_raw(boxed) });
        runtime.check(status)?;
    }
    runtime.adopt(raw).ok_or_else(|| Error::new(Status::Internal, "nyx-capi made a host object but returned no handle"))
}

pub struct HostTypeBuilder<T> {
    runtime: Runtime,
    name: String,
    slots: TypeSlots<T>,
    value_layout: Option<ValueLayout<T>>,
}

impl<T: 'static> HostTypeBuilder<T> {
    pub(crate) fn new(runtime: Runtime, name: &str) -> HostTypeBuilder<T> {
        HostTypeBuilder {
            runtime,
            name: name.to_string(),
            slots: TypeSlots {
                methods: Vec::new(),
                static_methods: Vec::new(),
                operators: Vec::new(),
                properties: Vec::new(),
                constructor: None,
            },
            value_layout: None,
        }
    }

    pub(crate) fn new_value_type(runtime: Runtime, name: &str, defaults: T) -> HostTypeBuilder<T>
    where
        T: Copy,
    {
        let mut builder = HostTypeBuilder::new(runtime, name);
        builder.value_layout = Some(ValueLayout { defaults: Box::new(defaults), fields: Vec::new(), error: None });
        builder
    }

    pub fn field<F: ValueField>(mut self, name: &str, project: fn(&mut T) -> &mut F) -> Self {
        let type_name = self.name.clone();
        let Some(layout) = self.value_layout.as_mut() else {
            panic!("'{type_name}.{name}': only a type made with register_value_type has fields");
        };
        let base = ptr::from_mut::<T>(&mut layout.defaults) as usize;
        let at = ptr::from_mut::<F>(project(&mut layout.defaults)) as usize;
        if at < base || at + std::mem::size_of::<F>() > base + std::mem::size_of::<T>() {
            layout.error = Some(format!("'{type_name}.{name}' doesn't project to a field inside the value"));
            return self;
        }
        layout.fields.push(ValueFieldLayout { name: name.to_string(), kind: F::KIND, offset: at - base });
        self
    }

    pub fn method<F>(mut self, name: &str, signature: &str, receiver: Receiver, function: F) -> Self
    where
        F: Fn(&CallContext, &T, &[Value]) -> CallbackResult + 'static,
    {
        self.slots.methods.push(MethodSlot {
            runtime: Rc::downgrade(&self.runtime.inner),
            name: name.to_string(),
            signature: signature.to_string(),
            receiver,
            function: Box::new(function),
        });
        self
    }

    pub fn static_method<F>(mut self, name: &str, signature: &str, function: F) -> Self
    where
        F: Fn(&CallContext, &HostType<T>, &[Value]) -> CallbackResult + 'static,
    {
        self.slots.static_methods.push(StaticSlot {
            runtime: Rc::downgrade(&self.runtime.inner),
            name: name.to_string(),
            signature: signature.to_string(),
            value_type: self.value_layout.is_some(),
            function: Box::new(function),
        });
        self
    }

    pub fn operator<F>(mut self, symbol: &str, signature: &str, function: F) -> Self
    where
        F: Fn(&CallContext, &HostType<T>, &T, &[Value]) -> CallbackResult + 'static,
    {
        self.slots.operators.push(OperatorSlot {
            runtime: Rc::downgrade(&self.runtime.inner),
            symbol: symbol.to_string(),
            signature: signature.to_string(),
            function: Box::new(function),
        });
        self
    }

    /// A read-only field scripts can read as `instance.name`. `type_name` is Nyx type text, such as `i32`.
    pub fn property<G>(mut self, name: &str, type_name: &str, getter: G) -> Self
    where
        G: Fn(&CallContext, &T) -> std::result::Result<Value, CallbackError> + 'static,
    {
        self.slots.properties.push(PropertySlot {
            runtime: Rc::downgrade(&self.runtime.inner),
            name: name.to_string(),
            type_name: type_name.to_string(),
            getter: Box::new(getter),
            setter: None,
        });
        self
    }

    /// A field scripts can read and assign. The setter gets `&T`, like a method, so a type that changes
    /// through a property needs interior mutability such as `Cell` or `RefCell`.
    pub fn writable_property<G, S>(mut self, name: &str, type_name: &str, getter: G, setter: S) -> Self
    where
        G: Fn(&CallContext, &T) -> std::result::Result<Value, CallbackError> + 'static,
        S: Fn(&CallContext, &T, &Value) -> std::result::Result<(), CallbackError> + 'static,
    {
        self.slots.properties.push(PropertySlot {
            runtime: Rc::downgrade(&self.runtime.inner),
            name: name.to_string(),
            type_name: type_name.to_string(),
            getter: Box::new(getter),
            setter: Some(Box::new(setter)),
        });
        self
    }

    pub fn constructor<F>(mut self, signature: &str, function: F) -> Self
    where
        F: Fn(&CallContext, &[Value]) -> std::result::Result<T, CallbackError> + 'static,
    {
        self.slots.constructor = Some(Box::new(ConstructorSlot {
            runtime: Rc::downgrade(&self.runtime.inner),
            signature: signature.to_string(),
            value_type: self.value_layout.is_some(),
            function: Box::new(function),
        }));
        self
    }

    pub fn register(self) -> Result<HostType<T>> {
        let HostTypeBuilder { runtime, name, slots, value_layout } = self;
        if let Some(error) = value_layout.as_ref().and_then(|layout| layout.error.clone()) {
            return Err(Error::new(Status::InvalidArgument, error));
        }
        let raw_fields: Vec<sys::nyx_host_field> = value_layout
            .as_ref()
            .map(|layout| {
                layout
                    .fields
                    .iter()
                    .map(|field| sys::nyx_host_field {
                        name: nyx_str(&field.name),
                        kind: field.kind,
                        offset: field.offset,
                    })
                    .collect()
            })
            .unwrap_or_default();
        let raw_layout = value_layout.as_ref().map(|layout| sys::nyx_host_value_layout {
            size: std::mem::size_of::<T>(),
            alignment: std::mem::align_of::<T>(),
            fields: if raw_fields.is_empty() { ptr::null() } else { raw_fields.as_ptr() },
            field_count: raw_fields.len(),
            defaults: ptr::from_ref::<T>(&layout.defaults).cast(),
        });
        let slots = Box::new(slots);
        let methods: Vec<sys::nyx_host_method> = slots
            .methods
            .iter()
            .map(|slot| sys::nyx_host_method {
                name: nyx_str(&slot.name),
                signature: nyx_str(&slot.signature),
                function: Some(method_trampoline::<T>),
                receiver: slot.receiver.to_raw(),
                userdata: ptr::from_ref::<MethodSlot<T>>(slot).cast_mut().cast(),
            })
            .collect();
        let static_methods: Vec<sys::nyx_host_method> = slots
            .static_methods
            .iter()
            .map(|slot| sys::nyx_host_method {
                name: nyx_str(&slot.name),
                signature: nyx_str(&slot.signature),
                function: Some(static_trampoline::<T>),
                receiver: sys::NYX_RECEIVER_MUTABLE,
                userdata: ptr::from_ref::<StaticSlot<T>>(slot).cast_mut().cast(),
            })
            .collect();
        let operators: Vec<sys::nyx_host_operator> = slots
            .operators
            .iter()
            .map(|slot| sys::nyx_host_operator {
                symbol: nyx_str(&slot.symbol),
                signature: nyx_str(&slot.signature),
                function: Some(operator_trampoline::<T>),
                userdata: ptr::from_ref::<OperatorSlot<T>>(slot).cast_mut().cast(),
            })
            .collect();
        let properties: Vec<sys::nyx_host_property> = slots
            .properties
            .iter()
            .map(|slot| sys::nyx_host_property {
                name: nyx_str(&slot.name),
                type_: nyx_str(&slot.type_name),
                getter: Some(getter_trampoline::<T>),
                setter: slot.setter.as_ref().map(|_| setter_trampoline::<T> as _),
                userdata: ptr::from_ref::<PropertySlot<T>>(slot).cast_mut().cast(),
            })
            .collect();
        let constructor = slots.constructor.as_ref().map(|slot| sys::nyx_host_constructor {
            signature: nyx_str(&slot.signature),
            function: Some(constructor_trampoline::<T>),
            userdata: ptr::from_ref::<ConstructorSlot<T>>(slot).cast_mut().cast(),
        });
        let table = sys::nyx_host_type_table {
            methods: if methods.is_empty() { ptr::null() } else { methods.as_ptr() },
            method_count: methods.len(),
            static_methods: if static_methods.is_empty() { ptr::null() } else { static_methods.as_ptr() },
            static_method_count: static_methods.len(),
            constructor: constructor.as_ref().map_or(ptr::null(), ptr::from_ref),
            release_instance: if raw_layout.is_some() { None } else { Some(release_instance::<T>) },
            value_layout: raw_layout.as_ref().map_or(ptr::null(), ptr::from_ref),
            operators: if operators.is_empty() { ptr::null() } else { operators.as_ptr() },
            operator_count: operators.len(),
            properties: if properties.is_empty() { ptr::null() } else { properties.as_ptr() },
            property_count: properties.len(),
        };
        let userdata = Box::into_raw(slots);
        let mut raw_type = ptr::null_mut();
        let status = unsafe {
            sys::nyx_runtime_register_type(
                runtime.raw(),
                nyx_str(&name),
                &table,
                userdata.cast(),
                Some(free_boxed::<TypeSlots<T>>),
                &mut raw_type,
            )
        };
        if status != sys::NYX_OK {
            drop(unsafe { Box::from_raw(userdata) });
            runtime.check(status)?;
        }
        Ok(HostType { raw: raw_type, runtime, value_type: raw_layout.is_some(), marker: PhantomData })
    }
}

pub struct HostType<T> {
    raw: *mut sys::nyx_host_type,
    runtime: Runtime,
    value_type: bool,
    marker: PhantomData<fn() -> T>,
}

impl<T> Clone for HostType<T> {
    fn clone(&self) -> HostType<T> {
        HostType { raw: self.raw, runtime: self.runtime.clone(), value_type: self.value_type, marker: PhantomData }
    }
}

impl<T: 'static> HostType<T> {
    pub fn wrap(&self, instance: T) -> Result<Value> {
        wrap_instance(&self.runtime, self.raw, instance, self.value_type)
    }

    pub fn is_value_type(&self) -> bool {
        self.value_type
    }

    pub fn get<'value>(&self, value: &'value Value) -> Result<&'value T> {
        if !value.belongs_to(&self.runtime.inner) {
            return Err(Error::new(Status::InvalidArgument, "the value belongs to another runtime"));
        }
        let mut instance = ptr::null_mut();
        self.runtime.check(unsafe { sys::nyx_value_get_host_object(value.raw(), self.raw, &mut instance) })?;
        if instance.is_null() {
            return Err(Error::new(Status::Internal, "nyx-capi returned no instance"));
        }
        Ok(unsafe { &*instance.cast::<T>() })
    }
}
