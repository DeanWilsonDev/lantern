use nyx_sys as sys;

use crate::raw::owned_string;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Severity {
    Error,
    Warning,
    Note,
    Help,
}

impl Severity {
    fn from_raw(raw: sys::nyx_severity) -> Severity {
        match raw {
            sys::NYX_SEVERITY_WARNING => Severity::Warning,
            sys::NYX_SEVERITY_NOTE => Severity::Note,
            sys::NYX_SEVERITY_HELP => Severity::Help,
            _ => Severity::Error,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub severity: Severity,
    pub code: String,
    pub message: String,
    pub path: String,
    pub start_line: i32,
    pub start_column: i32,
    pub end_line: i32,
    pub end_column: i32,
    pub start_offset: u32,
    pub end_offset: u32,
}

impl Diagnostic {
    pub(crate) unsafe fn from_raw(raw: &sys::nyx_diagnostic) -> Diagnostic {
        Diagnostic {
            severity: Severity::from_raw(raw.severity),
            code: owned_string(raw.code),
            message: owned_string(raw.message),
            path: owned_string(raw.path),
            start_line: raw.start_line,
            start_column: raw.start_column,
            end_line: raw.end_line,
            end_column: raw.end_column,
            start_offset: raw.start_offset,
            end_offset: raw.end_offset,
        }
    }
}
