//! Smart Profiles via Win32 SetWinEventHook (EVENT_SYSTEM_FOREGROUND, 0% polling).

use std::path::Path;
use std::sync::{Arc, OnceLock};
use windows::core::PWSTR;
use windows::Win32::Foundation::{CloseHandle, HWND};
use windows::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows::Win32::UI::Accessibility::{SetWinEventHook, UnhookWinEvent, HWINEVENTHOOK};
use windows::Win32::UI::WindowsAndMessaging::{
    GetWindowThreadProcessId, EVENT_SYSTEM_FOREGROUND, WINEVENT_OUTOFCONTEXT,
};

static FOREGROUND_CB: OnceLock<Arc<dyn Fn(String) + Send + Sync>> = OnceLock::new();

pub struct ForegroundHook {
    hook: HWINEVENTHOOK,
}

impl ForegroundHook {
    pub fn install<F>(cb: F) -> Option<Self>
    where
        F: Fn(String) + Send + Sync + 'static,
    {
        let _ = FOREGROUND_CB.set(Arc::new(cb));
        unsafe {
            let hook = SetWinEventHook(
                EVENT_SYSTEM_FOREGROUND,
                EVENT_SYSTEM_FOREGROUND,
                None,
                Some(win_event_proc),
                0,
                0,
                WINEVENT_OUTOFCONTEXT,
            );
            if hook.is_invalid() {
                None
            } else {
                Some(Self { hook })
            }
        }
    }
}

impl Drop for ForegroundHook {
    fn drop(&mut self) {
        unsafe {
            let _ = UnhookWinEvent(self.hook);
        }
    }
}

unsafe extern "system" fn win_event_proc(
    _h_win_event_hook: HWINEVENTHOOK,
    _event: u32,
    hwnd: HWND,
    _id_object: i32,
    _id_child: i32,
    _id_event_thread: u32,
    _dwms_event_time: u32,
) {
    if hwnd.0.is_null() {
        return;
    }
    let mut pid: u32 = 0;
    GetWindowThreadProcessId(hwnd, Some(&mut pid));
    if pid == 0 {
        return;
    }
    let Ok(proc_h) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) else {
        return;
    };
    let mut buf = [0u16; 512];
    let mut len = buf.len() as u32;
    if QueryFullProcessImageNameW(proc_h, PROCESS_NAME_WIN32, PWSTR(buf.as_mut_ptr()), &mut len).is_ok() {
        let full = String::from_utf16_lossy(&buf[..len as usize]);
        if let Some(exe) = Path::new(&full).file_name().and_then(|s| s.to_str()) {
            if let Some(cb) = FOREGROUND_CB.get() {
                cb(exe.to_string());
            }
        }
    }
    let _ = CloseHandle(proc_h);
}
