//! GPU monitoring for Windows using WMI
//!
//! Supports Intel iGPU, NVIDIA, and AMD GPUs

use crate::diagnostics::{CheckResult, CheckStatus};

#[cfg(target_os = "windows")]
use wmi::{COMLibrary, WMIConnection};

#[cfg(target_os = "windows")]
use serde::Deserialize;

#[cfg(target_os = "windows")]
#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
struct Win32VideoController {
    name: Option<String>,
    adapter_ram: Option<u64>,
    #[serde(rename = "CurrentHorizontalResolution")]
    current_horizontal_resolution: Option<u32>,
}

/// GPU usage info
#[derive(Debug, Clone)]
pub struct GpuInfo {
    pub name: String,
    pub usage_percent: Option<f32>,
    pub memory_mb: Option<u64>,
}

/// Check GPU status
#[cfg(target_os = "windows")]
pub fn check_gpu() -> CheckResult {
    // Try to get GPU info via WMI
    match get_gpu_info_wmi() {
        Ok(gpus) => {
            if gpus.is_empty() {
                return CheckResult::new("GPU", CheckStatus::Inactive, "No GPU detected");
            }

            // Format GPU info
            let gpu_names: Vec<String> = gpus.iter()
                .map(|g| {
                    // Shorten common GPU names
                    let name = shorten_gpu_name(&g.name);
                    if let Some(usage) = g.usage_percent {
                        format!("{}: {}%", name, usage as u32)
                    } else {
                        name
                    }
                })
                .collect();

            let details = gpu_names.join(" :: ");
            
            // Determine status based on usage
            let max_usage = gpus.iter()
                .filter_map(|g| g.usage_percent)
                .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
                .unwrap_or(0.0);

            let status = if max_usage > 95.0 {
                CheckStatus::Error
            } else if max_usage > 80.0 {
                CheckStatus::Warning
            } else {
                CheckStatus::Ok
            };

            CheckResult::new("GPU", status, &details)
        }
        Err(e) => {
            // Fallback: just list GPUs without usage
            CheckResult::new("GPU", CheckStatus::Warning, &format!("Could not get GPU usage: {}", e))
        }
    }
}

#[cfg(not(target_os = "windows"))]
pub fn check_gpu() -> CheckResult {
    CheckResult::new("GPU", CheckStatus::Inactive, "GPU monitoring only available on Windows")
}

/// Shorten common GPU names for display
fn shorten_gpu_name(name: &str) -> String {
    let name = name.trim();
    
    // Intel
    if name.contains("Intel") {
        if name.contains("UHD") {
            if let Some(model) = extract_number_after(name, "UHD") {
                return format!("Intel UHD {}", model);
            }
            return "Intel UHD".to_string();
        }
        if name.contains("Iris") {
            return "Intel Iris".to_string();
        }
        return "Intel GPU".to_string();
    }
    
    // NVIDIA
    if name.contains("NVIDIA") || name.contains("GeForce") {
        if name.contains("RTX") {
            if let Some(model) = extract_rtx_model(name) {
                return format!("RTX {}", model);
            }
        }
        if name.contains("GTX") {
            if let Some(model) = extract_gtx_model(name) {
                return format!("GTX {}", model);
            }
        }
        return name.replace("NVIDIA ", "").replace("GeForce ", "");
    }
    
    // AMD
    if name.contains("AMD") || name.contains("Radeon") {
        if name.contains("RX") {
            if let Some(model) = extract_rx_model(name) {
                return format!("RX {}", model);
            }
        }
        return name.replace("AMD ", "").replace("Radeon ", "Radeon ");
    }
    
    // Return as-is if unknown
    if name.len() > 20 {
        name[..20].to_string() + "..."
    } else {
        name.to_string()
    }
}

fn extract_number_after(s: &str, prefix: &str) -> Option<String> {
    if let Some(idx) = s.find(prefix) {
        let after = &s[idx + prefix.len()..];
        let num: String = after.chars()
            .skip_while(|c| !c.is_ascii_digit())
            .take_while(|c| c.is_ascii_digit())
            .collect();
        if !num.is_empty() {
            return Some(num);
        }
    }
    None
}

fn extract_rtx_model(s: &str) -> Option<String> {
    if let Some(idx) = s.find("RTX") {
        let after = &s[idx + 3..];
        let model: String = after.chars()
            .skip_while(|c| c.is_whitespace())
            .take_while(|c| c.is_ascii_alphanumeric() || *c == ' ')
            .collect();
        let model = model.trim().to_string();
        if !model.is_empty() {
            return Some(model);
        }
    }
    None
}

fn extract_gtx_model(s: &str) -> Option<String> {
    if let Some(idx) = s.find("GTX") {
        let after = &s[idx + 3..];
        let model: String = after.chars()
            .skip_while(|c| c.is_whitespace())
            .take_while(|c| c.is_ascii_alphanumeric() || *c == ' ')
            .collect();
        let model = model.trim().to_string();
        if !model.is_empty() {
            return Some(model);
        }
    }
    None
}

fn extract_rx_model(s: &str) -> Option<String> {
    if let Some(idx) = s.find("RX") {
        let after = &s[idx + 2..];
        let model: String = after.chars()
            .skip_while(|c| c.is_whitespace())
            .take_while(|c| c.is_ascii_alphanumeric() || *c == ' ')
            .collect();
        let model = model.trim().to_string();
        if !model.is_empty() {
            return Some(model);
        }
    }
    None
}

/// Get GPU info using WMI
#[cfg(target_os = "windows")]
fn get_gpu_info_wmi() -> Result<Vec<GpuInfo>, String> {
    let com_con = COMLibrary::new().map_err(|e| format!("COM init failed: {:?}", e))?;
    let wmi_con = WMIConnection::new(com_con).map_err(|e| format!("WMI connection failed: {:?}", e))?;

    // Query video controllers
    let results: Vec<Win32VideoController> = wmi_con
        .raw_query("SELECT Name, AdapterRAM FROM Win32_VideoController")
        .map_err(|e| format!("WMI query failed: {:?}", e))?;

    let gpus: Vec<GpuInfo> = results
        .into_iter()
        .filter_map(|vc| {
            let name = vc.name?;
            Some(GpuInfo {
                name,
                usage_percent: None, // WMI doesn't provide real-time usage easily
                memory_mb: vc.adapter_ram.map(|r| r / (1024 * 1024)),
            })
        })
        .collect();

    // Try to get GPU usage from performance counters
    // This is more complex and may require additional queries
    // For now, we'll just return the GPU list
    
    Ok(gpus)
}
