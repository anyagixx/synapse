// MODULE_CONTRACT
// MODULE_ID: M-CLI-RTK-COMMANDS
// PURPOSE: First-class RTK-style CLI shortcuts and hook-facing rewrite decisions
// SCOPE: RtkProxyCmd shortcut dispatch for read, ls, tree, find, rg, grep, git, cargo, npm, pnpm, npx, and pytest; RewriteCmd dry-run rewriting for agent hooks
// DEPENDS: M-CONFIG, M-CLI-RUNTIME-COMMANDS, M-PROXY-ROUTER
// LINKS:
//   → M-CLI-RUNTIME-COMMANDS (depends) - delegates execution to ProxyCmd
//   → M-PROXY (depends) - proxy execution, filtering, tracking, and evidence
//   → UC-002 (implements) - token-saving command execution evidence
//   → NFR-003 (traces_to) - shorter command surface increases proxy adoption

// START_MODULE_MAP
// RtkProxyCmd::run_as — Prefixes a native command and delegates to ProxyCmd
// RewriteCmd::run — Prints a hook rewrite decision or exits 1 when unsupported
// rewrite_command — Converts routeable shell commands to syn proxy invocations
// split_route_tokens — Separates transparent shell prefixes from routeable command tokens
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v1.1.0 — Added syn rewrite hook decision path]
// END_CHANGE_SUMMARY

use super::{ProxyCmd, RewriteCmd, RtkProxyCmd};
use crate::config::Config;

// START_public_api

impl RtkProxyCmd {
    // START_CONTRACT_RtkProxyCmd::run_as
    // PURPOSE: Execute a first-class RTK-style shortcut by prefixing the native executable and delegating to syn proxy
    // INPUTS: { config: Config }, { executable: &str - native command to run }, { require_args: bool }, { usage: &str }
    // OUTPUTS: { anyhow::Result<()> }
    // SIDE_EFFECTS: may run external command through ProxyCmd
    // LINKS:
    //   → M-CLI-RUNTIME-COMMANDS (depends) - reuses ProxyCmd route/evidence/exit behavior
    //   → M-PROXY (depends) - applies token-saving routing, filtering, tracking, and evidence capture
    // START_rtk_proxy_cmd_run_as
    pub async fn run_as(
        &self,
        config: Config,
        executable: &str,
        require_args: bool,
        usage: &str,
    ) -> anyhow::Result<()> {
        if require_args && self.args.is_empty() {
            anyhow::bail!(usage.to_string());
        }

        let mut args = Vec::with_capacity(self.args.len() + 1);
        args.push(executable.to_string());
        args.extend(self.args.iter().cloned());

        ProxyCmd {
            route: self.route,
            evidence: self.evidence,
            args,
        }
        .run(config)
        .await
    }
    // END_rtk_proxy_cmd_run_as
}

impl RewriteCmd {
    // START_CONTRACT_RewriteCmd::run
    // PURPOSE: Print the Synapse proxy rewrite for a shell command, or exit 1 when no route is supported
    // INPUTS: { config: Config }
    // OUTPUTS: { anyhow::Result<()> }
    // SIDE_EFFECTS: writes rewrite to stdout or exits 1 with no stdout for pass-through commands
    // LINKS:
    //   → M-PROXY-ROUTER (depends) - router is the single source of truth for hook rewrite decisions
    //   → M-HOOK-OPENCODE-REWRITE (implements) - hooks delegate rewrite decisions to this command
    // START_rewrite_cmd_run
    pub async fn run(&self, _config: Config) -> anyhow::Result<()> {
        match rewrite_command(&self.args) {
            Some(rewritten) => {
                println!("{rewritten}");
                Ok(())
            }
            None => std::process::exit(1),
        }
    }
    // END_rewrite_cmd_run
}

// START_CONTRACT_rewrite_command
// PURPOSE: Convert a routeable raw shell command into a syn proxy invocation for thin agent hooks
// INPUTS: { args: &[String] - raw command as one shell string or argv tokens }
// OUTPUTS: { Option<String> - rewritten command when supported }
// LINKS:
//   → M-PROXY-ROUTER (depends) - route decisions define supported command families
//   → NFR-003 (traces_to) - auto-rewrite increases token-saving proxy coverage
// START_rewrite_command
pub(crate) fn rewrite_command(args: &[String]) -> Option<String> {
    let raw = raw_command(args)?;
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    if is_already_token_safe(trimmed) {
        return Some(trimmed.to_string());
    }

    let tokens = command_tokens(args, trimmed)?;
    if has_shell_operator(&tokens) {
        return None;
    }

    let (prefix_tokens, route_tokens) = split_route_tokens(&tokens);
    if route_tokens.is_empty() {
        return None;
    }

    let router = crate::proxy::router::CommandRouter::new();
    let decision = router.route(&route_tokens);
    if decision.should_proxy {
        Some(format_rewritten_command(&prefix_tokens, &route_tokens))
    } else {
        None
    }
}
// END_rewrite_command

// END_public_api

// START_CONTRACT_raw_command
// PURPOSE: Build a shell-safe raw command from clap args while preserving single-string hook input
// INPUTS: { args: &[String] }
// OUTPUTS: { Option<String> }
// START_raw_command
fn raw_command(args: &[String]) -> Option<String> {
    match args {
        [] => None,
        [single] => Some(single.trim().to_string()),
        many => Some(
            many.iter()
                .map(|arg| shell_quote(arg))
                .collect::<Vec<_>>()
                .join(" "),
        ),
    }
}
// END_raw_command

// START_CONTRACT_command_tokens
// PURPOSE: Parse hook input into tokens for router classification
// INPUTS: { args: &[String] }, { raw: &str }
// OUTPUTS: { Option<Vec<String>> }
// START_command_tokens
fn command_tokens(args: &[String], raw: &str) -> Option<Vec<String>> {
    if args.len() == 1 {
        split_shell_words(raw)
    } else {
        Some(args.to_vec())
    }
}
// END_command_tokens

// START_CONTRACT_split_route_tokens
// PURPOSE: Separate simple environment assignments and transparent prefixes from router classification tokens
// INPUTS: { tokens: &[String] }
// OUTPUTS: { (Vec<String>, Vec<String>) }
// START_split_route_tokens
fn split_route_tokens(tokens: &[String]) -> (Vec<String>, Vec<String>) {
    let mut index = 0;
    while index < tokens.len() {
        let token = tokens[index].as_str();
        if is_env_assignment(token) || matches!(token, "sudo" | "command" | "noglob" | "exec") {
            index += 1;
            continue;
        }
        if token == "env" {
            index += 1;
            continue;
        }
        break;
    }
    (tokens[..index].to_vec(), tokens[index..].to_vec())
}
// END_split_route_tokens

// START_CONTRACT_has_shell_operator
// PURPOSE: Detect shell command composition that this minimal rewrite path intentionally leaves untouched
// INPUTS: { tokens: &[String] }
// OUTPUTS: { bool }
// START_has_shell_operator
fn has_shell_operator(tokens: &[String]) -> bool {
    tokens
        .iter()
        .any(|token| matches!(token.as_str(), "&&" | "||" | ";" | "|" | "&"))
}
// END_has_shell_operator

// START_CONTRACT_format_rewritten_command
// PURPOSE: Render a shell-safe rewrite while preserving env/transparent prefixes before syn proxy
// INPUTS: { prefix_tokens: &[String] }, { route_tokens: &[String] }
// OUTPUTS: { String }
// START_format_rewritten_command
fn format_rewritten_command(prefix_tokens: &[String], route_tokens: &[String]) -> String {
    let command = route_tokens
        .iter()
        .map(|token| shell_quote(token))
        .collect::<Vec<_>>()
        .join(" ");
    if prefix_tokens.is_empty() {
        format!("syn proxy -- {command}")
    } else {
        let prefix = prefix_tokens
            .iter()
            .map(|token| shell_quote(token))
            .collect::<Vec<_>>()
            .join(" ");
        format!("{prefix} syn proxy -- {command}")
    }
}
// END_format_rewritten_command

// START_CONTRACT_is_already_token_safe
// PURPOSE: Detect commands that are already routed through a token-saving Synapse or RTK entry point
// INPUTS: { command: &str }
// OUTPUTS: { bool }
// START_is_already_token_safe
fn is_already_token_safe(command: &str) -> bool {
    command.starts_with("syn proxy ")
        || command.starts_with("syn proxy --")
        || command.starts_with("syn read ")
        || command.starts_with("syn ls")
        || command.starts_with("syn rg ")
        || command.starts_with("syn grep ")
        || command.starts_with("syn git ")
        || command.starts_with("syn cargo ")
        || command.starts_with("syn npm ")
        || command.starts_with("syn pnpm ")
        || command.starts_with("syn npx ")
        || command.starts_with("syn pytest")
        || command.starts_with("rtk ")
}
// END_is_already_token_safe

// START_CONTRACT_is_env_assignment
// PURPOSE: Identify simple shell environment assignment tokens
// INPUTS: { token: &str }
// OUTPUTS: { bool }
// START_is_env_assignment
fn is_env_assignment(token: &str) -> bool {
    let Some((name, _value)) = token.split_once('=') else {
        return false;
    };
    let mut chars = name.chars();
    matches!(chars.next(), Some('_') | Some('a'..='z') | Some('A'..='Z'))
        && chars.all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
}
// END_is_env_assignment

// START_CONTRACT_shell_quote
// PURPOSE: Quote argv tokens so syn rewrite multi-arg invocations remain shell-safe
// INPUTS: { value: &str }
// OUTPUTS: { String }
// START_shell_quote
fn shell_quote(value: &str) -> String {
    if value.is_empty() {
        return "''".into();
    }
    if value.chars().all(|ch| {
        ch.is_ascii_alphanumeric()
            || matches!(ch, '-' | '_' | '/' | '.' | ':' | '=' | '@' | '%' | '+')
    }) {
        return value.to_string();
    }
    format!("'{}'", value.replace('\'', "'\\''"))
}
// END_shell_quote

// START_CONTRACT_split_shell_words
// PURPOSE: Parse a minimal POSIX-like shell string for router classification without executing it
// INPUTS: { input: &str }
// OUTPUTS: { Option<Vec<String>> }
// START_split_shell_words
fn split_shell_words(input: &str) -> Option<Vec<String>> {
    let mut words = Vec::new();
    let mut current = String::new();
    let mut chars = input.chars().peekable();
    let mut in_single = false;
    let mut in_double = false;

    while let Some(ch) = chars.next() {
        match ch {
            '\'' if !in_double => in_single = !in_single,
            '"' if !in_single => in_double = !in_double,
            '\\' if !in_single => {
                if let Some(next) = chars.next() {
                    current.push(next);
                }
            }
            ch if ch.is_whitespace() && !in_single && !in_double => {
                if !current.is_empty() {
                    words.push(std::mem::take(&mut current));
                }
            }
            _ => current.push(ch),
        }
    }

    if in_single || in_double {
        return None;
    }
    if !current.is_empty() {
        words.push(current);
    }
    Some(words)
}
// END_split_shell_words

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rewrite_command_routes_simple_command() {
        let args = vec!["git status".to_string()];
        assert_eq!(
            rewrite_command(&args),
            Some("syn proxy -- git status".to_string())
        );
    }

    #[test]
    fn rewrite_command_quotes_multi_arg_input() {
        let args = vec![
            "git".to_string(),
            "commit".to_string(),
            "-m".to_string(),
            "it's fixed".to_string(),
        ];
        assert_eq!(
            rewrite_command(&args),
            Some("syn proxy -- git commit -m 'it'\\''s fixed'".to_string())
        );
    }

    #[test]
    fn rewrite_command_skips_unknown_command() {
        let args = vec!["htop".to_string()];
        assert_eq!(rewrite_command(&args), None);
    }

    #[test]
    fn rewrite_command_preserves_env_prefix_before_proxy() {
        let args = vec!["FOO=1 git status".to_string()];
        assert_eq!(
            rewrite_command(&args),
            Some("FOO=1 syn proxy -- git status".to_string())
        );
    }

    #[test]
    fn rewrite_command_skips_shell_composition() {
        let args = vec!["git status && cargo test".to_string()];
        assert_eq!(rewrite_command(&args), None);
    }

    #[test]
    fn split_shell_words_handles_quotes() {
        assert_eq!(
            split_shell_words("git commit -m \"hello world\"").unwrap(),
            vec!["git", "commit", "-m", "hello world"]
        );
    }
}
