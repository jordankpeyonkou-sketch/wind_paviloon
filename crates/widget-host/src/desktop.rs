#![deny(unsafe_op_in_unsafe_fn)]

use std::{error::Error, io};

use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use windows_sys::Win32::{
    Foundation::{GetLastError, SetLastError, HWND, LPARAM, WPARAM},
    UI::WindowsAndMessaging::{
        EnumWindows, FindWindowExW, FindWindowW, SendMessageTimeoutW, SetParent, SetWindowPos,
        GWL_STYLE, SMTO_NORMAL, SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOZORDER, WS_CHILD, WS_POPUP,
    },
};
use winit::window::Window;

const WORKERW_CREATE_MESSAGE: u32 = 0x052C;

pub fn attach_to_desktop(window: &Window) -> Result<(), Box<dyn Error>> {
    let desktop = find_desktop_worker().ok_or(
        "Could not find the Windows desktop WorkerW window. Start GlassDesk without --desktop.",
    )?;
    let hwnd = match window.window_handle()?.as_raw() {
        RawWindowHandle::Win32(handle) => handle.hwnd.get() as HWND,
        _ => return Err("The widget window does not expose a Win32 handle.".into()),
    };

    unsafe {
        SetLastError(0);
        let style = windows_sys::Win32::UI::WindowsAndMessaging::GetWindowLongPtrW(hwnd, GWL_STYLE);
        if style == 0 && GetLastError() != 0 {
            return Err(io::Error::last_os_error().into());
        }

        SetLastError(0);
        windows_sys::Win32::UI::WindowsAndMessaging::SetWindowLongPtrW(
            hwnd,
            GWL_STYLE,
            (style | WS_CHILD as isize) & !(WS_POPUP as isize),
        );
        if GetLastError() != 0 {
            return Err(io::Error::last_os_error().into());
        }

        SetLastError(0);
        SetParent(hwnd, desktop);
        if GetLastError() != 0 {
            return Err(io::Error::last_os_error().into());
        }
        if SetWindowPos(
            hwnd,
            std::ptr::null_mut(),
            32,
            32,
            460,
            190,
            SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED,
        ) == 0
        {
            return Err(io::Error::last_os_error().into());
        }
    }

    Ok(())
}

fn find_desktop_worker() -> Option<HWND> {
    let progman_class: Vec<u16> = "Progman\0".encode_utf16().collect();

    unsafe {
        let progman = FindWindowW(progman_class.as_ptr(), std::ptr::null());
        if progman.is_null() {
            return None;
        }

        let mut result = 0usize;
        SendMessageTimeoutW(
            progman,
            WORKERW_CREATE_MESSAGE,
            0 as WPARAM,
            0 as LPARAM,
            SMTO_NORMAL,
            1000,
            &mut result,
        );

        let mut worker: HWND = std::ptr::null_mut();
        let worker_ptr = &mut worker as *mut HWND as isize;
        EnumWindows(Some(find_worker_after_shell_view), worker_ptr);

        (!worker.is_null()).then_some(worker)
    }
}

unsafe extern "system" fn find_worker_after_shell_view(hwnd: HWND, parameter: LPARAM) -> i32 {
    let shell_view_class: Vec<u16> = "SHELLDLL_DefView\0".encode_utf16().collect();
    let worker_class: Vec<u16> = "WorkerW\0".encode_utf16().collect();
    let shell_view = unsafe {
        FindWindowExW(
            hwnd,
            std::ptr::null_mut(),
            shell_view_class.as_ptr(),
            std::ptr::null(),
        )
    };
    if shell_view.is_null() {
        return 1;
    }

    let worker = unsafe {
        FindWindowExW(
            std::ptr::null_mut(),
            hwnd,
            worker_class.as_ptr(),
            std::ptr::null(),
        )
    };
    if worker.is_null() {
        return 1;
    }

    let output = parameter as *mut HWND;
    if !output.is_null() {
        unsafe {
            *output = worker;
        }
    }
    0
}
