use std::ptr;

use nyx_sys::*;

#[test]
fn creates_and_destroys_a_runtime() {
    unsafe {
        assert_eq!(nyx_capi_version(), NYX_CAPI_VERSION as i32);
        let mut runtime: *mut nyx_runtime = ptr::null_mut();
        assert_eq!(nyx_runtime_create(&mut runtime), NYX_OK);
        assert!(!runtime.is_null());

        let mut value: *mut nyx_value = ptr::null_mut();
        assert_eq!(nyx_value_new_i32(runtime, 42, &mut value), NYX_OK);
        let mut read = 0i32;
        assert_eq!(nyx_value_get_i32(value, &mut read), NYX_OK);
        assert_eq!(read, 42);
        nyx_value_release(value);

        assert_eq!(nyx_runtime_destroy(runtime), NYX_OK);
    }
}
