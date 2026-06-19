//! Custom regex rule checks. Corresponds to Go `internal/checker/custom.go`.

use std::sync::atomic::AtomicBool;

use cc_config::{Config, CustomRule};
use cc_gitdiff::FileDiff;
use regex::Regex;

/// Applies custom regex rules to a commit message. Corresponds to Go `CheckMsgCustomRules`.
/// required rules error if the pattern is not found in the full message; forbidden (default) rules error if the pattern is found in any line.
pub fn check_msg_custom_rules(content: &str, rules: &[CustomRule]) -> Vec<String> {
    let mut errs = Vec::new();
    for rule in rules {
        if rule.pattern.is_empty() {
            continue;
        }
        let re = match Regex::new(&rule.pattern) {
            Ok(r) => r,
            Err(_) => continue,
        };
        let msg = if rule.message.is_empty() {
            format!("pattern: {}", rule.pattern)
        } else {
            rule.message.clone()
        };

        if rule.required {
            if !re.is_match(content) {
                errs.push(cc_i18n::t!(
                    "msg.custom_rule_required",
                    Name = rule.name,
                    Message = msg
                ));
            }
        } else {
            for (i, line) in content.split('\n').enumerate() {
                if re.is_match(line) {
                    errs.push(cc_i18n::t!(
                        "msg.custom_rule_forbidden",
                        Line = i + 1,
                        Name = rule.name,
                        Message = msg
                    ));
                }
            }
        }
    }
    errs
}

/// Applies forbidden custom rules to added lines in the staged diff. Corresponds to Go `CheckDiffCustomRules`.
pub fn check_diff_custom_rules(
    cfg: &Config,
    diffs: &[FileDiff],
    cancel: &AtomicBool,
) -> Result<Vec<String>, String> {
    let rules = &cfg.custom_rules.diff;
    if rules.is_empty() {
        return Ok(Vec::new());
    }

    // Compile only forbidden rules.
    let compiled: Vec<(&CustomRule, Regex)> = rules
        .iter()
        .filter(|r| !r.pattern.is_empty() && !r.required)
        .filter_map(|r| Regex::new(&r.pattern).ok().map(|re| (r, re)))
        .collect();
    if compiled.is_empty() {
        return Ok(Vec::new());
    }

    let ignore_patterns = &cfg.exceptions.global_ignore;
    let mut errs = Vec::new();

    for diff in diffs {
        crate::check_cancelled(cancel)?;
        if diff.is_deleted {
            continue;
        }
        if cc_pathutil::matches_any(&diff.path, ignore_patterns) {
            continue;
        }
        if diff.added_lines.is_empty() {
            continue;
        }

        let staged_content = match cc_gitdiff::get_staged_content(&diff.path) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let lines: Vec<&str> = staged_content.split('\n').collect();

        // Sort added line numbers for deterministic output (Go map iteration is non-deterministic).
        let mut line_nums: Vec<i64> = diff.added_lines.iter().copied().collect();
        line_nums.sort_unstable();

        for line_num in line_nums {
            if line_num < 1 || line_num as usize > lines.len() {
                continue;
            }
            let line = lines[(line_num - 1) as usize];
            for (rule, re) in &compiled {
                if re.is_match(line) {
                    let msg = if rule.message.is_empty() {
                        format!("pattern: {}", rule.pattern)
                    } else {
                        rule.message.clone()
                    };
                    errs.push(cc_i18n::t!(
                        "diff.custom_rule_forbidden",
                        Path = diff.path,
                        Line = line_num,
                        Name = rule.name,
                        Message = msg
                    ));
                }
            }
        }
    }
    Ok(errs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cc_config::CustomRule;

    fn rule(name: &str, pattern: &str, message: &str, required: bool) -> CustomRule {
        CustomRule {
            name: name.to_string(),
            pattern: pattern.to_string(),
            message: message.to_string(),
            required,
        }
    }

    #[test]
    fn forbidden_rule_matches_line() {
        let rules = vec![rule("no_wip", "(?i)wip", "WIP 금지", false)];
        let errs = check_msg_custom_rules("WIP: something\nbody", &rules);
        assert_eq!(errs.len(), 1);
    }

    #[test]
    fn required_rule_missing() {
        let rules = vec![rule("need_ticket", r"\[#\d+\]", "티켓 필요", true)];
        let errs = check_msg_custom_rules("feat: no ticket", &rules);
        assert_eq!(errs.len(), 1);
        let errs2 = check_msg_custom_rules("feat: has [#123]", &rules);
        assert!(errs2.is_empty());
    }

    #[test]
    fn empty_pattern_skipped() {
        let rules = vec![rule("x", "", "m", false)];
        assert!(check_msg_custom_rules("anything", &rules).is_empty());
    }
}
