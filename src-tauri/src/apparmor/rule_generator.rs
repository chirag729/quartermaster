use std::collections::HashMap;
use super::types::{DenialEvent, PermissionSuggestion, RiskLevel};

pub fn generate_suggestions(denials: &[DenialEvent]) -> Vec<PermissionSuggestion> {
    let mut groups: HashMap<(String, String, String), Vec<&DenialEvent>> = HashMap::new();

    for denial in denials {
        let key = (
            denial.profile.clone(),
            denial.name.clone(),
            denial.operation.clone(),
        );
        groups.entry(key).or_default().push(denial);
    }

    let mut suggestions = Vec::new();
    let mut id_counter = 0u32;

    for ((profile, name, operation), group_denials) in &groups {
        id_counter += 1;
        let denial_ids: Vec<String> = group_denials.iter().map(|d| d.id.clone()).collect();
        let denied_masks: Vec<&str> = group_denials.iter().map(|d| d.denied_mask.as_str()).collect();

        let (rule_text, description, risk_level, explanation) =
            generate_rule(name, operation, &denied_masks);

        suggestions.push(PermissionSuggestion {
            id: format!("suggestion-{}", id_counter),
            denial_ids,
            profile: profile.clone(),
            description,
            rule_text,
            risk_level,
            explanation,
        });
    }

    suggestions
}

fn generate_rule(
    name: &str,
    operation: &str,
    denied_masks: &[&str],
) -> (String, String, RiskLevel, String) {
    let mask = consolidate_masks(denied_masks);

    match operation {
        "open" | "mknod" | "mkdir" | "rename_dest" | "truncate" | "unlink" | "rmdir" => {
            let risk = assess_path_risk(name);
            let rule = format!("  {} {},", name, mask);
            let desc = format!("Allow {} access to {}", mask, name);
            let explanation = format!(
                "The profile tried to {} '{}' with '{}' permissions. This rule grants the requested access.",
                operation, name, mask
            );
            (rule, desc, risk, explanation)
        }
        "exec" => {
            let risk = if name.contains("/bin/") || name.contains("/usr/") {
                RiskLevel::Medium
            } else {
                RiskLevel::High
            };
            let rule = format!("  {} ix,", name);
            let desc = format!("Allow execution of {}", name);
            let explanation = format!(
                "The profile tried to execute '{}'. The 'ix' flag inherits the current profile.",
                name
            );
            (rule, desc, risk, explanation)
        }
        "connect" | "sendmsg" | "recvmsg" | "create" => {
            let rule = "  network inet stream,\n  network inet dgram,".to_string();
            let desc = "Allow network access (inet)".to_string();
            let explanation = format!(
                "The profile performed a '{}' network operation. This rule allows TCP and UDP networking.",
                operation
            );
            (rule, desc, RiskLevel::Medium, explanation)
        }
        "ptrace" => {
            let rule = format!("  ptrace (read) peer={},", name);
            let desc = format!("Allow ptrace read on {}", name);
            let explanation = "Ptrace allows process inspection. Read-only ptrace is generally safe for debugging tools.".to_string();
            (rule, desc, RiskLevel::High, explanation)
        }
        "signal" => {
            let rule = format!("  signal (send) set=(term, kill) peer={},", name);
            let desc = format!("Allow sending signals to {}", name);
            let explanation = "Allows the process to send termination signals to the specified peer.".to_string();
            (rule, desc, RiskLevel::Medium, explanation)
        }
        _ => {
            let rule = format!("  # TODO: manual rule for {} on {}", operation, name);
            let desc = format!("Manual rule needed: {} on {}", operation, name);
            let explanation = format!(
                "Unrecognized operation '{}' on '{}'. Manual review recommended.",
                operation, name
            );
            (rule, desc, RiskLevel::High, explanation)
        }
    }
}

fn consolidate_masks(masks: &[&str]) -> String {
    let mut chars = std::collections::BTreeSet::new();
    for mask in masks {
        for c in mask.chars() {
            chars.insert(c);
        }
    }
    if chars.is_empty() { "r".to_string() } else { chars.into_iter().collect() }
}

fn assess_path_risk(path: &str) -> RiskLevel {
    if path.starts_with("/etc/shadow") || path.starts_with("/etc/passwd")
        || path.starts_with("/etc/sudoers") || path.starts_with("/proc/") || path.starts_with("/sys/")
    {
        RiskLevel::Critical
    } else if path.starts_with("/etc/") || path.starts_with("/var/log/") {
        RiskLevel::High
    } else if path.starts_with("/tmp/") || path.starts_with("/run/") {
        RiskLevel::Low
    } else if path.starts_with("/home/") || path.starts_with("/usr/") {
        RiskLevel::Medium
    } else {
        RiskLevel::Medium
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_denial(profile: &str, name: &str, operation: &str, denied_mask: &str) -> DenialEvent {
        DenialEvent {
            id: format!("test-{}-{}", profile, name),
            timestamp: "2024-01-01T00:00:00Z".to_string(),
            profile: profile.to_string(),
            operation: operation.to_string(),
            name: name.to_string(),
            requested_mask: denied_mask.to_string(),
            denied_mask: denied_mask.to_string(),
            pid: 1,
            comm: "test".to_string(),
            raw_log: String::new(),
        }
    }

    #[test]
    fn generates_file_open_suggestion() {
        let denials = vec![make_denial("firefox", "/etc/hosts", "open", "r")];
        let suggestions = generate_suggestions(&denials);
        assert_eq!(suggestions.len(), 1);
        assert!(suggestions[0].rule_text.contains("/etc/hosts"));
        assert!(suggestions[0].rule_text.contains("r"));
    }

    #[test]
    fn generates_exec_suggestion() {
        let denials = vec![make_denial("snap.code", "/usr/bin/git", "exec", "x")];
        let suggestions = generate_suggestions(&denials);
        assert_eq!(suggestions.len(), 1);
        assert!(suggestions[0].rule_text.contains("ix"));
        assert!(suggestions[0].rule_text.contains("/usr/bin/git"));
    }

    #[test]
    fn generates_network_suggestion() {
        let denials = vec![make_denial("app", "0.0.0.0", "connect", "")];
        let suggestions = generate_suggestions(&denials);
        assert_eq!(suggestions.len(), 1);
        assert!(suggestions[0].rule_text.contains("network inet"));
    }

    #[test]
    fn groups_same_profile_name_operation() {
        let denials = vec![
            make_denial("firefox", "/etc/hosts", "open", "r"),
            make_denial("firefox", "/etc/hosts", "open", "w"),
        ];
        let suggestions = generate_suggestions(&denials);
        assert_eq!(suggestions.len(), 1);
        // Consolidated mask should contain both r and w
        assert!(suggestions[0].rule_text.contains("r"));
        assert!(suggestions[0].rule_text.contains("w"));
    }

    #[test]
    fn different_operations_produce_separate_suggestions() {
        let denials = vec![
            make_denial("app", "/tmp/file", "open", "r"),
            make_denial("app", "/usr/bin/ls", "exec", "x"),
        ];
        let suggestions = generate_suggestions(&denials);
        assert_eq!(suggestions.len(), 2);
    }

    #[test]
    fn risk_assessment_critical_paths() {
        assert!(matches!(assess_path_risk("/etc/shadow"), RiskLevel::Critical));
        assert!(matches!(assess_path_risk("/etc/passwd"), RiskLevel::Critical));
        assert!(matches!(assess_path_risk("/proc/1/status"), RiskLevel::Critical));
        assert!(matches!(assess_path_risk("/sys/class/net"), RiskLevel::Critical));
    }

    #[test]
    fn risk_assessment_low_paths() {
        assert!(matches!(assess_path_risk("/tmp/foo"), RiskLevel::Low));
        assert!(matches!(assess_path_risk("/run/user/1000/bus"), RiskLevel::Low));
    }

    #[test]
    fn risk_assessment_high_paths() {
        assert!(matches!(assess_path_risk("/etc/hostname"), RiskLevel::High));
        assert!(matches!(assess_path_risk("/var/log/syslog"), RiskLevel::High));
    }

    #[test]
    fn consolidate_masks_merges() {
        assert_eq!(consolidate_masks(&["r", "w"]), "rw");
        assert_eq!(consolidate_masks(&["rw", "r"]), "rw");
        assert_eq!(consolidate_masks(&[]), "r");
    }

    #[test]
    fn ptrace_suggestion() {
        let denials = vec![make_denial("debugger", "target_app", "ptrace", "")];
        let suggestions = generate_suggestions(&denials);
        assert_eq!(suggestions.len(), 1);
        assert!(suggestions[0].rule_text.contains("ptrace"));
    }

    #[test]
    fn signal_suggestion() {
        let denials = vec![make_denial("app", "child_app", "signal", "")];
        let suggestions = generate_suggestions(&denials);
        assert_eq!(suggestions.len(), 1);
        assert!(suggestions[0].rule_text.contains("signal"));
    }
}
