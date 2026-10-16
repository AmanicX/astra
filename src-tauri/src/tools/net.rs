use crate::error::{AppError, Result};
use crate::tools::ExecutionOutput;
use serde_json::json;

fn ps(script: &str) -> Result<String> {
    let out = std::process::Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .output()
        .map_err(|e| AppError::msg(format!("powershell: {e}")))?;
    let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if !out.status.success() && stdout.is_empty() {
        let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
        return Err(AppError::msg(if err.is_empty() { "command failed".into() } else { err }));
    }
    Ok(stdout)
}

// ---------------- Wi-Fi ----------------

const RADIO_HELPERS: &str = "
Add-Type -AssemblyName System.Runtime.WindowsRuntime | Out-Null;
$asTaskGeneric = ([System.WindowsRuntimeSystemExtensions].GetMethods() | Where-Object { $_.Name -eq 'AsTask' -and $_.GetParameters().Count -eq 1 -and $_.GetParameters()[0].ParameterType.Name -eq 'IAsyncOperation`1' })[0];
function Await($op, $type) { $t = $asTaskGeneric.MakeGenericMethod($type).Invoke($null, @($op)); $t.Wait(-1) | Out-Null; $t.Result }
";

pub fn wifi_status() -> Result<ExecutionOutput> {
    // Connection state via CIM (Unicode-safe).
    let conn = ps(
        "Get-CimInstance -ClassName MSFT_NetConnectionProfile -Namespace root/StandardCimv2 -ErrorAction SilentlyContinue | \
         Where-Object { $_.InterfaceAlias -match 'Wi-Fi|WLAN|Wireless' } | \
         Select-Object -First 1 | ConvertTo-Json -Compress",
    )
    .unwrap_or_default();

    let parsed: serde_json::Value = serde_json::from_str(&conn).unwrap_or(serde_json::Value::Null);
    let ssid = parsed.get("Name").and_then(|v| v.as_str()).unwrap_or("").to_string();

    let radio_script = format!(
        "{RADIO_HELPERS};\
         $null = [Windows.Devices.Radios.Radio,Windows.System.Devices,ContentType=WindowsRuntime];\
         $radios = Await ([Windows.Devices.Radios.Radio]::GetRadiosAsync()) ([System.Collections.Generic.IReadOnlyList[Windows.Devices.Radios.Radio]]);\
         ($radios | Where-Object {{ $_.Kind -eq [Windows.Devices.Radios.RadioKind]::WiFi }} | Select-Object -First 1).State"
    );
    let radio = ps(&radio_script).unwrap_or_default();
    let on = radio.trim().eq_ignore_ascii_case("On");

    let signal = if ssid.is_empty() {
        None
    } else {
        wifi_signal_percent()
    };

    let text = if !on {
        "Wi-Fi radio is off.".to_string()
    } else if ssid.is_empty() {
        "Wi-Fi is on but not connected.".to_string()
    } else {
        format!(
            "Connected to \"{ssid}\"{}.",
            signal.map(|s| format!(" (signal {s}%)")).unwrap_or_default()
        )
    };

    Ok(ExecutionOutput {
        text,
        data: Some(json!({
            "radio_on": on,
            "connected_ssid": if ssid.is_empty() { None } else { Some(ssid) },
            "signal_percent": signal,
        })),
    })
}

fn wifi_signal_percent() -> Option<i64> {
    // netsh output is localized; parse the numeric percentage next to a % sign.
    let out = std::process::Command::new("netsh")
        .args(["wlan", "show", "interfaces"])
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&out.stdout);
    for line in text.lines() {
        if line.contains('%') && line.to_lowercase().contains("signal") {
            if let Some(idx) = line.find('%') {
                let digits: String = line[..idx]
                    .chars()
                    .rev()
                    .take_while(|c| c.is_ascii_digit())
                    .collect::<String>()
                    .chars()
                    .rev()
                    .collect();
                if let Ok(v) = digits.parse::<i64>() {
                    return Some(v);
                }
            }
        }
    }
    None
}

pub async fn wifi_set(action: &str, ssid: Option<&str>) -> Result<ExecutionOutput> {
    match action {
        "on" => set_radio(true),
        "off" => set_radio(false),
        "connect" => {
            let ssid = ssid.unwrap_or("").trim();
            if ssid.is_empty() {
                return Err(AppError::Precondition("connect requires an ssid".into()));
            }
            let safe = ssid.replace('"', "");
            let out = std::process::Command::new("netsh")
                .args(["wlan", "connect", &format!("name={safe}")])
                .output()
                .map_err(|e| AppError::msg(format!("netsh: {e}")))?;
            let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if !out.status.success() {
                return Err(AppError::msg(format!(
                    "could not connect to \"{safe}\" (only saved profiles can be connected without a password): {text}"
                )));
            }
            Ok(ExecutionOutput {
                text: format!("Connecting to saved network \"{safe}\". {text}"),
                data: Some(json!({"ssid": safe})),
            })
        }
        other => Err(AppError::Precondition(format!("unknown wifi action '{other}'"))),
    }
}

fn set_radio(on: bool) -> Result<ExecutionOutput> {
    let state = if on { "On" } else { "Off" };
    let script = format!(
        "{RADIO_HELPERS};\
         $null = [Windows.Devices.Radios.Radio,Windows.System.Devices,ContentType=WindowsRuntime];\
         $null = [Windows.Devices.Radios.RadioState,Windows.System.Devices,ContentType=WindowsRuntime];\
         $radios = Await ([Windows.Devices.Radios.Radio]::GetRadiosAsync()) ([System.Collections.Generic.IReadOnlyList[Windows.Devices.Radios.Radio]]);\
         $r = $radios | Where-Object {{ $_.Kind -eq [Windows.Devices.Radios.RadioKind]::WiFi }} | Select-Object -First 1;\
         if (-not $r) {{ throw 'no wi-fi radio found' }};\
         $res = Await ($r.SetStateAsync([Windows.Devices.Radios.RadioState]::{state})) ([Windows.Devices.Radios.RadioAccessStatus]);\
         if ($res -ne [Windows.Devices.Radios.RadioAccessStatus]::Allowed) {{ throw \"radio change not permitted: $res\" }}; 'ok'"
    );
    ps(&script).map_err(|e| {
        AppError::msg(format!(
            "could not turn Wi-Fi {state} (Windows may require enabling the radio manually): {e}"
        ))
    })?;
    Ok(ExecutionOutput {
        text: format!("Wi-Fi radio turned {state}."),
        data: Some(json!({"radio_on": on})),
    })
}

pub async fn wifi_scan() -> Result<ExecutionOutput> {
    let out = tokio::process::Command::new("netsh")
        .args(["wlan", "show", "networks", "mode=bssid"])
        .output()
        .await
        .map_err(|e| AppError::msg(format!("netsh: {e}")))?;
    let text = String::from_utf8_lossy(&out.stdout).to_string();
    let mut networks: Vec<String> = Vec::new();
    let ignore = [
        "network type", "authentication", "encryption", "signal", "radio type", "channel",
        "bssid", "max speed", "basic rates", "other rates", "physical address", "network kind",
    ];
    for line in text.lines() {
        let Some((key, value)) = line.split_once(':') else { continue };
        let k = key.trim().trim_start_matches(|c: char| !c.is_alphanumeric());
        let v = value.trim();
        if v.is_empty() {
            continue;
        }
        let kl = k.to_lowercase();
        if ignore.iter().any(|i| kl.contains(i)) {
            continue;
        }
        if kl.contains("ssid") || k.chars().all(|c| !c.is_ascii_digit()) {
            // Localized "SSID" labels still start with SSID in most locales.
            if kl.contains("ssid") {
                networks.push(v.to_string());
            }
        }
    }
    networks.sort();
    networks.dedup();
    Ok(ExecutionOutput {
        text: if networks.is_empty() {
            "No networks found (or Wi-Fi is off).".into()
        } else {
            format!("Nearby networks: {}", networks.join(", "))
        },
        data: Some(json!({"networks": networks})),
    })
}

// ---------------- Bluetooth ----------------

pub fn bt_status() -> Result<ExecutionOutput> {
    let script = format!(
        "{RADIO_HELPERS};\
         $null = [Windows.Devices.Radios.Radio,Windows.System.Devices,ContentType=WindowsRuntime];\
         $radios = Await ([Windows.Devices.Radios.Radio]::GetRadiosAsync()) ([System.Collections.Generic.IReadOnlyList[Windows.Devices.Radios.Radio]]);\
         ($radios | Where-Object {{ $_.Kind -eq [Windows.Devices.Radios.RadioKind]::Bluetooth }} | Select-Object -First 1).State"
    );
    let state = ps(&script).unwrap_or_default();
    let trimmed = state.trim().to_string();
    if trimmed.is_empty() {
        return Ok(ExecutionOutput {
            text: "No Bluetooth radio found.".into(),
            data: Some(json!({"available": false})),
        });
    }
    let on = trimmed.eq_ignore_ascii_case("On");
    Ok(ExecutionOutput {
        text: format!("Bluetooth radio is {trimmed}."),
        data: Some(json!({"available": true, "on": on, "state": trimmed})),
    })
}

pub async fn bt_set(on: bool) -> Result<ExecutionOutput> {
    let state = if on { "On" } else { "Off" };
    let script = format!(
        "{RADIO_HELPERS};\
         $null = [Windows.Devices.Radios.Radio,Windows.System.Devices,ContentType=WindowsRuntime];\
         $null = [Windows.Devices.Radios.RadioState,Windows.System.Devices,ContentType=WindowsRuntime];\
         $radios = Await ([Windows.Devices.Radios.Radio]::GetRadiosAsync()) ([System.Collections.Generic.IReadOnlyList[Windows.Devices.Radios.Radio]]);\
         $r = $radios | Where-Object {{ $_.Kind -eq [Windows.Devices.Radios.RadioKind]::Bluetooth }} | Select-Object -First 1;\
         if (-not $r) {{ throw 'no bluetooth radio found' }};\
         $res = Await ($r.SetStateAsync([Windows.Devices.Radios.RadioState]::{state})) ([Windows.Devices.Radios.RadioAccessStatus]);\
         if ($res -ne [Windows.Devices.Radios.RadioAccessStatus]::Allowed) {{ throw \"radio change not permitted: $res\" }}; 'ok'"
    );
    ps(&script).map_err(|e| AppError::msg(format!("could not turn Bluetooth {state}: {e}")))?;
    Ok(ExecutionOutput {
        text: format!("Bluetooth turned {state}."),
        data: Some(json!({"on": on})),
    })
}
