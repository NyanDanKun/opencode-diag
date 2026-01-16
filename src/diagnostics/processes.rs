//! Process monitoring (OpenCode, terminals, etc.)

use crate::diagnostics::{CheckResult, CheckStatus};
use sysinfo::System;

/// Check if OpenCode process is running
pub fn check_opencode_process() -> CheckResult {
    let sys = System::new_all();
    
    // Look for opencode process
    let opencode_processes: Vec<_> = sys.processes()
        .values()
        .filter(|p| {
            let name = p.name().to_string_lossy().to_lowercase();
            name.contains("opencode")
        })
        .collect();

    if opencode_processes.is_empty() {
        CheckResult::new("OPENCODE", CheckStatus::Inactive, "Process not detected")
    } else {
        let total_mem: u64 = opencode_processes.iter()
            .map(|p| p.memory())
            .sum();
        let mem_mb = total_mem / (1024 * 1024);
        
        let proc = &opencode_processes[0];
        let pid = proc.pid();
        
        let count_str = if opencode_processes.len() > 1 {
            format!(" ({} instances)", opencode_processes.len())
        } else {
            String::new()
        };
        
        let details = format!("PID {} :: {}MB{}", pid, mem_mb, count_str);
        
        // Warn if using too much memory
        let status = if mem_mb > 2000 {
            CheckStatus::Warning
        } else {
            CheckStatus::Ok
        };
        
        CheckResult::new("OPENCODE", status, &details)
    }
}

/// Check terminal processes (cmd, powershell, Windows Terminal)
pub fn check_terminals() -> CheckResult {
    let sys = System::new_all();
    
    let mut cmd_count = 0;
    let mut powershell_count = 0;
    let mut wt_count = 0;
    let mut total_mem: u64 = 0;
    
    for process in sys.processes().values() {
        let name = process.name().to_string_lossy().to_lowercase();
        
        if name == "cmd.exe" {
            cmd_count += 1;
            total_mem += process.memory();
        } else if name.contains("powershell") {
            powershell_count += 1;
            total_mem += process.memory();
        } else if name == "windowsterminal.exe" || name == "wt.exe" {
            wt_count += 1;
            total_mem += process.memory();
        }
    }
    
    let total_count = cmd_count + powershell_count + wt_count;
    let mem_mb = total_mem / (1024 * 1024);
    
    if total_count == 0 {
        return CheckResult::new("TERMINALS", CheckStatus::Inactive, "No terminals detected");
    }
    
    let mut parts = Vec::new();
    if cmd_count > 0 {
        parts.push(format!("cmd:{}", cmd_count));
    }
    if powershell_count > 0 {
        parts.push(format!("ps:{}", powershell_count));
    }
    if wt_count > 0 {
        parts.push(format!("wt:{}", wt_count));
    }
    
    let details = format!("{} :: {}MB", parts.join(" "), mem_mb);
    
    // Warn if many terminals are open (might indicate many agents)
    let status = if total_count > 10 {
        CheckStatus::Warning
    } else {
        CheckStatus::Ok
    };
    
    CheckResult::new("TERMINALS", status, &details)
}

/// Get top processes by memory usage
#[allow(dead_code)]
pub fn get_top_processes(limit: usize) -> Vec<(String, u64)> {
    let sys = System::new_all();
    
    let mut processes: Vec<_> = sys.processes()
        .values()
        .map(|p| {
            (p.name().to_string_lossy().to_string(), p.memory())
        })
        .collect();
    
    processes.sort_by(|a, b| b.1.cmp(&a.1));
    processes.truncate(limit);
    
    processes
}

/// Get processes by name pattern
#[allow(dead_code)]
pub fn find_processes(pattern: &str) -> Vec<(String, u32, u64)> {
    let sys = System::new_all();
    let pattern_lower = pattern.to_lowercase();
    
    sys.processes()
        .values()
        .filter(|p| {
            p.name().to_string_lossy().to_lowercase().contains(&pattern_lower)
        })
        .map(|p| {
            (
                p.name().to_string_lossy().to_string(),
                p.pid().as_u32(),
                p.memory() / (1024 * 1024), // MB
            )
        })
        .collect()
}
