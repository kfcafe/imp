use std::hash::{Hash, Hasher};

const LUA_RESTART_DIRECTIVE: &str = "__IMP_RESTART_AFTER_COMMAND__";

pub(super) fn lua_result_requests_restart(result: Option<&str>) -> bool {
    result.is_some_and(|text| {
        text.lines()
            .any(|line| line.trim() == LUA_RESTART_DIRECTIVE)
    })
}

pub(super) fn strip_lua_restart_directive(result: &str) -> String {
    result
        .lines()
        .filter(|line| line.trim() != LUA_RESTART_DIRECTIVE)
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

pub(super) fn command_option_value(args: &str, option: &str) -> Option<String> {
    let needle = format!("--{option}");
    let (_, tail) = args.split_once(&needle)?;
    let value = tail.trim();
    if value.is_empty() {
        return None;
    }
    let next_option = value.find(" --").unwrap_or(value.len());
    Some(value[..next_option].trim().to_string()).filter(|value| !value.is_empty())
}

pub(super) fn parse_secret_field_names(input: &str) -> Vec<String> {
    let names: Vec<String> = input
        .split(',')
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(|name| name.to_string())
        .collect();
    if names.is_empty() {
        vec!["api_key".to_string()]
    } else {
        names
    }
}

pub(super) fn bump_epoch(epoch: &mut u64) {
    *epoch = epoch.wrapping_add(1);
}

pub(super) fn stable_hash<T: Hash>(value: &T) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

pub(super) fn single_line_preview(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}
