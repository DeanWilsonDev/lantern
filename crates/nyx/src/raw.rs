use std::path::Path;

use nyx_sys as sys;

use crate::error::{Error, Result, Status};

pub(crate) fn nyx_str(text: &str) -> sys::nyx_str {
    sys::nyx_str { data: text.as_ptr().cast(), length: text.len() }
}

pub(crate) unsafe fn borrowed_bytes<'a>(text: sys::nyx_str) -> &'a [u8] {
    if text.data.is_null() || text.length == 0 {
        return &[];
    }
    std::slice::from_raw_parts(text.data.cast(), text.length)
}

pub(crate) unsafe fn owned_string(text: sys::nyx_str) -> String {
    String::from_utf8_lossy(borrowed_bytes(text)).into_owned()
}

pub(crate) fn path_text(path: &Path) -> Result<&str> {
    path.to_str()
        .ok_or_else(|| Error::new(Status::InvalidArgument, format!("the path {} isn't valid UTF-8", path.display())))
}
