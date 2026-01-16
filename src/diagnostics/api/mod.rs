//! API health checks for various AI services

use crate::diagnostics::{CheckResult, CheckStatus};
use std::time::{Duration, Instant};

/// Extract error message from JSON response
fn extract_error_message(body: &str) -> Option<String> {
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(body) {
        // Try common error formats
        if let Some(error) = json.get("error") {
            if let Some(msg) = error.get("message") {
                return msg.as_str().map(|s| s.to_string());
            }
            // OpenAI format
            if error.is_string() {
                return error.as_str().map(|s| s.to_string());
            }
        }
        // Alternative format
        if let Some(msg) = json.get("message") {
            return msg.as_str().map(|s| s.to_string());
        }
    }
    None
}

/// Check Claude/Anthropic API status
pub fn check_claude_api() -> CheckResult {
    let client = match reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
    {
        Ok(c) => c,
        Err(_) => {
            return CheckResult::new("CLAUDE API", CheckStatus::Error, "Failed to create HTTP client");
        }
    };

    let start = Instant::now();
    
    // Use HEAD request to check if API is reachable without triggering 405
    // Or use the root domain which typically returns a valid response
    let result = client.head("https://api.anthropic.com")
        .send();

    let elapsed = start.elapsed().as_millis();

    match result {
        Ok(response) => {
            let status_code = response.status().as_u16();
            
            let (status, details) = match status_code {
                // HEAD to root may return various codes
                200..=399 => (CheckStatus::Ok, format!("api.anthropic.com :: reachable :: {}ms", elapsed)),
                401 | 403 => (CheckStatus::Ok, format!("api.anthropic.com :: reachable :: {}ms (auth required)", elapsed)),
                429 => (CheckStatus::Warning, format!("api.anthropic.com :: {} :: rate limited", status_code)),
                503 => (CheckStatus::Error, format!("api.anthropic.com :: {} :: server at capacity", status_code)),
                529 => (CheckStatus::Error, format!("api.anthropic.com :: {} :: overloaded", status_code)),
                500..=599 => (CheckStatus::Error, format!("api.anthropic.com :: {} :: server error", status_code)),
                _ => {
                    // For any other status, if we got a response, API is reachable
                    if elapsed < 3000 {
                        (CheckStatus::Ok, format!("api.anthropic.com :: reachable :: {}ms", elapsed))
                    } else {
                        (CheckStatus::Warning, format!("api.anthropic.com :: slow :: {}ms", elapsed))
                    }
                }
            };

            CheckResult::new("CLAUDE API", status, &details)
        }
        Err(e) => {
            let details = if e.is_timeout() {
                "api.anthropic.com :: timeout".to_string()
            } else if e.is_connect() {
                "api.anthropic.com :: connection failed".to_string()
            } else {
                format!("api.anthropic.com :: {}", e)
            };
            
            CheckResult::new("CLAUDE API", CheckStatus::Error, &details)
        }
    }
}

/// Check OpenAI API status
pub fn check_openai_api() -> CheckResult {
    let client = match reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
    {
        Ok(c) => c,
        Err(_) => {
            return CheckResult::new("OPENAI API", CheckStatus::Error, "Failed to create HTTP client");
        }
    };

    let start = Instant::now();
    
    // Check OpenAI API - models endpoint with no auth returns 401 but proves reachability
    let result = client.get("https://api.openai.com/v1/models")
        .send();

    let elapsed = start.elapsed().as_millis();

    match result {
        Ok(response) => {
            let status_code = response.status().as_u16();
            let body = response.text().unwrap_or_default();
            let error_msg = extract_error_message(&body);
            
            let (status, details) = match status_code {
                200..=299 => (CheckStatus::Ok, format!("api.openai.com :: {} :: {}ms", status_code, elapsed)),
                401 => (CheckStatus::Ok, format!("api.openai.com :: reachable :: {}ms (auth required)", elapsed)),
                429 => (CheckStatus::Warning, format!("api.openai.com :: {} :: rate limited", status_code)),
                500..=599 => (CheckStatus::Error, format!("api.openai.com :: {} :: server error", status_code)),
                _ => (CheckStatus::Warning, format!("api.openai.com :: {} :: {}ms", status_code, elapsed)),
            };

            let mut check = CheckResult::new("OPENAI API", status, &details);
            if let Some(msg) = error_msg {
                check = check.with_message(&msg);
            }
            check
        }
        Err(e) => {
            let details = if e.is_timeout() {
                "api.openai.com :: timeout".to_string()
            } else if e.is_connect() {
                "api.openai.com :: connection failed".to_string()
            } else {
                format!("api.openai.com :: {}", e)
            };
            
            CheckResult::new("OPENAI API", CheckStatus::Error, &details)
        }
    }
}

/// Check Google AI (Gemini) API status
pub fn check_google_api() -> CheckResult {
    let client = match reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
    {
        Ok(c) => c,
        Err(_) => {
            return CheckResult::new("GOOGLE AI", CheckStatus::Error, "Failed to create HTTP client");
        }
    };

    let start = Instant::now();
    
    // Check Google AI API endpoint
    let result = client.get("https://generativelanguage.googleapis.com/v1beta/models")
        .send();

    let elapsed = start.elapsed().as_millis();

    match result {
        Ok(response) => {
            let status_code = response.status().as_u16();
            
            let (status, details) = match status_code {
                200..=299 => (CheckStatus::Ok, format!("googleapis.com :: {} :: {}ms", status_code, elapsed)),
                400 | 401 | 403 => (CheckStatus::Ok, format!("googleapis.com :: reachable :: {}ms (auth required)", elapsed)),
                429 => (CheckStatus::Warning, format!("googleapis.com :: {} :: rate limited", status_code)),
                500..=599 => (CheckStatus::Error, format!("googleapis.com :: {} :: server error", status_code)),
                _ => (CheckStatus::Warning, format!("googleapis.com :: {} :: {}ms", status_code, elapsed)),
            };

            CheckResult::new("GOOGLE AI", status, &details)
        }
        Err(e) => {
            let details = if e.is_timeout() {
                "googleapis.com :: timeout".to_string()
            } else if e.is_connect() {
                "googleapis.com :: connection failed".to_string()
            } else {
                format!("googleapis.com :: {}", e)
            };
            
            CheckResult::new("GOOGLE AI", CheckStatus::Error, &details)
        }
    }
}
