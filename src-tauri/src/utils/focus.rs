use windows_sys::Win32::UI::WindowsAndMessaging::{
    GetForegroundWindow, SetForegroundWindow, AllowSetForegroundWindow,
    GetWindowThreadProcessId,
};

/// 保存前一个前台窗口的信息，用于面板关闭后恢复焦点
pub struct FocusManager {
    /// HWND 在 windows-sys 0.52 中是 isize
    last_foreground_hwnd: isize,
    last_foreground_pid: u32,
}

impl FocusManager {
    pub fn new() -> Self {
        Self {
            last_foreground_hwnd: 0,
            last_foreground_pid: 0,
        }
    }

    /// 在显示面板前调用：记录当前前台窗口
    pub fn save_current_foreground(&mut self) {
        unsafe {
            let hwnd = GetForegroundWindow();
            if hwnd != 0 {
                self.last_foreground_hwnd = hwnd;
                let mut pid: u32 = 0;
                GetWindowThreadProcessId(hwnd, &mut pid);
                self.last_foreground_pid = pid;
            }
        }
    }

    /// 在面板关闭后调用：恢复焦点到之前的窗口
    pub fn restore_foreground(&self) {
        if self.last_foreground_hwnd != 0 {
            unsafe {
                // 先允许设置前台窗口
                if self.last_foreground_pid != 0 {
                    AllowSetForegroundWindow(self.last_foreground_pid);
                }
                // 恢复焦点 — SetForegroundWindow 接受 HWND (= isize)
                SetForegroundWindow(self.last_foreground_hwnd);
            }
        }
    }
}
