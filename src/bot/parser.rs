//! Slash command parser

/// Parse "/command args" from input.
/// Returns (command, args) or None if not a command.
pub fn parse_command(input: &str) -> Option<(&str, &str)> {
    let trimmed = input.trim();
    if !trimmed.starts_with('/') {
        return None;
    }
    let rest = &trimmed[1..];
    if rest.is_empty() {
        return Some(("", ""));
    }
    let (cmd, args) = match rest.split_once(char::is_whitespace) {
        Some((c, a)) => (c, a.trim()),
        None => (rest, ""),
    };
    Some((cmd, args))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_full_command() {
        let (cmd, args) = parse_command("/translate on").unwrap();
        assert_eq!(cmd, "translate");
        assert_eq!(args, "on");
    }

    #[test]
    fn parse_command_no_args() {
        let (cmd, args) = parse_command("/help").unwrap();
        assert_eq!(cmd, "help");
        assert_eq!(args, "");
    }

    #[test]
    fn parse_slash_only() {
        let (cmd, args) = parse_command("/").unwrap();
        assert_eq!(cmd, "");
        assert_eq!(args, "");
    }

    #[test]
    fn parse_not_command() {
        assert!(parse_command("hello").is_none());
    }

    #[test]
    fn parse_empty() {
        assert!(parse_command("").is_none());
    }

    #[test]
    fn parse_extra_whitespace() {
        let (cmd, args) = parse_command("/translate   on  ").unwrap();
        assert_eq!(cmd, "translate");
        assert_eq!(args, "on");
    }
}
