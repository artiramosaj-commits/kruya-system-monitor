use crate::models::*;
use chrono::{Local, Utc};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use sysinfo::{Disks, Networks, System};

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;

#[cfg(target_os = "windows")]
static TEMP_PS_FAILED: AtomicBool = AtomicBool::new(false);

#[cfg(target_os = "windows")]
fn run_ps(script: &str) -> Option<String> {
    let output = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .ok()?;
    if output.status.success() {
        let s = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if s.is_empty() { None } else { Some(s) }
    } else {
        None
    }
}

#[cfg(target_os = "windows")]
fn reg_read_str(subkey: &str, value: &str) -> String {
    use windows::Win32::System::Registry::*;
    use windows::Win32::Foundation::ERROR_SUCCESS;
    use windows::core::HSTRING;
    unsafe {
        let mut handle = HKEY::default();
        let status = RegOpenKeyExW(
            HKEY_LOCAL_MACHINE,
            &HSTRING::from(subkey),
            0,
            KEY_READ,
            &mut handle,
        );
        if status != ERROR_SUCCESS {
            return String::new();
        }
        let mut buf = [0u16; 2048];
        let mut size = (buf.len() * 2) as u32;
        let status = RegQueryValueExW(
            handle,
            &HSTRING::from(value),
            None,
            None,
            Some(buf.as_mut_ptr() as *mut u8),
            Some(&mut size),
        );
        let _ = RegCloseKey(handle);
        if status == ERROR_SUCCESS && size > 0 {
            let len = (size as usize / 2).min(buf.len());
            String::from_utf16_lossy(&buf[..len]).trim_end_matches('\0').to_string()
        } else {
            String::new()
        }
    }
}

#[cfg(target_os = "windows")]
fn reg_read_dword(subkey: &str, value: &str) -> u32 {
    use windows::Win32::System::Registry::*;
    use windows::Win32::Foundation::ERROR_SUCCESS;
    use windows::core::HSTRING;
    unsafe {
        let mut handle = HKEY::default();
        let status = RegOpenKeyExW(
            HKEY_LOCAL_MACHINE,
            &HSTRING::from(subkey),
            0,
            KEY_READ,
            &mut handle,
        );
        if status != ERROR_SUCCESS {
            return 0;
        }
        let mut data: u32 = 0;
        let mut size = 4u32;
        let mut kind = REG_VALUE_TYPE::default();
        let status = RegQueryValueExW(
            handle,
            &HSTRING::from(value),
            None,
            Some(&mut kind),
            Some(&mut data as *mut u32 as *mut u8),
            Some(&mut size),
        );
        let _ = RegCloseKey(handle);
        if status == ERROR_SUCCESS && kind == REG_DWORD {
            data
        } else {
            0
        }
    }
}

#[cfg(target_os = "windows")]
fn smbios_string(data: &[u8], str_start: usize, idx: u8) -> String {
    if idx == 0 || str_start >= data.len() {
        return String::new();
    }
    let mut pos = str_start;
    let mut cur: u8 = 1;
    while cur < idx && pos < data.len() {
        if data[pos] == 0 {
            cur += 1;
        }
        pos += 1;
    }
    if cur != idx || pos >= data.len() {
        return String::new();
    }
    let start = pos;
    let mut end = pos;
    while end < data.len() && data[end] != 0 {
        end += 1;
    }
    String::from_utf8_lossy(&data[start..end]).to_string()
}

#[cfg(target_os = "windows")]
fn smbios_skip_strings(data: &[u8], str_start: usize) -> usize {
    let mut next = str_start;
    let mut null_count = 0u32;
    while next < data.len() {
        if data[next] == 0 {
            null_count += 1;
            if null_count >= 2 {
                next += 1;
                break;
            }
        } else {
            null_count = 0;
        }
        next += 1;
    }
    // Align to 2-byte boundary
    (next + 1) & !1
}

#[cfg(target_os = "windows")]
fn reg_read_binary(subkey: &str, value: &str) -> Vec<u8> {
    use windows::Win32::System::Registry::*;
    use windows::Win32::Foundation::ERROR_SUCCESS;
    use windows::core::HSTRING;
    unsafe {
        let mut handle = HKEY::default();
        let status = RegOpenKeyExW(
            HKEY_LOCAL_MACHINE, &HSTRING::from(subkey), 0, KEY_READ, &mut handle,
        );
        if status != ERROR_SUCCESS { return vec![]; }
        let mut size = 0u32;
        let status = RegQueryValueExW(
            handle, &HSTRING::from(value), None, None, None, Some(&mut size),
        );
        if status != ERROR_SUCCESS || size == 0 { let _ = RegCloseKey(handle); return vec![]; }
        let mut buf = vec![0u8; size as usize];
        let status = RegQueryValueExW(
            handle, &HSTRING::from(value), None, None, Some(buf.as_mut_ptr()), Some(&mut size),
        );
        let _ = RegCloseKey(handle);
        if status == ERROR_SUCCESS { buf.truncate(size as usize); buf } else { vec![] }
    }
}

#[cfg(target_os = "windows")]
fn read_smbios_data() -> Vec<u8> {
    use windows::Win32::System::SystemInformation::{GetSystemFirmwareTable, FIRMWARE_TABLE_PROVIDER};
    // Try GetSystemFirmwareTable first
    let from_api = unsafe {
        let sig = FIRMWARE_TABLE_PROVIDER(0x52534D42u32);
        let size = GetSystemFirmwareTable(sig, 0, None);
        if size >= 4 {
            let mut b = vec![0u8; size as usize];
            let written = GetSystemFirmwareTable(sig, 0, Some(&mut b));
            if written >= 4 { Some(b) } else { None }
        } else { None }
    };
    if let Some(mut data) = from_api {
        // Skip SMBIOS entry point if present
        let dmi_start = if data.len() >= 4 && &data[0..4] == b"_SM3_" { 24 }
            else if data.len() >= 4 && &data[0..4] == b"_SM_" { 31 } else { 0 };
        if dmi_start > 0 && dmi_start < data.len() { data.drain(..dmi_start); }
        if data.len() >= 4 { return data; }
    }
    // Fallback: read from registry cache
    let reg = reg_read_binary(
        r"SYSTEM\CurrentControlSet\Services\mssmbios\Data", "SMBiosData",
    );
    if reg.len() >= 4 { return reg; }
    vec![]
}

#[cfg(target_os = "windows")]
fn parse_smbios_ram() -> (u32, String, u32, u32) {
    let data = read_smbios_data();
    if data.len() < 4 { return (0, String::new(), 0, 0); }

    let mut speed = 0u32;
    let mut form_factor = String::new();
    let mut slots_used = 0u32;
    let mut total_slots = 0u32;
    let mut i = 0usize;
    while i + 4 <= data.len() {
        let stype = data[i];
        let slen = data[i + 1] as usize;
        if slen < 2 { break; }
        if stype == 0x7F { break; }
        if stype == 0x11 && slen >= 0x17 && i + 0x17 < data.len() {
            let total_width = u16::from_ne_bytes([data[i + 0x08], data[i + 0x09]]);
            if total_width > 0 {
                let dev_speed = u16::from_ne_bytes([data[i + 0x15], data[i + 0x16]]);
                speed = speed.max(dev_speed as u32);
                let ff = data[i + 0x0E];
                let ff_str = match ff {
                    0x09 => "DIMM",
                    0x0D => "SODIMM",
                    0x0C => "RIMM",
                    0x0F => "FB-DIMM",
                    _ => "",
                };
                if !ff_str.is_empty() { form_factor = ff_str.to_string(); }
                slots_used += 1;
            }
        } else if stype == 0x10 && slen >= 0x0F && i + 0x0E < data.len() {
            total_slots = u16::from_ne_bytes([data[i + 0x0D], data[i + 0x0E]]) as u32;
        }
        let next = smbios_skip_strings(&data, i + slen);
        if next <= i { break; }
        i = next;
    }
    // Fallback: PowerShell for speed and slots if SMBIOS returned nothing
    if speed == 0 || total_slots == 0 || slots_used == 0 {
        if let Some(out) = run_ps(
            "Get-CimInstance Win32_PhysicalMemory | ForEach-Object { \"$($_.Speed)|$($_.FormFactor)\" }"
        ) {
            for line in out.lines() {
                let line = line.trim();
                if line.is_empty() { continue; }
                let parts: Vec<&str> = line.split('|').collect();
                if parts.len() >= 2 {
                    if let Ok(s) = parts[0].trim().parse::<u32>() {
                        if s > 0 {
                            speed = speed.max(s);
                            slots_used += 1;
                        }
                    }
                    let ff = parts[1].trim();
                    if !ff.is_empty() && ff != "0" {
                        let ff_clean = match ff {
                            "8" | "DIMM" => "DIMM",
                            "12" | "SODIMM" => "SODIMM",
                            "9" | "RAMBUS" => "RAMBUS",
                            _ => ff,
                        };
                        form_factor = ff_clean.to_string();
                    }
                }
            }
        }
        if total_slots == 0 {
            if let Some(out) = run_ps(
                "Get-CimInstance Win32_PhysicalMemoryArray | Select-Object -ExpandProperty MemoryDevices"
            ) {
                total_slots = out.trim().parse::<u32>().unwrap_or(0);
            }
        }
    }
    (speed, form_factor, slots_used, total_slots)
}

#[cfg(target_os = "windows")]
fn get_smbios_serial() -> String {
    let data = read_smbios_data();
    if data.len() < 4 { return String::new(); }
    let mut i = 0usize;
    while i + 4 <= data.len() {
        let stype = data[i];
        let slen = data[i + 1] as usize;
        if slen < 2 { break; }
        if stype == 0x7F { break; }
        if stype == 0x01 && slen >= 0x08 && i + 0x07 < data.len() {
            let idx = data[i + 0x07];
            let serial = smbios_string(&data, i + slen, idx);
            if !serial.is_empty() && serial != "To be filled by O.E.M." {
                return serial;
            }
        }
        let next = smbios_skip_strings(&data, i + slen);
        if next <= i { break; }
        i = next;
    }
    String::new()
}

fn classify_gpu_type(name: &str, dedicated_memory_mb: u64) -> String {
    let lower = name.to_lowercase();
    if dedicated_memory_mb >= 1024 {
        return "Dedicated".into();
    }
    // Integrated GPU keywords (Intel, AMD APU, Apple)
    if lower.contains("intel")
        && (lower.contains("hd graphics")
            || lower.contains("uhd graphics")
            || lower.contains("iris")
            || lower.contains("xe graphics")
            || lower.contains("arc"))
    {
        if lower.contains("arc") {
            return "Dedicated".into(); // Intel Arc are discrete
        }
        return "Integrated".into();
    }
    if lower.contains("amd")
        && (lower.contains("radeon(tm) graphics")
            || lower.contains("radeon vega")
            || lower.contains("radeon hd"))
        && !lower.contains("rx")
        && !lower.contains("pro")
    {
        return "Integrated".into(); // AMD APU integrated
    }
    if lower.contains("apple") && (lower.contains("m1") || lower.contains("m2") || lower.contains("m3") || lower.contains("m4")) {
        return "Integrated".into();
    }
    if lower.contains("nvidia") || lower.contains("quadro") || lower.contains("tesla") {
        return "Dedicated".into();
    }
    if lower.contains("amd") && (lower.contains("rx") || lower.contains("pro") || lower.contains("fire")) {
        return "Dedicated".into();
    }
    if dedicated_memory_mb > 512 {
        return "Dedicated".into();
    }
    if dedicated_memory_mb > 0 {
        return "Dedicated".into();
    }
    "Integrated".into()
}

fn get_gpu_info() -> crate::models::GpuInfo {
    #[cfg(target_os = "windows")]
    {
        use windows::Win32::Graphics::Dxgi::*;
        unsafe {
            if let Ok(factory) = CreateDXGIFactory1::<IDXGIFactory1>() {
                let mut idx = 0u32;
                loop {
                    if let Ok(adapter) = factory.EnumAdapters1(idx) {
                        if let Ok(desc) = adapter.GetDesc1() {
                            if desc.DedicatedVideoMemory > 0 {
                                let name = String::from_utf16_lossy(&desc.Description).trim_end_matches('\0').to_string();
                                let mem_mb = (desc.DedicatedVideoMemory / (1024 * 1024)) as u64;
                                let driver = reg_read_str(
                                    r"SYSTEM\CurrentControlSet\Control\Class\{4d36e968-e325-11ce-bfc1-08002be10318}\0000",
                                    "DriverVersion",
                                );
                                let gpu_type = classify_gpu_type(&name, mem_mb);
                                return crate::models::GpuInfo { name, memory_mb: mem_mb, driver_version: driver, gpu_type };
                            }
                        }
                    } else {
                        break;
                    }
                    idx += 1;
                }
            }
        }
    }
    #[cfg(target_os = "linux")]
    {
        if let Ok(output) = Command::new("lspci").arg("-v").output() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                let trimmed = line.trim();
                if trimmed.contains("VGA compatible") || trimmed.contains("Display controller") {
                    if let Some(name_part) = trimmed.split(':').nth(2) {
                        let name = name_part.trim().to_string();
                        if !name.is_empty() {
                            let gpu_type = classify_gpu_type(&name, 0);
                            return crate::models::GpuInfo {
                                name,
                                memory_mb: 0,
                                driver_version: String::new(),
                                gpu_type,
                            };
                        }
                    }
                }
            }
        }
    }
    #[cfg(target_os = "macos")]
    {
        if let Ok(output) = Command::new("system_profiler")
            .args(["SPDisplaysDataType"])
            .output()
        {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let mut name = String::new();
            let mut vram = 0u64;
            for line in stdout.lines() {
                let trimmed = line.trim();
                if trimmed.contains("Chipset Model:") {
                    name = trimmed.trim_start_matches("Chipset Model:").trim().to_string();
                } else if trimmed.contains("VRAM") {
                    let vram_str = trimmed.trim_start_matches("VRAM (Total):").trim();
                    if let Some(mb) = parse_mac_vram(vram_str) {
                        vram = mb;
                    }
                }
            }
            if !name.is_empty() {
                let gpu_type = classify_gpu_type(&name, vram);
                return crate::models::GpuInfo {
                    name,
                    memory_mb: vram,
                    driver_version: String::new(),
                    gpu_type,
                };
            }
        }
    }
    crate::models::GpuInfo {
        name: "Not available".into(),
        memory_mb: 0,
        driver_version: String::new(),
        gpu_type: "Unknown".into(),
    }
}

#[cfg(target_os = "macos")]
fn parse_mac_vram(s: &str) -> Option<u64> {
    let s = s.trim().to_lowercase();
    if s.ends_with("mb") {
        s.trim_end_matches("mb").trim().parse::<u64>().ok()
    } else if s.ends_with("gb") {
        s.trim_end_matches("gb").trim().parse::<f64>().ok().map(|v| (v * 1024.0) as u64)
    } else {
        None
    }
}

fn get_cpu_temperature() -> f32 {
    #[cfg(target_os = "windows")]
    {
        use sysinfo::Components;
        let components = Components::new_with_refreshed_list();
        for c in &components {
            if let Some(temp) = c.temperature() {
                if temp > 0.0 && temp < 120.0 {
                    return temp;
                }
            }
        }
        // PowerShell fallback (only once if it already failed)
        if !TEMP_PS_FAILED.load(Ordering::Relaxed) {
            if let Some(out) = run_ps(
                "Get-CimInstance -Namespace root/wmi -ClassName MSAcpi_ThermalZoneTemperature | Select-Object -ExpandProperty CurrentTemperature"
            ) {
                for line in out.lines() {
                    let trimmed = line.trim();
                    if let Ok(val) = trimmed.parse::<f32>() {
                        let celsius = (val / 10.0) - 273.15;
                        if celsius > 0.0 && celsius < 120.0 { return celsius; }
                    }
                }
            }
            TEMP_PS_FAILED.store(true, Ordering::Relaxed);
        }
    }
    #[cfg(target_os = "linux")]
    {
        if let Ok(entries) = std::fs::read_dir("/sys/class/thermal") {
            for entry in entries.flatten() {
                let name = entry.file_name();
                let name_str = name.to_string_lossy();
                if name_str.starts_with("thermal_zone") {
                    let temp_path = entry.path().join("temp");
                    if let Ok(temp_str) = std::fs::read_to_string(&temp_path) {
                        if let Ok(millicelsius) = temp_str.trim().parse::<f32>() {
                            let celsius = millicelsius / 1000.0;
                            if celsius > 0.0 && celsius < 120.0 {
                                return celsius;
                            }
                        }
                    }
                }
            }
        }
    }
    #[cfg(target_os = "macos")]
    {
        if let Ok(output) = Command::new("pmset").args(["-g", "therm"]).output() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                let trimmed = line.trim();
                if let Some(temp_str) = trimmed.split(':').nth(1) {
                    if let Some(val_str) = temp_str.trim().split(' ').next() {
                        if let Ok(celsius) = val_str.parse::<f32>() {
                            if celsius > 0.0 && celsius < 120.0 {
                                return celsius;
                            }
                        }
                    }
                }
            }
        }
        // Fallback: try sysctl on macOS for CPU temperature
        if let Ok(output) = Command::new("sysctl")
            .args(["-n", "machdep.xcpm.cpu_thermal_level"])
            .output()
        {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if let Ok(level) = stdout.trim().parse::<f32>() {
                // thermal_level: 0=normal, 1=moderate, 2=heavy, ...
                if level > 0.0 {
                    return level * 30.0; // rough mapping: level * ~30°C above baseline
                }
            }
        }
    }
    0.0
}

fn get_cpu_extended_info() -> (u32, u64, String) {
    #[cfg(target_os = "windows")]
    {
        use windows::Win32::System::SystemInformation::*;
        unsafe {
            let mut info = SYSTEM_INFO::default();
            GetNativeSystemInfo(&mut info);
            let proc_arch = info.Anonymous.Anonymous.wProcessorArchitecture;
            let raw = proc_arch.0;
            let arch = match raw {
                0 => "x86".into(),
                5 => "ARM".into(),
                6 => "IA-64".into(),
                9 => "x64".into(),
                _ => format!("Unknown({})", raw),
            };
            let mut cores = 0u32;
            let mut needed = 0u32;
            let _ = GetLogicalProcessorInformation(None, &mut needed);
            if needed > 0 {
                let count = (needed as usize) / std::mem::size_of::<SYSTEM_LOGICAL_PROCESSOR_INFORMATION>();
                let mut vec = vec![SYSTEM_LOGICAL_PROCESSOR_INFORMATION::default(); count];
                if GetLogicalProcessorInformation(Some(vec.as_mut_ptr()), &mut needed).is_ok() {
                    for entry in &vec {
                        if entry.Relationship == RelationProcessorCore {
                            cores += 1;
                        }
                    }
                }
            }
            let freq = reg_read_dword(
                r"HARDWARE\DESCRIPTION\System\CentralProcessor\0",
                "~MHz",
            ) as u64;
            return (cores, freq, arch);
        }
    }
    #[cfg(target_os = "linux")]
    {
        let cores = std::fs::read_to_string("/proc/cpuinfo")
            .ok()
            .and_then(|s| {
                s.lines()
                    .filter(|l| l.starts_with("cpu cores"))
                    .filter_map(|l| l.split(':').nth(1))
                    .filter_map(|v| v.trim().parse::<u32>().ok())
                    .next()
            })
            .unwrap_or(0);
        let freq = std::fs::read_to_string("/sys/devices/system/cpu/cpu0/cpufreq/cpuinfo_max_freq")
            .ok()
            .and_then(|s| s.trim().parse::<u64>().ok())
            .map(|khz| khz / 1000)
            .unwrap_or(0);
        let arch = Command::new("uname").arg("-m").output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| s.trim().to_string())
            .unwrap_or_default();
        return (cores, freq, arch);
    }
    #[cfg(target_os = "macos")]
    {
        let cores = Command::new("sysctl")
            .args(["-n", "machdep.cpu.core_count"])
            .output().ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .and_then(|s| s.trim().parse::<u32>().ok())
            .unwrap_or(0);
        let freq = Command::new("sysctl")
            .args(["-n", "hw.cpufrequency_max"])
            .output().ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .and_then(|s| s.trim().parse::<u64>().ok())
            .map(|hz| hz / 1_000_000)
            .unwrap_or(0);
        let arch = Command::new("uname").arg("-m").output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| s.trim().to_string())
            .unwrap_or_default();
        return (cores, freq, arch);
    }
    (0, 0, String::new())
}

fn get_system_manufacturer_info() -> (String, String, String, String) {
    #[cfg(target_os = "windows")]
    {
        let bios = r"HARDWARE\DESCRIPTION\System\BIOS";
        let vendor = reg_read_str(bios, "SystemManufacturer");
        let model = reg_read_str(bios, "SystemProductName");
        let mut serial = reg_read_str(bios, "SystemSerialNumber");
        let sku = reg_read_str(bios, "SystemSKU");
        if serial.is_empty() {
            serial = get_smbios_serial();
        }
        if serial.is_empty() {
            if let Some(out) = run_ps(
                "Get-CimInstance Win32_ComputerSystemProduct | Select-Object -ExpandProperty IdentifyingNumber"
            ) {
                let s = out.trim().to_string();
                if !s.is_empty() { serial = s; }
            }
        }
        if !model.is_empty() {
            return (vendor, model, serial, sku);
        }
    }
    #[cfg(target_os = "linux")]
    {
        let vendor = std::fs::read_to_string("/sys/class/dmi/id/sys_vendor").unwrap_or_default().trim().to_string();
        let model = std::fs::read_to_string("/sys/class/dmi/id/product_name").unwrap_or_default().trim().to_string();
        let serial = std::fs::read_to_string("/sys/class/dmi/id/product_serial").unwrap_or_default().trim().to_string();
        let sku = std::fs::read_to_string("/sys/class/dmi/id/product_sku").unwrap_or_default().trim().to_string();
        if !model.is_empty() {
            return (vendor, model, serial, sku);
        }
    }
    #[cfg(target_os = "macos")]
    {
        if let Ok(output) = Command::new("system_profiler").args(["SPHardwareDataType"]).output() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let mut vendor = String::new();
            let mut model = String::new();
            let mut serial = String::new();
            for line in stdout.lines() {
                let t = line.trim();
                if t.contains("Model Name:") { model = t.trim_start_matches("Model Name:").trim().to_string(); vendor.clone_from(&model); }
                if t.contains("Model Identifier:") { model = t.trim_start_matches("Model Identifier:").trim().to_string(); }
                if t.contains("Serial Number:") { serial = t.trim_start_matches("Serial Number:").trim().to_string(); }
            }
            if !model.is_empty() {
                return (vendor, model, serial, String::new());
            }
        }
    }
    (String::new(), String::new(), String::new(), String::new())
}

fn get_bios_info() -> (String, String, String) {
    #[cfg(target_os = "windows")]
    {
        let bios = r"HARDWARE\DESCRIPTION\System\BIOS";
        let vendor = reg_read_str(bios, "BIOSVendor");
        let version = reg_read_str(bios, "BIOSVersion");
        let date = reg_read_str(bios, "BIOSReleaseDate");
        if !vendor.is_empty() {
            return (vendor, version, date);
        }
    }
    #[cfg(target_os = "linux")]
    {
        let vendor = std::fs::read_to_string("/sys/class/dmi/id/bios_vendor").unwrap_or_default().trim().to_string();
        let version = std::fs::read_to_string("/sys/class/dmi/id/bios_version").unwrap_or_default().trim().to_string();
        let date = std::fs::read_to_string("/sys/class/dmi/id/bios_date").unwrap_or_default().trim().to_string();
        return (vendor, version, date);
    }
    #[cfg(target_os = "macos")]
    {
        if let Ok(output) = Command::new("system_profiler").args(["SPHardwareDataType"]).output() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let mut version = String::new();
            for line in stdout.lines() {
                let t = line.trim();
                if t.contains("Boot ROM Version:") {
                    version = t.trim_start_matches("Boot ROM Version:").trim().to_string();
                    return ("Apple".into(), version, String::new());
                }
            }
        }
    }
    (String::new(), String::new(), String::new())
}

fn get_motherboard_info() -> (String, String) {
    #[cfg(target_os = "windows")]
    {
        let bios = r"HARDWARE\DESCRIPTION\System\BIOS";
        let manu = reg_read_str(bios, "BaseBoardManufacturer");
        let prod = reg_read_str(bios, "BaseBoardProduct");
        if !manu.is_empty() {
            return (manu, prod);
        }
    }
    #[cfg(target_os = "linux")]
    {
        let manu = std::fs::read_to_string("/sys/class/dmi/id/board_vendor").unwrap_or_default().trim().to_string();
        let prod = std::fs::read_to_string("/sys/class/dmi/id/board_name").unwrap_or_default().trim().to_string();
        return (manu, prod);
    }
    #[cfg(target_os = "macos")]
    {
        if let Ok(output) = Command::new("system_profiler").args(["SPHardwareDataType"]).output() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                let t = line.trim();
                if t.contains("Model Identifier:") {
                    let id = t.trim_start_matches("Model Identifier:").trim().to_string();
                    return ("Apple".into(), id);
                }
            }
        }
    }
    (String::new(), String::new())
}

fn get_ram_info() -> (u32, String, u32, u32) {
    #[cfg(target_os = "windows")]
    {
        return parse_smbios_ram();
    }
    #[cfg(target_os = "linux")]
    {
        let mut speed = 0u32;
        let mut form_factor = String::new();
        let mut slots_used = 0u32;
        let mut total_slots = 0u32;
        if let Ok(output) = Command::new("dmidecode").args(["-t", "memory"]).output() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let mut in_mem = false;
            for line in stdout.lines() {
                let t = line.trim();
                if t.contains("Memory Device") { in_mem = true; }
                if t.contains("Memory Controller") { in_mem = false; }
                if in_mem {
                    if t.contains("Speed:") && t.contains("MHz") {
                        if let Some(val) = t.split(':').nth(1).and_then(|s| s.trim().trim_start_matches("Unknown").trim().split_whitespace().next()) {
                            if let Ok(s) = val.parse::<u32>() { speed = speed.max(s); slots_used += 1; }
                        }
                    }
                    if t.contains("Form Factor:") {
                        let ff = t.split(':').nth(1).unwrap_or("").trim();
                        if !ff.is_empty() && ff != "Unknown" { form_factor = ff.to_string(); }
                    }
                }
                if t.starts_with("Number Of Devices:") {
                    total_slots = t.split(':').nth(1).and_then(|s| s.trim().parse::<u32>().ok()).unwrap_or(0);
                }
            }
        } else {
            // Fallback: try decoding /proc
            let meminfo = std::fs::read_to_string("/proc/meminfo").unwrap_or_default();
        }
        return (speed, form_factor, slots_used, total_slots);
    }
    #[cfg(target_os = "macos")]
    {
        let mut speed = 0u32;
        let mut form_factor = String::new();
        let mut slots_used = 0u32;
        let mut total_slots = 0u32;
        if let Ok(output) = Command::new("system_profiler").args(["SPMemoryDataType"]).output() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                let t = line.trim();
                if t.contains("Speed:") {
                    let s = t.trim_start_matches("Speed:").trim();
                    if let Some(mhz) = s.split_whitespace().next().and_then(|v| v.parse::<u32>().ok()) {
                        speed = speed.max(mhz);
                    }
                }
                if t.contains("Type:") && t.contains("DDR") {
                    form_factor = t.trim_start_matches("Type:").trim().to_string();
                }
                if t.contains("Status:") {
                    let status = t.trim_start_matches("Status:").trim();
                    if status == "OK" { slots_used += 1; }
                }
            }
        }
        return (speed, form_factor, slots_used, total_slots);
    }
    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    { (0, String::new(), 0, 0) }
}

fn get_os_extended_info() -> (String, String, String, String) {
    #[cfg(target_os = "windows")]
    {
        use windows::Win32::System::SystemInformation::*;
        let nt = r"SOFTWARE\Microsoft\Windows NT\CurrentVersion";
        let edition = reg_read_str(nt, "ProductName");
        let build = reg_read_str(nt, "CurrentBuild");
            let install_raw = reg_read_dword(nt, "InstallDate");
            let install_str = if install_raw > 0 {
                let secs = install_raw as i64;
                chrono::DateTime::from_timestamp(secs, 0)
                    .map(|d| d.format("%Y-%m-%d").to_string())
                    .unwrap_or_default()
            } else {
            String::new()
        };
        unsafe {
            let mut info = SYSTEM_INFO::default();
            GetNativeSystemInfo(&mut info);
            let raw = info.Anonymous.Anonymous.wProcessorArchitecture.0;
            let arch = match raw {
                0 => "x86",
                5 => "ARM",
                6 => "IA-64",
                9 => "x64",
                _ => "Unknown",
            }.to_string();
            return (edition, build, install_str, arch);
        }
    }
    #[cfg(target_os = "linux")]
    {
        let edition = std::fs::read_to_string("/etc/os-release")
            .ok()
            .and_then(|f| {
                f.lines()
                    .find(|l| l.starts_with("PRETTY_NAME="))
                    .and_then(|l| l.split('=').nth(1))
                    .map(|v| v.trim_matches('"').to_string())
            })
            .unwrap_or_default();
        let build = std::fs::read_to_string("/proc/version_signature")
            .ok()
            .map(|s| s.trim().to_string())
            .unwrap_or_default();
        let arch = Command::new("uname").arg("-m").output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| s.trim().to_string())
            .unwrap_or_default();
        return (edition, build, String::new(), arch);
    }
    #[cfg(target_os = "macos")]
    {
        let edition = Command::new("sw_vers").arg("-productName").output()
            .ok().and_then(|o| String::from_utf8(o.stdout).ok()).map(|s| s.trim().to_string()).unwrap_or_default();
        let version = Command::new("sw_vers").arg("-productVersion").output()
            .ok().and_then(|o| String::from_utf8(o.stdout).ok()).map(|s| s.trim().to_string()).unwrap_or_default();
        let build = Command::new("sw_vers").arg("-buildVersion").output()
            .ok().and_then(|o| String::from_utf8(o.stdout).ok()).map(|s| s.trim().to_string()).unwrap_or_default();
        let arch = Command::new("uname").arg("-m").output()
            .ok().and_then(|o| String::from_utf8(o.stdout).ok()).map(|s| s.trim().to_string()).unwrap_or_default();
        return (format!("macOS {}", edition), build, String::new(), arch);
    }
    (String::new(), String::new(), String::new(), String::new())
}

fn get_displays() -> Vec<DisplayInfo> {
    let mut displays = Vec::new();
    #[cfg(target_os = "windows")]
    {
        use windows::Win32::Graphics::Gdi::*;
        unsafe {
            let mut dev = DISPLAY_DEVICEW::default();
            dev.cb = std::mem::size_of::<DISPLAY_DEVICEW>() as u32;
            let mut dev_idx = 0u32;
            while EnumDisplayDevicesW(None, dev_idx, &mut dev, 0).as_bool() {
                if (dev.StateFlags & DISPLAY_DEVICE_ATTACHED_TO_DESKTOP) != 0 {
                    let name = String::from_utf16_lossy(&dev.DeviceString).trim_end_matches('\0').to_string();
                    let dev_name = String::from_utf16_lossy(&dev.DeviceName).trim_end_matches('\0').to_string();
                    let mut dm = DEVMODEW::default();
                    dm.dmSize = std::mem::size_of::<DEVMODEW>() as u16;
                    if EnumDisplaySettingsW(&windows::core::HSTRING::from(&dev_name), ENUM_CURRENT_SETTINGS, &mut dm).as_bool() {
                        let w = dm.dmPelsWidth;
                        let h = dm.dmPelsHeight;
                        let rr = dm.dmDisplayFrequency;
                        if w > 0 && h > 0 {
                            displays.push(DisplayInfo {
                                name,
                                resolution_width: w,
                                resolution_height: h,
                                refresh_rate: rr,
                                is_primary: displays.is_empty(),
                            });
                        }
                    }
                }
                dev.cb = std::mem::size_of::<DISPLAY_DEVICEW>() as u32;
                dev_idx += 1;
            }
        }
    }
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    {
        if let Ok(output) = Command::new("xrandr").arg("--query").output() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                if line.contains(" connected") {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    let name = parts[0].to_string();
                    let mut w = 0u32;
                    let mut h = 0u32;
                    let mut rr = 0u32;
                    if let Some(res_part) = parts.iter().find(|p| p.contains('x')) {
                        let dims: Vec<&str> = res_part.split('x').collect();
                        w = dims.first().and_then(|s| s.parse::<u32>().ok()).unwrap_or(0);
                        if let Some(h_part) = dims.get(1) {
                            h = h_part.split('+').next().and_then(|s| s.parse::<u32>().ok()).unwrap_or(0);
                        }
                    }
                    if let Some(rr_part) = parts.iter().find(|p| p.ends_with("Hz")) {
                        rr = rr_part.trim_end_matches("Hz").trim_end_matches('*').parse::<u32>().unwrap_or(0);
                    }
                    if w > 0 && h > 0 {
                        displays.push(DisplayInfo {
                            name,
                            resolution_width: w,
                            resolution_height: h,
                            refresh_rate: rr,
                            is_primary: displays.is_empty(),
                        });
                    }
                }
            }
        }
    }
    displays
}

#[cfg(target_os = "windows")]
pub fn get_startup_programs() -> Vec<StartupProgram> {
    let script = "Get-CimInstance Win32_StartupCommand | Select Name, Command, Location | ConvertTo-Json -Compress";
        if let Some(json) = run_ps(script) {
        #[derive(serde::Deserialize)]
        #[allow(non_snake_case)]
        struct RawStartup {
            Name: Option<String>,
            Command: Option<String>,
            Location: Option<String>,
        }
        if let Ok(entries) = serde_json::from_str::<Vec<RawStartup>>(&json) {
            return entries.into_iter().map(|r| StartupProgram {
                name: r.Name.unwrap_or_default(),
                command: r.Command.unwrap_or_default(),
                location: r.Location.unwrap_or_default(),
                enabled: true,
            }).collect();
        }
    }
    Vec::new()
}

#[cfg(not(target_os = "windows"))]
pub fn get_startup_programs() -> Vec<StartupProgram> {
    Vec::new()
}

#[cfg(target_os = "windows")]
pub fn get_battery_info() -> Option<BatteryInfo> {
    run_ps(
        "Get-CimInstance Win32_Battery | Select-Object @{N='DesignCapacity';E={$_.DesignCapacity}}, \
         @{N='FullChargeCapacity';E={$_.FullChargeCapacity}}, \
         @{N='CycleCount';E={$_.CycleCount}} | ConvertTo-Json"
    ).and_then(|json| {
        #[derive(serde::Deserialize)]
        #[allow(non_snake_case)]
        struct RawBattery {
            DesignCapacity: Option<u32>,
            FullChargeCapacity: Option<u32>,
            CycleCount: Option<u32>,
        }
        serde_json::from_str::<RawBattery>(&json).ok().map(|r| {
            let wear = r.DesignCapacity.map(|d| {
                if d > 0 {
                    let full = r.FullChargeCapacity.unwrap_or(0) as f32;
                    ((d as f32 - full) / d as f32 * 100.0).max(0.0)
                } else { 0.0 }
            }).unwrap_or(0.0);
            BatteryInfo {
                design_capacity_mwh: r.DesignCapacity.unwrap_or(0),
                full_charge_capacity_mwh: r.FullChargeCapacity.unwrap_or(0),
                cycle_count: r.CycleCount.unwrap_or(0),
                wear_level_pct: wear,
            }
        })
    })
}

#[cfg(not(target_os = "windows"))]
pub fn get_battery_info() -> Option<BatteryInfo> {
    None
}

pub struct TelemetryCollector {
    system: System,
    cpu_brand: String,
    gpu_info: crate::models::GpuInfo,
    sys_info: crate::models::SystemInfo,
    inited: bool,
}

impl TelemetryCollector {
    pub fn new() -> Self {
        TelemetryCollector {
            system: System::new(),
            cpu_brand: String::new(),
            gpu_info: crate::models::GpuInfo {
                name: String::new(), memory_mb: 0, driver_version: String::new(), gpu_type: String::new(),
            },
            sys_info: crate::models::SystemInfo {
                os_name: String::new(), os_version: String::new(), kernel_version: String::new(),
                hostname: String::new(), cpu_brand: String::new(), cpu_cores: 0, total_ram_mb: 0,
                cpu_physical_cores: 0, cpu_max_frequency_mhz: 0, cpu_architecture: String::new(),
                system_manufacturer: String::new(), system_model: String::new(),
                system_serial: String::new(), system_sku: String::new(), bios_vendor: String::new(),
                bios_version: String::new(), bios_date: String::new(),
                motherboard_manufacturer: String::new(), motherboard_model: String::new(),
                ram_speed_mhz: 0, ram_form_factor: String::new(), ram_slots_used: 0, ram_slots_total: 0,
                os_edition: String::new(), os_build: String::new(), os_install_date: String::new(),
                os_architecture: String::new(),
            },
            inited: false,
        }
    }

    fn ensure_init(&mut self) {
        if self.inited { return; }
        self.inited = true;
        self.system.refresh_cpu_all();
        self.system.refresh_memory();
        self.cpu_brand = self.system.cpus().first().map(|c| c.brand().to_string()).unwrap_or_default();
        let gpu_info = get_gpu_info();
        let (phys_cores, max_freq, cpu_arch) = get_cpu_extended_info();
        let (sys_manu, sys_model, sys_serial, sys_sku) = get_system_manufacturer_info();
        let (bios_vendor, bios_version, bios_date) = get_bios_info();
        let (mb_manu, mb_model) = get_motherboard_info();
        let (ram_speed, ram_factor, ram_used, ram_total) = get_ram_info();
        let (os_edition, os_build, os_install, os_arch) = get_os_extended_info();
        self.gpu_info = gpu_info;
        self.sys_info = crate::models::SystemInfo {
            os_name: if os_edition.is_empty() { System::long_os_version().unwrap_or_default() } else { os_edition.clone() },
            os_version: System::kernel_version().unwrap_or_default(),
            kernel_version: System::kernel_version().unwrap_or_default(),
            hostname: System::host_name().unwrap_or_default(),
            cpu_brand: self.cpu_brand.clone(),
            cpu_cores: self.system.cpus().len() as u32,
            total_ram_mb: self.system.total_memory() / (1024 * 1024),
            cpu_physical_cores: phys_cores,
            cpu_max_frequency_mhz: max_freq,
            cpu_architecture: cpu_arch,
            system_manufacturer: sys_manu,
            system_model: sys_model,
            system_serial: sys_serial,
            system_sku: sys_sku,
            bios_vendor,
            bios_version,
            bios_date,
            motherboard_manufacturer: mb_manu,
            motherboard_model: mb_model,
            ram_speed_mhz: ram_speed,
            ram_form_factor: ram_factor,
            ram_slots_used: ram_used,
            ram_slots_total: ram_total,
            os_edition,
            os_build,
            os_install_date: os_install,
            os_architecture: os_arch,
        };
    }

    pub fn collect_snapshot(&mut self) -> SystemSnapshot {
        self.ensure_init();
        self.system.refresh_all();

        let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

        // CPU
        let cpu_usage = self.system.global_cpu_usage();
        let per_core: Vec<f32> = self.system.cpus().iter().map(|c| c.cpu_usage()).collect();
        let cpu_freq: Vec<u64> = self.system.cpus().iter().map(|c| c.frequency()).collect();

        // Memory
        let total_mem = self.system.total_memory();
        let used_mem = self.system.used_memory();
        let avail_mem = self.system.available_memory();
        let total_swap = self.system.total_swap();
        let used_swap = self.system.used_swap();
        let mem_pct = if total_mem > 0 {
            ((used_mem as f64 / total_mem as f64) * 100.0) as u8
        } else {
            0
        };

        // Disk
        let disks = Disks::new_with_refreshed_list();
        let mut total_disk_gb = 0u64;
        let mut used_disk_gb = 0u64;
        let mut disk_read = 0u64;
        let mut disk_write = 0u64;
        let mut drives = Vec::new();

        for disk in &disks {
            let total = disk.total_space();
            let available = disk.available_space();
            let used = total.saturating_sub(available);
            let total_gb = total / (1024 * 1024 * 1024);
            let used_gb = used / (1024 * 1024 * 1024);
            total_disk_gb += total_gb;
            used_disk_gb += used_gb;
            disk_read += disk.usage().read_bytes;
            disk_write += disk.usage().written_bytes;
            let pct = if total_gb > 0 {
                ((used_gb as f64 / total_gb as f64) * 100.0) as u8
            } else {
                0
            };
            let fs_type = disk.file_system().to_string_lossy().to_string();
            drives.push(DriveInfo {
                mount_point: disk.mount_point().to_string_lossy().to_string(),
                total_gb,
                used_gb,
                usage_percent: pct,
                filesystem_type: fs_type,
                is_removable: disk.is_removable(),
            });
        }

        let disk_pct = if total_disk_gb > 0 {
            ((used_disk_gb as f64 / total_disk_gb as f64) * 100.0) as u8
        } else {
            0
        };

        // Network
        let networks = Networks::new_with_refreshed_list();
        let mut net_down = 0u64;
        let mut net_up = 0u64;
        let mut interfaces = Vec::new();
        for (name, net) in &networks {
            net_down += net.total_received();
            net_up += net.total_transmitted();
            interfaces.push(NetworkInterface {
                name: name.clone(),
                total_received: net.total_received(),
                total_transmitted: net.total_transmitted(),
            });
        }

        // Uptime
        let boot_time = System::boot_time();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let uptime_secs = now.saturating_sub(boot_time);

        // Processes
        let total_procs = self.system.processes().len() as u32;
        let running_procs = self
            .system
            .processes()
            .iter()
            .filter(|(_, p)| matches!(p.status(), sysinfo::ProcessStatus::Run))
            .count() as u32;

        let mut process_list: Vec<ProcessInfo> = self
            .system
            .processes()
            .iter()
            .map(|(pid, p)| {
                let name = p.name().to_string_lossy().to_string();
                let mem_mb = p.memory() / (1024 * 1024);
                let cpu = p.cpu_usage();
                let thread_count = if let Some(tasks) = p.tasks() {
                    tasks.len() as u32
                } else {
                    p.thread_kind().map(|_| 1u32).unwrap_or(0)
                };
                let runtime = p.run_time();

                ProcessInfo {
                    name: if name.ends_with(".exe") || cfg!(not(target_os = "windows")) {
                        name
                    } else {
                        format!("{}.exe", name)
                    },
                    cpu_percent: cpu,
                    memory_mb: mem_mb,
                    threads: thread_count,
                    pid: pid.as_u32(),
                    runtime_seconds: runtime,
                    cmd: p.cmd().iter().map(|s| s.to_string_lossy()).collect::<Vec<_>>().join(" "),
                    path: p.exe().map(|e| e.to_string_lossy().to_string()).unwrap_or_default(),
                }
            })
            .collect();

        process_list.sort_by_key(|b| std::cmp::Reverse(b.memory_mb));
        process_list.truncate(50);

        // Foreground
        let (fg_name, fg_title, fg_pid) = crate::foreground::get_foreground_info();
        let foreground = ForegroundInfo {
            name: fg_name,
            title: fg_title,
            pid: fg_pid,
        };

        SystemSnapshot {
            timestamp,
            system: SystemMetrics {
                cpu_total_usage: cpu_usage,
                per_core_usage: per_core,
                cpu_frequency_mhz: cpu_freq,
                cpu_brand: self.cpu_brand.clone(),
                memory_total_mb: total_mem / (1024 * 1024),
                memory_used_mb: used_mem / (1024 * 1024),
                memory_available_mb: avail_mem / (1024 * 1024),
                memory_total_usage: mem_pct,
                swap_total_mb: total_swap / (1024 * 1024),
                swap_used_mb: used_swap / (1024 * 1024),
                disk_total_gb: total_disk_gb,
                disk_used_gb: used_disk_gb,
                disk_usage_percent: disk_pct,
                disk_read_bytes: disk_read,
                disk_write_bytes: disk_write,
                network_down_bytes: net_down,
                network_up_bytes: net_up,
                temperature_celsius: get_cpu_temperature(),
                battery_percent: crate::foreground::get_battery_percentage(),
                ac_line_status: crate::foreground::get_ac_line_status(),
                uptime_seconds: uptime_secs,
                total_processes: total_procs,
                running_processes: running_procs,
            },
            processes: process_list,
            foreground,
            drives,
            gpu: self.gpu_info.clone(),
            interfaces,
            sys_info: self.sys_info.clone(),
            displays: get_displays(),
        }
    }
}
