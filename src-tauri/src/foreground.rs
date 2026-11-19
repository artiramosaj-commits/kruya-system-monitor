fn get_process_name(path: &str) -> String {
    std::path::Path::new(path)
        .file_name()
        .map(|f| f.to_string_lossy().into_owned())
        .unwrap_or_else(|| "unknown".into())
}

#[cfg(target_os = "windows")]
mod platform_impl {
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;

    #[link(name = "user32")]
    extern "system" {
        fn GetForegroundWindow() -> isize;
        fn GetWindowTextW(hwnd: isize, lpString: *mut u16, nMaxCount: i32) -> i32;
        fn GetWindowThreadProcessId(hwnd: isize, lpdwProcessId: *mut u32) -> u32;
        fn GetLastInputInfo(plii: *mut LASTINPUTINFO) -> i32;
    }

    #[repr(C)]
    #[allow(non_snake_case)]
    struct LASTINPUTINFO {
        cbSize: u32,
        dwTime: u32,
    }

    #[link(name = "kernel32")]
    extern "system" {
        fn OpenProcess(dwDesiredAccess: u32, bInheritHandle: i32, dwProcessId: u32) -> isize;
        fn QueryFullProcessImageNameW(hProcess: isize, dwFlags: u32, lpExeName: *mut u16, lpdwSize: *mut u32) -> i32;
        fn CloseHandle(hObject: isize) -> i32;
        fn GetSystemPowerStatus(lpSystemPowerStatus: *mut SYSTEM_POWER_STATUS) -> i32;
        fn GetTickCount() -> u32;
    }

    #[repr(C)]
    struct SYSTEM_POWER_STATUS {
        ac_line_status: u8,
        battery_flag: u8,
        battery_life_percent: u8,
        system_status_flag: u8,
        battery_life_time: u32,
        battery_full_life_time: u32,
    }

    const PROCESS_QUERY_LIMITED_INFORMATION: u32 = 0x1000;

    fn get_process_path(pid: u32) -> Option<String> {
        unsafe {
            let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
            if handle == 0 || handle == -1 { return None; }

            let mut buf = [0u16; 4096];
            let mut size = 4096u32;
            let result = QueryFullProcessImageNameW(handle, 0, buf.as_mut_ptr(), &mut size);
            CloseHandle(handle);

            if result != 0 {
                let s = OsString::from_wide(&buf[..size as usize])
                    .to_string_lossy()
                    .into_owned();
                Some(s)
            } else {
                None
            }
        }
    }

    pub fn get_foreground_info() -> (String, String, u32) {
        unsafe {
            let hwnd = GetForegroundWindow();
            if hwnd == 0 {
                return (String::from("unknown"), String::new(), 0);
            }

            let mut pid: u32 = 0;
            GetWindowThreadProcessId(hwnd, &mut pid);

            let mut title_buf = [0u16; 512];
            let len = GetWindowTextW(hwnd, title_buf.as_mut_ptr(), 512);
            let title = if len > 0 {
                OsString::from_wide(&title_buf[..len as usize])
                    .to_string_lossy()
                    .into_owned()
            } else {
                String::new()
            };

            let name = if pid > 0 {
                get_process_path(pid)
                    .as_deref()
                    .map(super::get_process_name)
                    .unwrap_or_else(|| "unknown".into())
            } else {
                String::from("unknown")
            };

            (name, title, pid)
        }
    }

    pub fn get_battery_percentage() -> i8 {
        unsafe {
            let mut status: SYSTEM_POWER_STATUS = std::mem::zeroed();
            let result = GetSystemPowerStatus(&mut status);
            if result != 0 && status.battery_life_percent <= 100 {
                status.battery_life_percent as i8
            } else {
                -1
            }
        }
    }

    pub fn get_ac_line_status() -> i8 {
        unsafe {
            let mut status: SYSTEM_POWER_STATUS = std::mem::zeroed();
            let result = GetSystemPowerStatus(&mut status);
            if result != 0 {
                status.ac_line_status as i8
            } else {
                -1
            }
        }
    }

    pub fn get_idle_seconds() -> u64 {
        unsafe {
            let mut info = LASTINPUTINFO { cbSize: std::mem::size_of::<LASTINPUTINFO>() as u32, dwTime: 0 };
            if GetLastInputInfo(&mut info) != 0 {
                let tick = GetTickCount();
                if tick >= info.dwTime {
                    (tick - info.dwTime) as u64 / 1000
                } else {
                    0
                }
            } else {
                0
            }
        }
    }
}

#[cfg(target_os = "linux")]
mod platform_impl {
    use std::fs;
    use std::path::Path;
    use std::process::Command;

    pub fn get_foreground_info() -> (String, String, u32) {
        if let Ok(output) = Command::new("xdotool")
            .args(["getactivewindow", "getwindowpid"])
            .output()
        {
            if output.status.success() {
                let pid_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if let Ok(pid) = pid_str.parse::<u32>() {
                    let name = if let Ok(comm) = fs::read_to_string(format!("/proc/{}/comm", pid)) {
                        comm.trim().to_string()
                    } else {
                        "unknown".into()
                    };

                    let title_output = Command::new("xdotool")
                        .args(["getactivewindow", "getwindowname"])
                        .output();
                    let title = match title_output {
                        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).trim().to_string(),
                        _ => String::new(),
                    };

                    return (name, title, pid);
                }
            }
        }
        (String::from("unknown"), String::new(), 0)
    }

    pub fn get_battery_percentage() -> i8 {
        let power_dir = Path::new("/sys/class/power_supply");
        if !power_dir.exists() {
            return -1;
        }
        let entries = match fs::read_dir(power_dir) {
            Ok(e) => e,
            Err(_) => return -1,
        };
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if !name_str.starts_with("BAT") {
                continue;
            }
            let cap_path = entry.path().join("capacity");
            if let Ok(cap_str) = fs::read_to_string(&cap_path) {
                if let Ok(pct) = cap_str.trim().parse::<i8>() {
                    if (0..=100).contains(&pct) {
                        return pct;
                    }
                }
            }
        }
        -1
    }

    pub fn get_ac_line_status() -> i8 {
        let power_dir = Path::new("/sys/class/power_supply");
        if !power_dir.exists() { return -1; }
        if let Ok(entries) = fs::read_dir(power_dir) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with("AC") || name.starts_with("ADAPT") {
                    if let Ok(status) = fs::read_to_string(entry.path().join("online")) {
                        if let Ok(v) = status.trim().parse::<i8>() {
                            return v; // 0 = offline, 1 = online
                        }
                    }
                }
            }
        }
        -1
    }

    pub fn get_idle_seconds() -> u64 {
        if let Ok(output) = std::process::Command::new("xprintidle").output() {
            if let Ok(s) = String::from_utf8(output.stdout) {
                if let Ok(ms) = s.trim().parse::<u64>() {
                    return ms / 1000;
                }
            }
        }
        0
    }
}

#[cfg(target_os = "macos")]
mod platform_impl {
    use std::process::Command;

    pub fn get_foreground_info() -> (String, String, u32) {
        if let Ok(output) = Command::new("osascript")
            .args(["-e", r#"tell application "System Events" to get {name, pid} of first process whose frontmost is true"#])
            .output()
        {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
                let parts: Vec<&str> = stdout.splitn(2, ", ").collect();
                if parts.len() == 2 {
                    let name = parts[0].to_string();
                    if let Ok(pid) = parts[1].parse::<u32>() {
                        let title_script = r#"tell application "System Events" to get title of first window of first process whose frontmost is true"#;
                        let title_output = Command::new("osascript")
                            .args(["-e", title_script])
                            .output();
                        let title = match title_output {
                            Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).trim().to_string(),
                            _ => String::new(),
                        };
                        return (name, title, pid);
                    }
                }
            }
        }
        (String::from("unknown"), String::new(), 0)
    }

    pub fn get_battery_percentage() -> i8 {
        if let Ok(output) = Command::new("pmset")
            .args(["-g", "batt"])
            .output()
        {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                if let Some(pct_str) = line.split('%').next() {
                    if let Some(pct) = pct_str.rsplit(' ').next() {
                        if let Ok(val) = pct.parse::<i8>() {
                            if (0..=100).contains(&val) {
                                return val;
                            }
                        }
                    }
                }
            }
        }
        -1
    }

    pub fn get_ac_line_status() -> i8 {
        if let Ok(output) = Command::new("pmset").args(["-g", "batt"]).output() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                if line.contains("AC Power") { return 1; }
                if line.contains("Battery Power") { return 0; }
            }
        }
        -1
    }

    pub fn get_idle_seconds() -> u64 {
        use std::ffi::{c_void, c_uint};
        #[link(name = "CoreGraphics", kind = "framework")]
        extern "C" {
            fn CGEventSourceSecondsSinceLastEventType(
                sourceId: c_uint,
                eventType: c_uint,
            ) -> f64;
        }
        unsafe {
            let secs = CGEventSourceSecondsSinceLastEventType(0, 0);
            if secs >= 0.0 { secs as u64 } else { 0 }
        }
    }
}

#[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
mod platform_impl {
    pub fn get_foreground_info() -> (String, String, u32) {
        (String::from("unknown"), String::new(), 0)
    }

    pub fn get_battery_percentage() -> i8 {
        -1
    }

    pub fn get_ac_line_status() -> i8 {
        -1
    }

    pub fn get_idle_seconds() -> u64 {
        0
    }
}

pub use platform_impl::*;
