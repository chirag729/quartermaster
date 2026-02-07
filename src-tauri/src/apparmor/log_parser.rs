use regex::Regex;
use chrono::Utc;
use std::sync::LazyLock;

use super::types::DenialEvent;
use crate::error::AppError;

static DENIAL_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"apparmor="DENIED" operation="([^"]*)" (?:class="[^"]*" )?profile="([^"]*)" name="([^"]*)" pid=(\d+) comm="([^"]*)"(?: requested_mask="([^"]*)")?(?: denied_mask="([^"]*)")?"#
    ).unwrap()
});

static TIMESTAMP_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"msg=audit\((\d+\.\d+):\d+\)"#).unwrap()
});

pub fn parse_denial_line(line: &str) -> Option<DenialEvent> {
    let caps = DENIAL_RE.captures(line)?;

    let timestamp = if let Some(ts_caps) = TIMESTAMP_RE.captures(line) {
        let epoch: f64 = ts_caps[1].parse().unwrap_or(0.0);
        let dt = chrono::DateTime::from_timestamp(epoch as i64, ((epoch.fract()) * 1_000_000_000.0) as u32)
            .unwrap_or_else(|| Utc::now().into());
        dt.to_rfc3339()
    } else {
        Utc::now().to_rfc3339()
    };

    let id = format!("{}-{}", &timestamp, &caps[4]);

    Some(DenialEvent {
        id,
        timestamp,
        operation: caps[1].to_string(),
        profile: caps[2].to_string(),
        name: caps[3].to_string(),
        pid: caps[4].parse().unwrap_or(0),
        comm: caps[5].to_string(),
        requested_mask: caps.get(6).map(|m| m.as_str().to_string()).unwrap_or_default(),
        denied_mask: caps.get(7).map(|m| m.as_str().to_string()).unwrap_or_default(),
        raw_log: line.to_string(),
    })
}

pub async fn parse_audit_log() -> Result<Vec<DenialEvent>, AppError> {
    let mut denials = Vec::new();

    // Try audit.log first
    let audit_path = "/var/log/audit/audit.log";
    if std::path::Path::new(audit_path).exists() {
        // Read only the last 1MB of the file in a blocking thread
        let content = tokio::task::spawn_blocking(move || -> Option<String> {
            use std::io::{Read, Seek, SeekFrom};
            let mut file = std::fs::File::open(audit_path).ok()?;
            let len = file.metadata().ok()?.len();
            const MAX_BYTES: u64 = 1_048_576; // 1MB
            if len > MAX_BYTES {
                file.seek(SeekFrom::End(-(MAX_BYTES as i64))).ok()?;
                // Skip partial first line
                let mut buf_reader = std::io::BufReader::new(file);
                let mut skip = String::new();
                std::io::BufRead::read_line(&mut buf_reader, &mut skip).ok()?;
                let mut content = String::new();
                buf_reader.read_to_string(&mut content).ok()?;
                Some(content)
            } else {
                let mut content = String::new();
                file.read_to_string(&mut content).ok()?;
                Some(content)
            }
        }).await.unwrap_or(None);

        if let Some(content) = content {
            for line in content.lines() {
                if line.contains("apparmor=\"DENIED\"") {
                    if let Some(denial) = parse_denial_line(line) {
                        denials.push(denial);
                    }
                }
            }
            if denials.len() > 200 {
                denials = denials.split_off(denials.len() - 200);
            }
            return Ok(denials);
        }
    }

    // Fallback: journalctl
    let output = tokio::process::Command::new("journalctl")
        .args(["--no-pager", "-n", "500", "-g", "apparmor.*DENIED"])
        .output()
        .await?;

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            if line.contains("DENIED") {
                if let Some(denial) = parse_denial_line(line) {
                    denials.push(denial);
                }
            }
        }
    }

    Ok(denials)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_standard_denial_line() {
        let line = r#"type=AVC msg=audit(1700000000.123:456): apparmor="DENIED" operation="open" class="file" profile="/usr/bin/firefox" name="/etc/hosts" pid=1234 comm="firefox" requested_mask="r" denied_mask="r""#;
        let result = parse_denial_line(line);
        assert!(result.is_some());
        let event = result.unwrap();
        assert_eq!(event.operation, "open");
        assert_eq!(event.profile, "/usr/bin/firefox");
        assert_eq!(event.name, "/etc/hosts");
        assert_eq!(event.pid, 1234);
        assert_eq!(event.comm, "firefox");
        assert_eq!(event.requested_mask, "r");
        assert_eq!(event.denied_mask, "r");
    }

    #[test]
    fn parse_denial_without_class() {
        let line = r#"type=AVC msg=audit(1700000000.000:789): apparmor="DENIED" operation="exec" profile="snap.code.code" name="/usr/bin/git" pid=5678 comm="code" requested_mask="x" denied_mask="x""#;
        let result = parse_denial_line(line);
        assert!(result.is_some());
        let event = result.unwrap();
        assert_eq!(event.operation, "exec");
        assert_eq!(event.profile, "snap.code.code");
        assert_eq!(event.name, "/usr/bin/git");
    }

    #[test]
    fn parse_denial_with_timestamp() {
        let line = r#"type=AVC msg=audit(1700000000.500:100): apparmor="DENIED" operation="open" profile="test" name="/tmp/foo" pid=42 comm="cat" denied_mask="r""#;
        let result = parse_denial_line(line);
        assert!(result.is_some());
        let event = result.unwrap();
        assert!(event.timestamp.contains("2023-11-14"));
    }

    #[test]
    fn returns_none_for_non_denial() {
        let line = "some random log line with no apparmor denial";
        assert!(parse_denial_line(line).is_none());
    }

    #[test]
    fn returns_none_for_allowed() {
        let line = r#"apparmor="ALLOWED" operation="open" profile="test" name="/tmp/ok" pid=1 comm="test""#;
        assert!(parse_denial_line(line).is_none());
    }

    #[test]
    fn parse_denial_without_masks() {
        let line = r#"type=AVC msg=audit(1700000000.000:1): apparmor="DENIED" operation="connect" profile="app" name="0.0.0.0" pid=100 comm="curl""#;
        let result = parse_denial_line(line);
        assert!(result.is_some());
        let event = result.unwrap();
        assert_eq!(event.requested_mask, "");
        assert_eq!(event.denied_mask, "");
    }
}
