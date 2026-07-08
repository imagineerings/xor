pub mod server;

use std::{
    fs,
    path::PathBuf,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{Context as _, Result, anyhow, bail, ensure};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImageFormat {
    Png,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Rect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl Rect {
    fn validate(&self) -> Result<()> {
        ensure!(
            self.width > 0,
            "capture region width must be greater than zero"
        );
        ensure!(
            self.height > 0,
            "capture region height must be greater than zero"
        );
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ScreenCaptureResult {
    pub data: Vec<u8>,
    pub format: ImageFormat,
    pub region: Option<Rect>,
    pub display: Option<usize>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DisplayInfo {
    pub id: usize,
    pub name: String,
    pub primary: bool,
}

pub trait ScreenCapture: Send + Sync {
    fn capture_fullscreen(&self, display: Option<usize>) -> Result<ScreenCaptureResult>;
    fn capture_region(&self, region: Rect, display: Option<usize>) -> Result<ScreenCaptureResult>;
    fn displays(&self) -> Result<Vec<DisplayInfo>>;
    fn supported_formats(&self) -> Vec<ImageFormat> {
        vec![ImageFormat::Png]
    }
}

#[derive(Default)]
pub struct NativeScreenCapture;

impl ScreenCapture for NativeScreenCapture {
    fn capture_fullscreen(&self, display: Option<usize>) -> Result<ScreenCaptureResult> {
        capture_native(None, display)
    }

    fn capture_region(&self, region: Rect, display: Option<usize>) -> Result<ScreenCaptureResult> {
        region.validate()?;
        capture_native(Some(region), display)
    }

    fn displays(&self) -> Result<Vec<DisplayInfo>> {
        Ok(vec![DisplayInfo {
            id: 0,
            name: "Primary display".to_string(),
            primary: true,
        }])
    }
}

fn capture_native(region: Option<Rect>, display: Option<usize>) -> Result<ScreenCaptureResult> {
    let output_path = temp_capture_path();
    let command_result = if cfg!(target_os = "macos") {
        capture_macos(&output_path, region.as_ref())
    } else if cfg!(target_os = "linux") {
        capture_linux(&output_path, region.as_ref())
    } else if cfg!(target_os = "windows") {
        Err(anyhow!(
            "Windows screen capture is not wired yet; use a DXGI-backed Peekaboo build"
        ))
    } else {
        Err(anyhow!("screen capture is unsupported on this platform"))
    };

    if let Err(error) = command_result {
        if output_path.exists() {
            fs::remove_file(&output_path)
                .with_context(|| format!("removing {}", output_path.display()))?;
        }
        return Err(error);
    }

    let data = fs::read(&output_path)
        .with_context(|| format!("reading screen capture {}", output_path.display()))?;
    fs::remove_file(&output_path).with_context(|| format!("removing {}", output_path.display()))?;
    ensure!(
        data.starts_with(b"\x89PNG\r\n\x1a\n"),
        "screen capture backend did not produce a PNG image"
    );

    Ok(ScreenCaptureResult {
        data,
        format: ImageFormat::Png,
        region,
        display,
    })
}

fn capture_macos(output_path: &PathBuf, region: Option<&Rect>) -> Result<()> {
    let mut command = Command::new("screencapture");
    command.arg("-x");
    if let Some(region) = region {
        command.arg("-R").arg(format!(
            "{},{},{},{}",
            region.x, region.y, region.width, region.height
        ));
    }
    command.arg(output_path);
    run_capture_command(command, "macOS screencapture")
}

fn capture_linux(output_path: &PathBuf, region: Option<&Rect>) -> Result<()> {
    let mut grim = Command::new("grim");
    if let Some(region) = region {
        grim.arg("-g").arg(format!(
            "{},{} {}x{}",
            region.x, region.y, region.width, region.height
        ));
    }
    grim.arg(output_path);
    match run_capture_command(grim, "grim") {
        Ok(()) => Ok(()),
        Err(grim_error) if region.is_none() => {
            let mut gnome = Command::new("gnome-screenshot");
            gnome.arg("-f").arg(output_path);
            run_capture_command(gnome, "gnome-screenshot").map_err(|gnome_error| {
                anyhow!(
                    "Linux screen capture failed with grim ({grim_error}) and gnome-screenshot ({gnome_error})"
                )
            })
        }
        Err(error) => Err(error),
    }
}

fn run_capture_command(mut command: Command, label: &str) -> Result<()> {
    let output = command.output().with_context(|| {
        format!("starting {label}; install it or grant screen recording access")
    })?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("{label} failed: {stderr}");
    }
    Ok(())
}

fn temp_capture_path() -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    std::env::temp_dir().join(format!("sim-peekaboo-{}-{nanos}.png", std::process::id()))
}

#[cfg(test)]
pub struct MockScreenCapture {
    pub image: Vec<u8>,
}

#[cfg(test)]
impl Default for MockScreenCapture {
    fn default() -> Self {
        Self {
            image: b"\x89PNG\r\n\x1a\nmock".to_vec(),
        }
    }
}

#[cfg(test)]
impl ScreenCapture for MockScreenCapture {
    fn capture_fullscreen(&self, display: Option<usize>) -> Result<ScreenCaptureResult> {
        Ok(ScreenCaptureResult {
            data: self.image.clone(),
            format: ImageFormat::Png,
            region: None,
            display,
        })
    }

    fn capture_region(&self, region: Rect, display: Option<usize>) -> Result<ScreenCaptureResult> {
        region.validate()?;
        Ok(ScreenCaptureResult {
            data: self.image.clone(),
            format: ImageFormat::Png,
            region: Some(region),
            display,
        })
    }

    fn displays(&self) -> Result<Vec<DisplayInfo>> {
        Ok(vec![DisplayInfo {
            id: 0,
            name: "Mock display".to_string(),
            primary: true,
        }])
    }
}
