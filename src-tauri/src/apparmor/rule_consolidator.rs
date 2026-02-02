use std::collections::HashMap;

use super::types::{ConsolidatedRule, ConsolidationResult, PermissionSuggestion};

/// Minimum number of paths sharing a directory prefix to trigger glob consolidation.
const MIN_PATHS_FOR_GLOB: usize = 3;

/// Parse a file rule into (path, mask) if it matches the AppArmor file rule pattern.
/// Examples: "/foo/bar r," -> Some(("/foo/bar", "r"))
///           "/foo/** rw," -> Some(("/foo/**", "rw"))
fn parse_file_rule(rule: &str) -> Option<(&str, &str)> {
    let trimmed = rule.trim().trim_end_matches(',');
    // File rules start with /
    if !trimmed.starts_with('/') {
        return None;
    }
    // Split on last whitespace to get path and mask
    let last_space = trimmed.rfind(|c: char| c.is_whitespace())?;
    let path = trimmed[..last_space].trim();
    let mask = trimmed[last_space..].trim();
    // Validate mask is only permission characters
    if mask.is_empty() || !mask.chars().all(|c| "rwxalmpkicuo".contains(c)) {
        return None;
    }
    Some((path, mask))
}

/// Check if a rule is a non-file rule (network, ptrace, signal, capability, etc.)
fn is_non_file_rule(rule: &str) -> bool {
    let trimmed = rule.trim();
    trimmed.starts_with("network ")
        || trimmed.starts_with("ptrace ")
        || trimmed.starts_with("signal ")
        || trimmed.starts_with("capability ")
        || trimmed.starts_with("dbus ")
        || trimmed.starts_with("mount ")
        || trimmed.starts_with("umount ")
        || trimmed.starts_with("pivot_root ")
        || trimmed.starts_with("unix ")
}

/// Check if a path already contains glob characters.
fn is_glob_path(path: &str) -> bool {
    path.contains('*') || path.contains('?') || path.contains('[')
}

/// Extract the parent directory of a path.
fn parent_dir(path: &str) -> Option<&str> {
    let trimmed = path.trim_end_matches('/');
    trimmed.rfind('/').map(|idx| &trimmed[..idx])
}

/// Get all ancestor directories for a path (from deepest to shallowest).
/// E.g., "/a/b/c/d.txt" -> ["/a/b/c", "/a/b", "/a"]
fn ancestor_dirs(path: &str) -> Vec<String> {
    let mut dirs = Vec::new();
    let mut current = path.to_string();
    while let Some(parent) = parent_dir(&current) {
        if parent.is_empty() {
            break;
        }
        dirs.push(parent.to_string());
        current = parent.to_string();
    }
    dirs
}

/// Consolidate a set of (path, original_rule) entries with the same mask.
///
/// Two-pass approach:
/// 1. Group by immediate parent - consolidate groups with 3+ entries
/// 2. Collect leftovers (from groups < 3), find common ancestor directories,
///    and consolidate those with 3+ entries using `**` glob
fn consolidate_paths_for_mask(
    mask: &str,
    paths_and_rules: &[(String, String)],
) -> Vec<ConsolidatedRule> {
    let mut result = Vec::new();

    // Pass 1: Group by immediate parent directory
    let mut by_parent: HashMap<String, Vec<usize>> = HashMap::new(); // parent -> indices
    let mut no_parent: Vec<usize> = Vec::new();

    for (i, (path, _)) in paths_and_rules.iter().enumerate() {
        if let Some(parent) = parent_dir(path) {
            by_parent.entry(parent.to_string()).or_default().push(i);
        } else {
            no_parent.push(i);
        }
    }

    let mut leftover_indices: Vec<usize> = Vec::new();

    for (dir, indices) in &by_parent {
        if indices.len() >= MIN_PATHS_FOR_GLOB {
            let glob_rule = format!("  {}/*  {},", dir, mask);
            let originals: Vec<String> = indices
                .iter()
                .map(|&i| paths_and_rules[i].1.clone())
                .collect();
            result.push(ConsolidatedRule {
                rule_text: glob_rule,
                original_rules: originals,
                is_glob: true,
            });
        } else {
            leftover_indices.extend(indices);
        }
    }
    leftover_indices.extend(&no_parent);

    if leftover_indices.len() < MIN_PATHS_FOR_GLOB {
        // Not enough leftovers to consolidate, emit individually
        for &i in &leftover_indices {
            let rule = &paths_and_rules[i].1;
            result.push(ConsolidatedRule {
                rule_text: rule.clone(),
                original_rules: vec![rule.clone()],
                is_glob: false,
            });
        }
        return result;
    }

    // Pass 2: Find common ancestor directories among leftovers
    // Count how many leftover paths fall under each ancestor directory
    let mut ancestor_counts: HashMap<String, Vec<usize>> = HashMap::new();
    for &i in &leftover_indices {
        let path = &paths_and_rules[i].0;
        for ancestor in ancestor_dirs(path) {
            ancestor_counts.entry(ancestor).or_default().push(i);
        }
    }

    // Find the most specific (longest) ancestor that has 3+ paths
    let mut best_ancestors: Vec<(String, Vec<usize>)> = ancestor_counts
        .into_iter()
        .filter(|(_, indices)| indices.len() >= MIN_PATHS_FOR_GLOB)
        .collect();
    // Sort by path length descending (most specific first)
    best_ancestors.sort_by(|a, b| b.0.len().cmp(&a.0.len()));

    let mut consumed: Vec<bool> = vec![false; paths_and_rules.len()];

    for (ancestor, indices) in &best_ancestors {
        let unconsumed: Vec<usize> = indices.iter().copied().filter(|&i| !consumed[i]).collect();
        if unconsumed.len() >= MIN_PATHS_FOR_GLOB {
            let glob_rule = format!("  {}/**  {},", ancestor, mask);
            let originals: Vec<String> = unconsumed
                .iter()
                .map(|&i| paths_and_rules[i].1.clone())
                .collect();
            for &i in &unconsumed {
                consumed[i] = true;
            }
            result.push(ConsolidatedRule {
                rule_text: glob_rule,
                original_rules: originals,
                is_glob: true,
            });
        }
    }

    // Emit any remaining unconsumed leftovers as individual rules
    for &i in &leftover_indices {
        if !consumed[i] {
            let rule = &paths_and_rules[i].1;
            result.push(ConsolidatedRule {
                rule_text: rule.clone(),
                original_rules: vec![rule.clone()],
                is_glob: false,
            });
        }
    }

    result
}

/// Consolidate permission suggestions into glob rules where possible.
///
/// Groups rules by (profile, mask), finds common directory prefixes,
/// and replaces 3+ files in the same directory with glob patterns.
/// Uses `*` for same-directory consolidation and `**` for ancestor-directory
/// consolidation (when paths span subdirectories).
pub fn consolidate(suggestions: &[PermissionSuggestion]) -> Vec<ConsolidationResult> {
    // Group suggestions by profile
    let mut by_profile: HashMap<String, Vec<&PermissionSuggestion>> = HashMap::new();
    for s in suggestions {
        by_profile.entry(s.profile.clone()).or_default().push(s);
    }

    let mut results: Vec<ConsolidationResult> = Vec::new();

    for (profile, suggestions) in &by_profile {
        let mut consolidated_rules: Vec<ConsolidatedRule> = Vec::new();

        // Separate file rules from non-file rules, group file rules by mask
        let mut file_rules_by_mask: HashMap<String, Vec<(String, String)>> = HashMap::new();
        let mut non_file_rules: Vec<String> = Vec::new();
        let mut glob_rules: Vec<String> = Vec::new();

        for s in suggestions {
            let rule = s.rule_text.trim().to_string();
            if is_non_file_rule(&rule) {
                non_file_rules.push(rule);
            } else if let Some((path, mask)) = parse_file_rule(&rule) {
                if is_glob_path(path) {
                    glob_rules.push(rule);
                } else {
                    file_rules_by_mask
                        .entry(mask.to_string())
                        .or_default()
                        .push((path.to_string(), rule));
                }
            } else {
                non_file_rules.push(rule);
            }
        }

        // Pass through non-file rules
        for rule in non_file_rules {
            consolidated_rules.push(ConsolidatedRule {
                rule_text: rule.clone(),
                original_rules: vec![rule],
                is_glob: false,
            });
        }

        // Pass through already-globbed rules
        for rule in glob_rules {
            consolidated_rules.push(ConsolidatedRule {
                rule_text: rule.clone(),
                original_rules: vec![rule],
                is_glob: false,
            });
        }

        // Consolidate file rules by mask
        for (mask, paths_and_rules) in &file_rules_by_mask {
            consolidated_rules.extend(consolidate_paths_for_mask(mask, paths_and_rules));
        }

        // Sort: globs first, then individual rules alphabetically
        consolidated_rules.sort_by(|a, b| {
            b.is_glob.cmp(&a.is_glob).then_with(|| a.rule_text.cmp(&b.rule_text))
        });

        results.push(ConsolidationResult {
            profile: profile.clone(),
            rules: consolidated_rules,
        });
    }

    results.sort_by(|a, b| a.profile.cmp(&b.profile));
    results
}

/// Consolidate raw rule strings for a single profile.
///
/// This is used to consolidate existing rules already in a profile file,
/// not just new suggestions. Takes the profile name and its current rules,
/// returns a ConsolidationResult with consolidated rules.
pub fn consolidate_raw_rules(profile: &str, rules: &[String]) -> ConsolidationResult {
    let mut consolidated_rules: Vec<ConsolidatedRule> = Vec::new();

    let mut file_rules_by_mask: HashMap<String, Vec<(String, String)>> = HashMap::new();
    let mut non_file_rules: Vec<String> = Vec::new();
    let mut glob_rules: Vec<String> = Vec::new();

    for rule_str in rules {
        let rule = rule_str.trim().to_string();
        if rule.is_empty() {
            continue;
        }
        if is_non_file_rule(&rule) {
            non_file_rules.push(rule);
        } else if let Some((path, mask)) = parse_file_rule(&rule) {
            if is_glob_path(path) {
                glob_rules.push(rule);
            } else {
                file_rules_by_mask
                    .entry(mask.to_string())
                    .or_default()
                    .push((path.to_string(), rule));
            }
        } else {
            non_file_rules.push(rule);
        }
    }

    for rule in non_file_rules {
        consolidated_rules.push(ConsolidatedRule {
            rule_text: rule.clone(),
            original_rules: vec![rule],
            is_glob: false,
        });
    }

    for rule in glob_rules {
        consolidated_rules.push(ConsolidatedRule {
            rule_text: rule.clone(),
            original_rules: vec![rule],
            is_glob: false,
        });
    }

    for (mask, paths_and_rules) in &file_rules_by_mask {
        consolidated_rules.extend(consolidate_paths_for_mask(mask, paths_and_rules));
    }

    consolidated_rules.sort_by(|a, b| {
        b.is_glob.cmp(&a.is_glob).then_with(|| a.rule_text.cmp(&b.rule_text))
    });

    ConsolidationResult {
        profile: profile.to_string(),
        rules: consolidated_rules,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::apparmor::types::RiskLevel;

    fn make_suggestion(profile: &str, rule_text: &str) -> PermissionSuggestion {
        PermissionSuggestion {
            id: format!("test-{}", rule_text.trim()),
            denial_ids: vec![],
            profile: profile.to_string(),
            description: String::new(),
            rule_text: rule_text.to_string(),
            risk_level: RiskLevel::Low,
            explanation: String::new(),
        }
    }

    #[test]
    fn test_consolidate_same_dir_glob() {
        let suggestions = vec![
            make_suggestion("test_profile", "  /opt/intellij/plugins/foo.jar r,"),
            make_suggestion("test_profile", "  /opt/intellij/plugins/bar.jar r,"),
            make_suggestion("test_profile", "  /opt/intellij/plugins/baz.jar r,"),
        ];

        let results = consolidate(&suggestions);
        assert_eq!(results.len(), 1);
        let result = &results[0];
        assert_eq!(result.profile, "test_profile");
        assert_eq!(result.rules.len(), 1);

        let rule = &result.rules[0];
        assert!(rule.is_glob);
        assert!(rule.rule_text.contains("/opt/intellij/plugins/*"));
        assert!(!rule.rule_text.contains("**")); // same dir, no subdirs
        assert_eq!(rule.original_rules.len(), 3);
    }

    #[test]
    fn test_consolidate_with_subdirs() {
        let suggestions = vec![
            make_suggestion("test_profile", "  /opt/intellij/plugins/a/foo.jar r,"),
            make_suggestion("test_profile", "  /opt/intellij/plugins/b/bar.jar r,"),
            make_suggestion("test_profile", "  /opt/intellij/plugins/baz.jar r,"),
        ];

        let results = consolidate(&suggestions);
        assert_eq!(results.len(), 1);
        let result = &results[0];

        // Should use ** because some paths have subdirectories
        let glob_rules: Vec<_> = result.rules.iter().filter(|r| r.is_glob).collect();
        assert_eq!(glob_rules.len(), 1);
        assert!(glob_rules[0].rule_text.contains("/opt/intellij/plugins/**"));
    }

    #[test]
    fn test_no_consolidation_below_threshold() {
        let suggestions = vec![
            make_suggestion("test_profile", "  /opt/intellij/plugins/foo.jar r,"),
            make_suggestion("test_profile", "  /opt/intellij/plugins/bar.jar r,"),
        ];

        let results = consolidate(&suggestions);
        assert_eq!(results.len(), 1);
        let result = &results[0];
        assert_eq!(result.rules.len(), 2);
        assert!(result.rules.iter().all(|r| !r.is_glob));
    }

    #[test]
    fn test_different_masks_separate_globs() {
        let suggestions = vec![
            make_suggestion("test_profile", "  /opt/app/a.so r,"),
            make_suggestion("test_profile", "  /opt/app/b.so r,"),
            make_suggestion("test_profile", "  /opt/app/c.so r,"),
            make_suggestion("test_profile", "  /opt/app/x.conf rw,"),
            make_suggestion("test_profile", "  /opt/app/y.conf rw,"),
            make_suggestion("test_profile", "  /opt/app/z.conf rw,"),
        ];

        let results = consolidate(&suggestions);
        assert_eq!(results.len(), 1);
        let result = &results[0];

        let glob_rules: Vec<_> = result.rules.iter().filter(|r| r.is_glob).collect();
        assert_eq!(glob_rules.len(), 2);

        // Both should be globs for /opt/app/* but with different masks
        let masks: Vec<&str> = glob_rules.iter().map(|r| {
            if r.rule_text.contains(" r,") { "r" } else { "rw" }
        }).collect();
        assert!(masks.contains(&"r"));
        assert!(masks.contains(&"rw"));
    }

    #[test]
    fn test_non_file_rules_pass_through() {
        let suggestions = vec![
            make_suggestion("test_profile", "  network inet stream,"),
            make_suggestion("test_profile", "  ptrace read,"),
            make_suggestion("test_profile", "  signal send,"),
        ];

        let results = consolidate(&suggestions);
        assert_eq!(results.len(), 1);
        let result = &results[0];
        assert_eq!(result.rules.len(), 3);
        assert!(result.rules.iter().all(|r| !r.is_glob));
    }

    #[test]
    fn test_mixed_profiles() {
        let suggestions = vec![
            make_suggestion("profile_a", "  /opt/app/a.so r,"),
            make_suggestion("profile_a", "  /opt/app/b.so r,"),
            make_suggestion("profile_a", "  /opt/app/c.so r,"),
            make_suggestion("profile_b", "  /usr/lib/foo.so r,"),
        ];

        let results = consolidate(&suggestions);
        assert_eq!(results.len(), 2);

        let a = results.iter().find(|r| r.profile == "profile_a").unwrap();
        assert_eq!(a.rules.len(), 1);
        assert!(a.rules[0].is_glob);

        let b = results.iter().find(|r| r.profile == "profile_b").unwrap();
        assert_eq!(b.rules.len(), 1);
        assert!(!b.rules[0].is_glob);
    }

    #[test]
    fn test_already_globbed_rules_pass_through() {
        let suggestions = vec![
            make_suggestion("test_profile", "  /opt/app/** r,"),
        ];

        let results = consolidate(&suggestions);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].rules.len(), 1);
        assert!(!results[0].rules[0].is_glob); // not consolidated, just passed through
    }

    #[test]
    fn test_empty_input() {
        let results = consolidate(&[]);
        assert!(results.is_empty());
    }

    #[test]
    fn test_parse_file_rule() {
        assert_eq!(parse_file_rule("  /foo/bar r,"), Some(("/foo/bar", "r")));
        assert_eq!(parse_file_rule("  /foo/bar rw,"), Some(("/foo/bar", "rw")));
        assert_eq!(parse_file_rule("  /foo/** r,"), Some(("/foo/**", "r")));
        assert_eq!(parse_file_rule("  network inet stream,"), None);
        assert_eq!(parse_file_rule("  ptrace read,"), None);
    }

    #[test]
    fn test_consolidate_raw_rules_basic() {
        let rules = vec![
            "  /opt/intellij/plugins/a.jar r,".to_string(),
            "  /opt/intellij/plugins/b.jar r,".to_string(),
            "  /opt/intellij/plugins/c.jar r,".to_string(),
            "  network inet stream,".to_string(),
        ];

        let result = consolidate_raw_rules("test_profile", &rules);
        assert_eq!(result.profile, "test_profile");

        let globs: Vec<_> = result.rules.iter().filter(|r| r.is_glob).collect();
        assert_eq!(globs.len(), 1);
        assert!(globs[0].rule_text.contains("/opt/intellij/plugins/*"));
        assert_eq!(globs[0].original_rules.len(), 3);

        // Non-file rule should pass through
        let non_globs: Vec<_> = result.rules.iter().filter(|r| !r.is_glob).collect();
        assert!(non_globs.iter().any(|r| r.rule_text.contains("network")));
    }

    #[test]
    fn test_consolidate_raw_rules_no_consolidation_needed() {
        let rules = vec![
            "  /opt/app/config.yaml r,".to_string(),
            "  /usr/bin/bash ix,".to_string(),
            "  network inet stream,".to_string(),
        ];

        let result = consolidate_raw_rules("test_profile", &rules);
        assert!(result.rules.iter().all(|r| !r.is_glob));
    }

    #[test]
    fn test_consolidate_raw_rules_mixed() {
        let rules = vec![
            "  /opt/intellij/plugins/a/x.jar r,".to_string(),
            "  /opt/intellij/plugins/b/y.jar r,".to_string(),
            "  /opt/intellij/plugins/c.jar r,".to_string(),
            "  /opt/intellij/lib/a.jar r,".to_string(),
            "  /opt/intellij/lib/b.jar r,".to_string(),
            "  /opt/intellij/lib/c.jar r,".to_string(),
            "  ptrace read,".to_string(),
        ];

        let result = consolidate_raw_rules("intellij", &rules);

        let globs: Vec<_> = result.rules.iter().filter(|r| r.is_glob).collect();
        // Should have 2 globs: lib/* and plugins/**
        assert_eq!(globs.len(), 2);
    }
}
