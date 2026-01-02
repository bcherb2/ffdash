/// FFmpeg command assertion utilities
#[allow(dead_code)]
pub fn assert_cmd_contains(cmd: &str, flag: &str) {
    assert!(
        cmd.contains(flag),
        "Expected FFmpeg command to contain '{}' but it didn't.\nCommand: {}",
        flag,
        cmd
    );
}

/// Check if a command string does NOT contain a specific flag
#[allow(dead_code)]
pub fn assert_cmd_not_contains(cmd: &str, flag: &str) {
    assert!(
        !cmd.contains(flag),
        "Expected FFmpeg command to NOT contain '{}' but it did.\nCommand: {}",
        flag,
        cmd
    );
}

/// Check if a command contains a flag with a specific value
#[allow(dead_code)]
pub fn assert_cmd_has_flag_value(cmd: &str, flag: &str, value: &str) {
    let pattern = format!("{} {}", flag, value);
    assert!(
        cmd.contains(&pattern),
        "Expected FFmpeg command to contain '{} {}' but it didn't.\nCommand: {}",
        flag,
        value,
        cmd
    );
}

/// Check if a command contains any of the given flags
#[allow(dead_code)]
pub fn assert_cmd_contains_any(cmd: &str, flags: &[&str]) {
    let found = flags.iter().any(|flag| cmd.contains(flag));
    assert!(
        found,
        "Expected FFmpeg command to contain at least one of {:?} but none were found.\nCommand: {}",
        flags, cmd
    );
}

/// Check if a command contains all of the given flags
#[allow(dead_code)]
pub fn assert_cmd_contains_all(cmd: &str, flags: &[&str]) {
    for flag in flags {
        assert_cmd_contains(cmd, flag);
    }
}

/// Parse a flag value from the command (e.g., get "30" from "-crf 30")
#[allow(dead_code)]
pub fn get_flag_value<'a>(cmd: &'a str, flag: &str) -> Option<&'a str> {
    let pattern = format!("{} ", flag);
    cmd.find(&pattern).and_then(|pos| {
        let after_flag = &cmd[pos + pattern.len()..];
        after_flag.split_whitespace().next()
    })
}

/// Assert that a numeric flag has a specific value
#[allow(dead_code)]
pub fn assert_numeric_flag(cmd: &str, flag: &str, expected: i32) {
    if let Some(value_str) = get_flag_value(cmd, flag) {
        let value: i32 = value_str.parse().unwrap_or_else(|_| {
            panic!("Could not parse value '{}' for flag '{}'", value_str, flag)
        });
        assert_eq!(
            value, expected,
            "Expected {} to be {} but got {}",
            flag, expected, value
        );
    } else {
        panic!("Flag '{}' not found in command: {}", flag, cmd);
    }
}

/// Assert that mutually exclusive flags are not both present
#[allow(dead_code)]
pub fn assert_mutually_exclusive(cmd: &str, flag1: &str, flag2: &str) {
    let has_flag1 = cmd.contains(flag1);
    let has_flag2 = cmd.contains(flag2);

    assert!(
        !(has_flag1 && has_flag2),
        "Mutually exclusive flags '{}' and '{}' both found in command: {}",
        flag1,
        flag2,
        cmd
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_flag_value() {
        let cmd = "ffmpeg -i input.mp4 -crf 30 -b:v 2000k output.webm";
        assert_eq!(get_flag_value(cmd, "-crf"), Some("30"));
        assert_eq!(get_flag_value(cmd, "-b:v"), Some("2000k"));
        assert_eq!(get_flag_value(cmd, "-nonexistent"), None);
    }

    #[test]
    fn test_assert_cmd_contains() {
        let cmd = "ffmpeg -i input.mp4 -c:v libvpx-vp9";
        assert_cmd_contains(cmd, "-c:v");
        assert_cmd_contains(cmd, "libvpx-vp9");
    }

    #[test]
    #[should_panic(expected = "Expected FFmpeg command to contain")]
    fn test_assert_cmd_contains_fails() {
        let cmd = "ffmpeg -i input.mp4";
        assert_cmd_contains(cmd, "-nonexistent");
    }
}
