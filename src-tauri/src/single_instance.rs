//! 单实例机制：全局命名 Mutex（Global\PerchSingleInstance）确保跨完整性级别单实例；
//! 重复启动时通过命名管道 \\.\pipe\PerchFocus 请求主实例聚焦窗口。

use std::ptr;

use tauri::Manager;
use windows_sys::Win32::Foundation::{
    CloseHandle, ERROR_ALREADY_EXISTS, FALSE, GENERIC_WRITE, GetLastError, INVALID_HANDLE_VALUE,
    TRUE,
};
use windows_sys::Win32::Storage::FileSystem::{
    CreateFileW, OPEN_EXISTING, PIPE_ACCESS_INBOUND, ReadFile, WriteFile,
};
use windows_sys::Win32::System::Pipes::{
    ConnectNamedPipe, CreateNamedPipeW, DisconnectNamedPipe, PIPE_READMODE_BYTE, PIPE_TYPE_BYTE,
    PIPE_WAIT,
};
use windows_sys::Win32::System::Threading::{CreateMutexW, ReleaseMutex};

const MUTEX_NAME: &str = "Global\\PerchSingleInstance";
const PIPE_NAME: &str = "\\\\.\\pipe\\PerchFocus";

fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// 是否为重复启动的二级实例（Mutex 已存在）。
pub fn is_secondary_instance() -> bool {
    let name = to_wide(MUTEX_NAME);
    let handle = unsafe { CreateMutexW(ptr::null(), FALSE, name.as_ptr()) };
    if handle == 0 || handle == INVALID_HANDLE_VALUE {
        // 创建失败，保守放行
        return false;
    }
    let already = unsafe { GetLastError() } == ERROR_ALREADY_EXISTS;
    if already {
        // 二级实例：不持有该 Mutex
        unsafe {
            let _ = ReleaseMutex(handle);
            CloseHandle(handle);
        }
    } else {
        // 主实例：不关闭句柄，Mutex 所有权保持至进程结束
        let _ = handle;
    }
    already
}

/// 二级实例向主实例发送聚焦请求。
pub fn focus_primary() {
    let name = to_wide(PIPE_NAME);
    let h = unsafe {
        CreateFileW(
            name.as_ptr(),
            GENERIC_WRITE,
            0,
            ptr::null(),
            OPEN_EXISTING,
            0,
            0,
        )
    };
    if h != 0 && h != INVALID_HANDLE_VALUE {
        let buf: [u8; 1] = [1];
        let mut written: u32 = 0;
        unsafe {
            WriteFile(
                h,
                buf.as_ptr() as *const u8,
                1,
                &mut written,
                ptr::null_mut(),
            );
            CloseHandle(h);
        }
    }
}

/// 主实例启动命名管道监听，收到聚焦请求时拉起主窗口。
pub fn start_focus_listener(app: tauri::AppHandle) {
    let name = to_wide(PIPE_NAME);
    std::thread::spawn(move || {
        loop {
            let pipe = unsafe {
                CreateNamedPipeW(
                    name.as_ptr(),
                    PIPE_ACCESS_INBOUND,
                    PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT,
                    1,
                    0,
                    0,
                    0,
                    ptr::null(),
                )
            };
            if pipe == 0 || pipe == INVALID_HANDLE_VALUE {
                break;
            }
            if unsafe { ConnectNamedPipe(pipe, ptr::null_mut()) } == TRUE {
                let mut buf: [u8; 1] = [0];
                let mut read: u32 = 0;
                unsafe {
                    ReadFile(
                        pipe,
                        buf.as_mut_ptr() as *mut u8,
                        1,
                        &mut read,
                        ptr::null_mut(),
                    );
                }
                if let Some(w) = app.get_webview_window("main") {
                    let _ = w.show();
                    let _ = w.set_focus();
                }
            }
            unsafe {
                DisconnectNamedPipe(pipe);
                CloseHandle(pipe);
            }
        }
    });
}
