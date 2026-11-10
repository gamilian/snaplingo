use global_hotkey::hotkey::HotKey as Shortcut;
use std::ffi::c_void;

#[repr(C)]
struct HotkeyId {
    signature: u32,
    id: u32,
}

#[link(name = "Carbon", kind = "framework")]
extern "C" {
    fn GetApplicationEventTarget() -> *mut c_void;
    fn CreateEvent(
        allocator: *const c_void,
        class: u32,
        kind: u32,
        time: f64,
        attributes: u32,
        event: *mut *mut c_void,
    ) -> i32;
    fn SetEventParameter(
        event: *mut c_void,
        name: u32,
        kind: u32,
        size: u32,
        data: *const c_void,
    ) -> i32;
    fn SendEventToEventTarget(event: *mut c_void, target: *mut c_void) -> i32;
    fn ReleaseEvent(event: *mut c_void);
}

pub(in crate::test_support) fn send_hotkey(shortcut: &Shortcut, released: bool) {
    let id = HotkeyId {
        signature: u32::from_be_bytes(*b"htrs"),
        id: shortcut.id(),
    };
    unsafe {
        let mut event = std::ptr::null_mut();
        assert_eq!(
            CreateEvent(
                std::ptr::null(),
                u32::from_be_bytes(*b"keyb"),
                if released { 6 } else { 5 },
                0.0,
                0,
                &mut event
            ),
            0
        );
        assert_eq!(
            SetEventParameter(
                event,
                u32::from_be_bytes(*b"----"),
                u32::from_be_bytes(*b"hkid"),
                std::mem::size_of::<HotkeyId>() as u32,
                &id as *const _ as *const c_void
            ),
            0
        );
        let result = SendEventToEventTarget(event, GetApplicationEventTarget());
        ReleaseEvent(event);
        assert_eq!(result, 0);
    }
}
