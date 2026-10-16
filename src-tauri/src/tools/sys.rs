use crate::error::Result;
use crate::tools::ExecutionOutput;
use serde_json::json;
use sysinfo::{CpuRefreshKind, MemoryRefreshKind, ProcessesToUpdate, RefreshKind, System};

pub fn system_info() -> Result<ExecutionOutput> {
    let mut sys = System::new_with_specifics(
        RefreshKind::nothing()
            .with_cpu(CpuRefreshKind::nothing().with_cpu_usage())
            .with_memory(MemoryRefreshKind::everything()),
    );
    sys.refresh_cpu_usage();
    sys.refresh_memory();
    let _ = sys.refresh_processes(ProcessesToUpdate::All, true);

    let os_name = System::name().unwrap_or_else(|| "unknown".into());
    let os_ver = System::os_version().unwrap_or_default();
    let host = System::host_name().unwrap_or_default();
    let uptime_s = System::uptime();
    let total_mem = sys.total_memory();
    let used_mem = sys.used_memory();
    let mem_pct = if total_mem > 0 {
        (used_mem as f64 / total_mem as f64 * 100.0).round() as u64
    } else {
        0
    };
    let load = sys.global_cpu_usage();
    let procs = sys.processes().len();

    let battery = battery_percent();

    Ok(ExecutionOutput {
        text: format!(
            "{os_name} {os_ver} on {host}. CPU load {load:.0}%, memory {mem_pct}% of {total_mem} bytes, {procs} processes, up {uptime_s}s{}.",
            battery.as_ref().map(|b| format!(", battery {b}%")).unwrap_or_default()
        ),
        data: Some(json!({
            "os": format!("{os_name} {os_ver}"),
            "host": host,
            "cpu_load_percent": load,
            "memory_used_bytes": used_mem,
            "memory_total_bytes": total_mem,
            "memory_percent": mem_pct,
            "processes": procs,
            "uptime_seconds": uptime_s,
            "battery_percent": battery,
        })),
    })
}

fn battery_percent() -> Option<i64> {
    let out = std::process::Command::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            "(Get-CimInstance Win32_Battery | Select-Object -First 1 -ExpandProperty EstimatedChargeRemaining)",
        ])
        .output()
        .ok()?;
    let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
    s.parse::<i64>().ok()
}

pub fn time_info() -> Result<ExecutionOutput> {
    let now = chrono::Local::now();
    let text = now.format("%A, %B %d %Y, %H:%M:%S %Z").to_string();
    let tz = now.format("%Z").to_string();
    Ok(ExecutionOutput {
        text: text.clone(),
        data: Some(json!({
            "local": text,
            "rfc3339": now.to_rfc3339(),
            "timezone": tz,
            "weekday": now.format("%A").to_string(),
        })),
    })
}
