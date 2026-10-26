use std::fmt;

use nyx_sys as sys;

use crate::value::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Status {
    InvalidArgument,
    InvalidState,
    Refused,
    NotFound,
    Arguments,
    TypeMismatch,
    OutOfRange,
    Failure,
    Runtime,
    RestartRequired,
    Io,
    OutOfMemory,
    Internal,
    Unknown(u32),
}

impl Status {
    pub(crate) fn from_raw(raw: sys::nyx_status) -> Option<Status> {
        Some(match raw {
            sys::NYX_OK => return None,
            sys::NYX_ERROR_INVALID_ARGUMENT => Status::InvalidArgument,
            sys::NYX_ERROR_INVALID_STATE => Status::InvalidState,
            sys::NYX_ERROR_REFUSED => Status::Refused,
            sys::NYX_ERROR_NOT_FOUND => Status::NotFound,
            sys::NYX_ERROR_ARGUMENTS => Status::Arguments,
            sys::NYX_ERROR_TYPE_MISMATCH => Status::TypeMismatch,
            sys::NYX_ERROR_OUT_OF_RANGE => Status::OutOfRange,
            sys::NYX_ERROR_FAILURE => Status::Failure,
            sys::NYX_ERROR_RUNTIME => Status::Runtime,
            sys::NYX_ERROR_RESTART_REQUIRED => Status::RestartRequired,
            sys::NYX_ERROR_IO => Status::Io,
            sys::NYX_ERROR_OUT_OF_MEMORY => Status::OutOfMemory,
            sys::NYX_ERROR_INTERNAL => Status::Internal,
            #[allow(clippy::unnecessary_cast)]
            other => Status::Unknown(other as u32),
        })
    }

    pub fn name(self) -> &'static str {
        match self {
            Status::InvalidArgument => "NYX_ERROR_INVALID_ARGUMENT",
            Status::InvalidState => "NYX_ERROR_INVALID_STATE",
            Status::Refused => "NYX_ERROR_REFUSED",
            Status::NotFound => "NYX_ERROR_NOT_FOUND",
            Status::Arguments => "NYX_ERROR_ARGUMENTS",
            Status::TypeMismatch => "NYX_ERROR_TYPE_MISMATCH",
            Status::OutOfRange => "NYX_ERROR_OUT_OF_RANGE",
            Status::Failure => "NYX_ERROR_FAILURE",
            Status::Runtime => "NYX_ERROR_RUNTIME",
            Status::RestartRequired => "NYX_ERROR_RESTART_REQUIRED",
            Status::Io => "NYX_ERROR_IO",
            Status::OutOfMemory => "NYX_ERROR_OUT_OF_MEMORY",
            Status::Internal => "NYX_ERROR_INTERNAL",
            Status::Unknown(_) => "NYX_ERROR_UNKNOWN_STATUS",
        }
    }
}

#[derive(Clone)]
pub struct Error {
    pub status: Status,
    pub message: String,
    pub failure: Option<Value>,
}

impl Error {
    pub(crate) fn new(status: Status, message: impl Into<String>) -> Error {
        Error { status, message: message.into(), failure: None }
    }

    pub fn is_failure(&self) -> bool {
        self.status == Status::Failure
    }
}

impl fmt::Debug for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Error")
            .field("status", &self.status)
            .field("message", &self.message)
            .field("has_failure", &self.failure.is_some())
            .finish()
    }
}

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.status.name(), self.message)
    }
}

impl std::error::Error for Error {}

pub type Result<T> = std::result::Result<T, Error>;

pub enum CallbackError {
    Failure(Value),
    Refusal(String),
}

impl From<Error> for CallbackError {
    fn from(error: Error) -> CallbackError {
        match error.failure {
            Some(failure) => CallbackError::Failure(failure),
            None => CallbackError::Refusal(error.to_string()),
        }
    }
}

pub type CallbackResult = std::result::Result<Option<Value>, CallbackError>;
