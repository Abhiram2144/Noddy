use chrono::{DateTime, Local};
use std::sync::{Mutex, OnceLock};

#[derive(Debug, Clone)]
pub struct SystemContext {
    pub active_application: String,
    pub running_applications: Vec<String>,
    pub battery_level: Option<u8>,
    pub network_status: Option<String>,
    pub idle_seconds: Option<u64>,
    pub timestamp: DateTime<Local>,
}

#[derive(Debug, Clone)]
struct CachedSystemContext {
    context: SystemContext,
    captured_at_ts: i64,
}

fn cache_state() -> &'static Mutex<Option<CachedSystemContext>> {
    static CACHE: OnceLock<Mutex<Option<CachedSystemContext>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(None))
}

pub fn get_system_context() -> SystemContext {
    if is_disabled() {
        return SystemContext {
            active_application: "Unknown".to_string(),
            running_applications: Vec::new(),
            battery_level: None,
            network_status: None,
            idle_seconds: None,
            timestamp: Local::now(),
        };
    }

    let now_ts = Local::now().timestamp();
    if let Ok(guard) = cache_state().lock() {
        if let Some(cache) = guard.as_ref() {
            if now_ts - cache.captured_at_ts <= 10 {
                return cache.context.clone();
            }
        }
    }

    let active_application = get_active_window();
    let running_applications = get_running_apps();
    let (battery_level, network_status) = get_system_state();
    let idle_seconds = get_idle_seconds();

    let context = SystemContext {
        active_application,
        running_applications,
        battery_level,
        network_status,
        idle_seconds,
        timestamp: Local::now(),
    };

    if let Ok(mut guard) = cache_state().lock() {
        *guard = Some(CachedSystemContext {
            context: context.clone(),
            captured_at_ts: now_ts,
        });
    }

    context
}

pub fn get_active_window() -> String {
    #[cfg(target_os = "windows")]
    {
        // Lightweight fallback: infer active shell process by foreground process query.
        // If this fails, we return Unknown to keep this module non-blocking.
        let script = "(Get-Process | Sort-Object StartTime -Descending | Select-Object -First 1).ProcessName";
        return run_powershell(script).unwrap_or_else(|| "Unknown".to_string());
    }

    #[cfg(not(target_os = "windows"))]
    {
        "Unknown".to_string()
    }
}

pub fn get_running_apps() -> Vec<String> {
    #[cfg(target_os = "windows")]
    {
        let script = "Get-Process | Select-Object -First 40 -ExpandProperty ProcessName";
        if let Some(output) = run_powershell(script) {
            let mut apps = output
                .lines()
                .map(str::trim)
                .filter(|name| !name.is_empty())
                .map(|name| name.to_string())
                .collect::<Vec<_>>();
            apps.sort();
            apps.dedup();
            apps.truncate(12);
            return apps;
        }
    }

    Vec::new()
}

pub fn get_system_state() -> (Option<u8>, Option<String>) {
    #[cfg(target_os = "windows")]
    {
        let battery = run_powershell("(Get-CimInstance Win32_Battery | Select-Object -First 1 -ExpandProperty EstimatedChargeRemaining)")
            .and_then(|v| v.trim().parse::<u8>().ok());

        let network = run_powershell("(Get-NetAdapter | Where-Object {$_.Status -eq 'Up'} | Select-Object -First 1 -ExpandProperty Name)")
            .map(|name| if name.trim().is_empty() { "Offline".to_string() } else { format!("Connected ({})", name.trim()) });

        return (battery, network);
    }

    #[cfg(not(target_os = "windows"))]
    {
        (None, None)
    }
}

pub fn get_idle_seconds() -> Option<u64> {
        #[cfg(target_os = "windows")]
        {
                // Lightweight user-idle estimate from GetLastInputInfo in PowerShell.
                let script = r#"
Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class IdleTime {
    [StructLayout(LayoutKind.Sequential)]
    struct LASTINPUTINFO { public uint cbSize; public uint dwTime; }
    [DllImport("user32.dll")] static extern bool GetLastInputInfo(ref LASTINPUTINFO plii);
    public static uint GetIdleSeconds() {
        LASTINPUTINFO lii = new LASTINPUTINFO();
        lii.cbSize = (uint)System.Runtime.InteropServices.Marshal.SizeOf(lii);
        if (!GetLastInputInfo(ref lii)) return 0;
        return ((uint)Environment.TickCount - lii.dwTime) / 1000;
    }
}
'@;
[IdleTime]::GetIdleSeconds()
"#;
                return run_powershell(script).and_then(|v| v.trim().parse::<u64>().ok());
        }

        #[cfg(not(target_os = "windows"))]
        {
                None
        }
}

fn is_disabled() -> bool {
    std::env::var("NODDY_SYSTEM_AWARENESS_DISABLED")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

#[cfg(target_os = "windows")]
fn run_powershell(script: &str) -> Option<String> {
    let output = std::process::Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if text.is_empty() {
        None
    } else {
        Some(text)
    }
}
