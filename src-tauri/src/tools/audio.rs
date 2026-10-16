use crate::error::{AppError, Result};
use crate::tools::ExecutionOutput;
use serde_json::json;
use std::process::Command;

// ---------------- volume (Windows Core Audio) ----------------

#[cfg(windows)]
fn with_endpoint<T>(
    f: impl FnOnce(&windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolume)
    -> windows::core::Result<T>,
) -> Result<T> {
    use windows::Win32::Media::Audio::{
        eConsole, eRender, IMMDeviceEnumerator, MMDeviceEnumerator,
    };
    use windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolume;
    use windows::Win32::System::Com::{CoCreateInstance, CoInitializeEx, CLSCTX_ALL, COINIT_MULTITHREADED};
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        let enumerator: IMMDeviceEnumerator =
            CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)
                .map_err(|e| AppError::msg(format!("audio init: {e}")))?;
        let device = enumerator
            .GetDefaultAudioEndpoint(eRender, eConsole)
            .map_err(|e| AppError::msg(format!("no output device: {e}")))?;
        let volume: IAudioEndpointVolume = device
            .Activate::<IAudioEndpointVolume>(CLSCTX_ALL, None)
            .map_err(|e| AppError::msg(format!("audio endpoint: {e}")))?;
        f(&volume).map_err(|e| AppError::msg(format!("audio: {e}")))
    }
}

#[cfg(windows)]
pub fn get() -> Result<ExecutionOutput> {
    let (level, muted) = with_endpoint(|v| unsafe {
        let scalar = v.GetMasterVolumeLevelScalar()?;
        let muted = v.GetMute()?.as_bool();
        Ok((scalar, muted))
    })?;
    let pct = (level * 100.0).round() as u8;
    Ok(ExecutionOutput {
        text: format!("Volume {pct}%{}", if muted { " (muted)" } else { "" }),
        data: Some(json!({"level": pct, "muted": muted})),
    })
}

#[cfg(not(windows))]
pub fn get() -> Result<ExecutionOutput> {
    let out = Command::new("sh")
        .args(["-c", "command -v pactl >/dev/null && pactl get-sink-volume @DEFAULT_SINK@ | grep -o '[0-9]\\+%' | head -1"])
        .output()
        .map_err(|e| AppError::msg(format!("audio: {e}")))?;
    let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
    Ok(ExecutionOutput {
        text: if text.is_empty() { "Volume unavailable (install pactl).".into() } else { format!("Volume {text}") },
        data: Some(json!({"raw": text})),
    })
}

#[cfg(windows)]
pub fn set_level(level: u8) -> Result<ExecutionOutput> {
    let scalar = (level.min(100) as f32) / 100.0;
    with_endpoint(|v| unsafe { v.SetMasterVolumeLevelScalar(scalar, std::ptr::null()) })?;
    Ok(ExecutionOutput {
        text: format!("Volume set to {level}%."),
        data: Some(json!({"level": level.min(100)})),
    })
}

#[cfg(not(windows))]
pub fn set_level(level: u8) -> Result<ExecutionOutput> {
    let pct = level.min(100);
    Command::new("sh")
        .args(["-c", &format!("pactl set-sink-volume @DEFAULT_SINK@ {pct}%")])
        .output()
        .map_err(|e| AppError::msg(format!("audio: {e}")))?;
    Ok(ExecutionOutput { text: format!("Volume set to {pct}%."), data: None })
}

#[cfg(windows)]
pub fn set_mute(mute: bool) -> Result<ExecutionOutput> {
    with_endpoint(|v| unsafe { v.SetMute(mute, std::ptr::null()) })?;
    Ok(ExecutionOutput {
        text: if mute { "Audio muted.".into() } else { "Audio unmuted.".into() },
        data: Some(json!({"muted": mute})),
    })
}

#[cfg(not(windows))]
pub fn set_mute(mute: bool) -> Result<ExecutionOutput> {
    let arg = if mute { "1" } else { "0" };
    Command::new("sh")
        .args(["-c", &format!("pactl set-sink-mute @DEFAULT_SINK@ {arg}")])
        .output()
        .map_err(|e| AppError::msg(format!("audio: {e}")))?;
    Ok(ExecutionOutput { text: format!("Mute set to {mute}."), data: None })
}

// ---------------- brightness (WMI, laptops) ----------------

fn ps(script: &str) -> Result<String> {
    let out = Command::new("powershell")
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

pub fn brightness_get() -> Result<ExecutionOutput> {
    let v = ps("(Get-CimInstance -Namespace root/wmi -ClassName WmiMonitorBrightness -ErrorAction Stop | Select-Object -First 1).CurrentBrightness")
        .map_err(|_| AppError::Precondition("brightness control is unavailable on this display (common for desktops)".into()))?;
    let level: u32 = v.trim().parse().map_err(|_| AppError::msg("unexpected brightness output"))?;
    Ok(ExecutionOutput {
        text: format!("Brightness {level}%"),
        data: Some(json!({"level": level})),
    })
}

pub fn brightness_set(level: u8) -> Result<ExecutionOutput> {
    let l = level.min(100);
    ps(&format!(
        "$m = Get-CimInstance -Namespace root/wmi -ClassName WmiMonitorBrightnessMethods -ErrorAction Stop; \
         $m | ForEach-Object {{ $_.WmiSetBrightness(0, {l}) }}"
    ))
    .map_err(|_| AppError::Precondition("brightness control is unavailable on this display".into()))?;
    Ok(ExecutionOutput {
        text: format!("Brightness set to {l}%."),
        data: Some(json!({"level": l})),
    })
}

// ---------------- wallpaper ----------------

#[cfg(windows)]
pub fn wallpaper_get() -> Result<ExecutionOutput> {
    use windows::Win32::UI::WindowsAndMessaging::{SystemParametersInfoW, SPI_GETDESKWALLPAPER};
    let mut buf = [0u16; 520];
    unsafe {
        SystemParametersInfoW(
            SPI_GETDESKWALLPAPER,
            buf.len() as u32,
            Some(buf.as_mut_ptr() as *mut _),
            Default::default(),
        )
        .map_err(|e| AppError::msg(format!("wallpaper: {e}")))?;
    }
    let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    let path = String::from_utf16_lossy(&buf[..len]);
    Ok(ExecutionOutput {
        text: if path.is_empty() { "No wallpaper path reported.".into() } else { format!("Wallpaper: {path}") },
        data: Some(json!({"path": path})),
    })
}

#[cfg(not(windows))]
pub fn wallpaper_get() -> Result<ExecutionOutput> {
    Ok(ExecutionOutput { text: "Wallpaper control is Windows-only in this build.".into(), data: None })
}

pub fn wallpaper_set(path: &str) -> Result<ExecutionOutput> {
    let p = crate::tools::resolve_path(path)?;
    if !p.exists() {
        return Err(AppError::NotFound(format!("wallpaper image {path}")));
    }
    let ext = p.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
    if !["png", "jpg", "jpeg", "bmp", "webp"].contains(&ext.as_str()) {
        return Err(AppError::Precondition("wallpaper must be a png/jpg/bmp/webp image".into()));
    }
    let data_dir = crate::tools::home_dir().join(".astra");
    let _ = data_dir;
    #[cfg(windows)]
    {
        use windows::Win32::UI::WindowsAndMessaging::{
            SystemParametersInfoW, SPI_SETDESKWALLPAPER, SPIF_SENDWININICHANGE, SPIF_UPDATEINIFILE,
        };
        let wide: Vec<u16> = p
            .to_string_lossy()
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();
        unsafe {
            SystemParametersInfoW(
                SPI_SETDESKWALLPAPER,
                0,
                Some(wide.as_ptr() as *mut _),
                SPIF_UPDATEINIFILE | SPIF_SENDWININICHANGE,
            )
            .map_err(|e| AppError::msg(format!("wallpaper: {e}")))?;
        }
    }
    #[cfg(not(windows))]
    {
        return Err(AppError::Precondition("wallpaper control is Windows-only in this build".into()));
    }
    Ok(ExecutionOutput {
        text: format!("Wallpaper set to {path}."),
        data: Some(json!({"path": path})),
    })
}
