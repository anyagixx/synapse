// MODULE_CONTRACT
// MODULE_ID: M-PROXY-ROUTER
// PURPOSE: RTK-style command router — classifies shell commands into token-saving adapter families
// SCOPE: CommandRouter, RouteDecision, SupportedAdapter, route normalization, adapter support catalogue, RTK parity router-family coverage gate
// DEPENDS: N/A
// LINKS:
//   → UC-002 (implements) - command routing makes token-saving behavior machine-checkable
//   → NFR-003 (traces_to) - adapter routing improves token economy analytics and filtering precision
//   ← V-M-PROXY-ROUTER (verified_by) - command-router verification shard

// START_MODULE_MAP
// CommandRouter — Classifies command argv into an adapter route
// RouteDecision — Machine-readable routing decision used by proxy and analytics
// SupportedAdapter — Documented adapter family entry for route previews and hooks
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v1.1.0 — Expanded adapter catalogue for RTK parity inventory]
// END_CHANGE_SUMMARY

// START_public_api

// START_RouteDecision
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteDecision {
    pub should_proxy: bool,
    pub adapter: String,
    pub family: String,
    pub route_key: String,
    pub filter_key: String,
    pub reason: String,
}
// END_RouteDecision

// START_SupportedAdapter
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SupportedAdapter {
    pub adapter: &'static str,
    pub family: &'static str,
    pub examples: &'static [&'static str],
}
// END_SupportedAdapter

// START_CommandRouter
#[derive(Debug, Default, Clone)]
pub struct CommandRouter;
// END_CommandRouter

impl CommandRouter {
    // START_CONTRACT_CommandRouter::new
    // PURPOSE: Create a command router with the built-in adapter catalogue
    // OUTPUTS: { CommandRouter }
    // LINKS:
    //   → NFR-003 (traces_to) - routing is the entry point for command-specific token savings
    // START_command_router_new
    pub fn new() -> Self {
        Self
    }
    // END_command_router_new

    // START_CONTRACT_CommandRouter::route
    // PURPOSE: Classify argv into a token-saving adapter decision without executing the command
    // INPUTS: { parts: &[String] — command argv }
    // OUTPUTS: { RouteDecision }
    // LINKS:
    //   → UC-002 (implements) - route decisions are machine-readable workflow evidence
    //   → NFR-003 (traces_to) - command-specific adapters reduce token-heavy shell output
    // START_command_router_route
    pub fn route(&self, parts: &[String]) -> RouteDecision {
        route_parts(parts)
    }
    // END_command_router_route

    // START_CONTRACT_CommandRouter::supported_adapters
    // PURPOSE: Return the built-in adapter catalogue for CLI previews and hook generation
    // OUTPUTS: { Vec<SupportedAdapter> }
    // LINKS:
    //   → UC-002 (implements) - agents can inspect supported routed command families
    // START_command_router_supported_adapters
    pub fn supported_adapters(&self) -> Vec<SupportedAdapter> {
        supported_adapters()
    }
    // END_command_router_supported_adapters
}

// START_CONTRACT_route_parts
// PURPOSE: Route command argv to a known adapter or passthrough decision
// INPUTS: { parts: &[String] — command argv }
// OUTPUTS: { RouteDecision }
// LINKS:
//   → NFR-003 (traces_to) - adapter-level classification drives savings analytics
// START_route_parts
fn route_parts(parts: &[String]) -> RouteDecision {
    let tokens = normalized_tokens(parts);
    if tokens.is_empty() {
        return passthrough("empty command");
    }

    if matches_prefix(&tokens, &["syn", "proxy"]) || matches_prefix(&tokens, &["rtk"]) {
        return passthrough("already routed through a token proxy");
    }

    if tokens[0] == "git" {
        return route_git(&tokens);
    }
    if tokens[0] == "cargo" {
        return route_two_token("rust-cargo", "rust", &tokens, "cargo command");
    }
    if tokens[0] == "go" || tokens[0] == "golangci-lint" {
        return route_go(&tokens);
    }
    if tokens[0] == "uv" {
        return route_uv(&tokens);
    }
    if matches!(
        tokens[0].as_str(),
        "npm" | "pnpm" | "yarn" | "npx" | "bun" | "deno"
    ) {
        return route_js(&tokens);
    }
    if tokens[0] == "pytest" || matches_prefix(&tokens, &["python", "-m", "pytest"]) {
        return route(
            "python-pytest",
            "python",
            "python -m pytest",
            "pytest runner",
        );
    }
    if matches!(
        tokens[0].as_str(),
        "ruff" | "mypy" | "basedpyright" | "pip" | "pip3"
    ) {
        return route_two_token("python-tooling", "python", &tokens, "python tooling");
    }
    if tokens[0] == "docker" {
        return route_docker(&tokens);
    }
    if matches!(
        tokens[0].as_str(),
        "kubectl" | "helm" | "terraform" | "tofu"
    ) {
        return route_two_token("infra-cli", "infrastructure", &tokens, "infrastructure CLI");
    }
    if matches!(tokens[0].as_str(), "gh" | "glab") {
        return route_two_token("vcs-hosting", "vcs", &tokens, "hosting CLI");
    }
    if matches!(
        tokens[0].as_str(),
        "rg" | "grep"
            | "find"
            | "ls"
            | "cat"
            | "tree"
            | "wc"
            | "du"
            | "ps"
            | "systemctl"
            | "journalctl"
            | "env"
            | "printenv"
    ) {
        return route_system(&tokens);
    }
    if matches!(
        tokens[0].as_str(),
        "make" | "just" | "gradle" | "gradlew" | "./gradlew"
    ) {
        return route_two_token("build-tool", "build", &tokens, "build tool");
    }
    if matches!(
        tokens[0].as_str(),
        "dotnet" | "rake" | "rspec" | "rubocop" | "bundle"
    ) {
        return route_two_token("language-tooling", "language", &tokens, "language tooling");
    }
    if matches!(tokens[0].as_str(), "aws" | "psql" | "curl" | "wget" | "jq") {
        return route_two_token("cloud-data", "cloud", &tokens, "cloud/data CLI");
    }

    passthrough("no specialized adapter")
}
// END_route_parts

// START_CONTRACT_supported_adapters
// PURPOSE: Return the RTK-style adapter catalogue supported by the router
// OUTPUTS: { Vec<SupportedAdapter> }
// LINKS:
//   → UC-002 (implements) - exposes command-router coverage to agents and diagnostics
// START_supported_adapters
fn supported_adapters() -> Vec<SupportedAdapter> {
    vec![
        SupportedAdapter {
            adapter: "vcs-git",
            family: "vcs",
            examples: &["git status", "git diff", "git log", "git show"],
        },
        SupportedAdapter {
            adapter: "rust-cargo",
            family: "rust",
            examples: &["cargo test", "cargo check", "cargo clippy", "cargo fmt"],
        },
        SupportedAdapter {
            adapter: "go-tooling",
            family: "go",
            examples: &["go test", "go build", "golangci-lint run"],
        },
        SupportedAdapter {
            adapter: "python-pytest",
            family: "python",
            examples: &["pytest", "python -m pytest", "uv run pytest"],
        },
        SupportedAdapter {
            adapter: "python-tooling",
            family: "python",
            examples: &["ruff check", "mypy", "basedpyright", "pip list"],
        },
        SupportedAdapter {
            adapter: "js-tooling",
            family: "javascript",
            examples: &["npm test", "pnpm build", "npx tsc", "bun test"],
        },
        SupportedAdapter {
            adapter: "infra-cli",
            family: "infrastructure",
            examples: &["docker compose logs", "kubectl get pods", "terraform plan"],
        },
        SupportedAdapter {
            adapter: "vcs-hosting",
            family: "vcs",
            examples: &["gh pr checks", "gh pr view", "glab mr list"],
        },
        SupportedAdapter {
            adapter: "system-text",
            family: "system",
            examples: &["rg pattern", "find .", "journalctl -u service"],
        },
        SupportedAdapter {
            adapter: "system-search",
            family: "search",
            examples: &["rg pattern", "grep needle", "find . -name '*.rs'"],
        },
        SupportedAdapter {
            adapter: "system-logs",
            family: "logs",
            examples: &["journalctl -u service", "systemctl status service"],
        },
        SupportedAdapter {
            adapter: "build-tool",
            family: "build",
            examples: &["make test", "just check", "./gradlew test"],
        },
        SupportedAdapter {
            adapter: "language-tooling",
            family: "language",
            examples: &["dotnet test", "rake test", "rspec"],
        },
        SupportedAdapter {
            adapter: "cloud-data",
            family: "cloud",
            examples: &["aws ec2 describe-instances", "psql -c", "jq ."],
        },
    ]
}
// END_supported_adapters

// END_public_api

// START_CONTRACT_normalized_tokens
// PURPOSE: Trim shell argv tokens and discard empty values before routing
// INPUTS: { parts: &[String] }
// OUTPUTS: { Vec<String> }
// LINKS:
//   → NFR-002 (traces_to) - normalization keeps routing deterministic
// START_normalized_tokens
fn normalized_tokens(parts: &[String]) -> Vec<String> {
    parts
        .iter()
        .map(|part| part.trim())
        .filter(|part| !part.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}
// END_normalized_tokens

// START_CONTRACT_matches_prefix
// PURPOSE: Check whether normalized argv starts with a specific token prefix
// INPUTS: { tokens: &[String] }, { prefix: &[&str] }
// OUTPUTS: { bool }
// LINKS:
//   → NFR-002 (traces_to) - exact token prefix checks avoid shell-string false positives
// START_matches_prefix
fn matches_prefix(tokens: &[String], prefix: &[&str]) -> bool {
    tokens.len() >= prefix.len()
        && tokens
            .iter()
            .zip(prefix.iter())
            .all(|(token, expected)| token == expected)
}
// END_matches_prefix

// START_CONTRACT_route_git
// PURPOSE: Route git subcommands to a VCS adapter with stable keys
// INPUTS: { tokens: &[String] }
// OUTPUTS: { RouteDecision }
// LINKS:
//   → NFR-003 (traces_to) - VCS output is a high-volume token-saving target
// START_route_git
fn route_git(tokens: &[String]) -> RouteDecision {
    let subcommand = tokens.get(1).map(String::as_str).unwrap_or("");
    let key = match subcommand {
        "status" | "diff" | "log" | "show" | "branch" | "stash" | "grep" | "add" | "commit"
        | "checkout" | "switch" | "restore" => format!("git {}", subcommand),
        _ => "git".into(),
    };
    route("vcs-git", "vcs", &key, "git command")
}
// END_route_git

// START_CONTRACT_route_go
// PURPOSE: Route Go and golangci commands to stable adapter keys
// INPUTS: { tokens: &[String] }
// OUTPUTS: { RouteDecision }
// LINKS:
//   → NFR-003 (traces_to) - Go test/build output benefits from command-specific filtering
// START_route_go
fn route_go(tokens: &[String]) -> RouteDecision {
    if tokens[0] == "golangci-lint" {
        return route("go-tooling", "go", "golangci-lint", "golangci linter");
    }
    route_two_token("go-tooling", "go", tokens, "go command")
}
// END_route_go

// START_CONTRACT_route_uv
// PURPOSE: Route uv Python package-manager and pytest runner commands
// INPUTS: { tokens: &[String] }
// OUTPUTS: { RouteDecision }
// LINKS:
//   → NFR-003 (traces_to) - Python package and test output is a high-volume token-saving target
// START_route_uv
fn route_uv(tokens: &[String]) -> RouteDecision {
    if matches_prefix(tokens, &["uv", "run", "pytest"]) {
        return route(
            "python-pytest",
            "python",
            "uv run pytest",
            "uv pytest runner",
        );
    }
    route_two_token("python-tooling", "python", tokens, "uv tooling")
}
// END_route_uv

// START_CONTRACT_route_js
// PURPOSE: Route JavaScript package-manager and runner commands
// INPUTS: { tokens: &[String] }
// OUTPUTS: { RouteDecision }
// LINKS:
//   → NFR-003 (traces_to) - JS tool output is a high-volume token-saving target
// START_route_js
fn route_js(tokens: &[String]) -> RouteDecision {
    if matches!(tokens[0].as_str(), "npm" | "pnpm" | "yarn")
        && tokens.get(1).map(String::as_str) == Some("run")
    {
        let key = match tokens.get(2) {
            Some(script) if !script.starts_with('-') => {
                format!("{} run {}", tokens[0], script)
            }
            _ => format!("{} run", tokens[0]),
        };
        return route(
            "js-tooling",
            "javascript",
            &key,
            "javascript package script",
        );
    }
    route_two_token("js-tooling", "javascript", tokens, "javascript tooling")
}
// END_route_js

// START_CONTRACT_route_docker
// PURPOSE: Route Docker and Docker Compose commands to infrastructure adapter keys
// INPUTS: { tokens: &[String] }
// OUTPUTS: { RouteDecision }
// LINKS:
//   → NFR-003 (traces_to) - container listings and logs are high-volume shell output
// START_route_docker
fn route_docker(tokens: &[String]) -> RouteDecision {
    if matches_prefix(tokens, &["docker", "compose"]) {
        let subcommand = tokens.get(2).map(String::as_str).unwrap_or("");
        let key = if subcommand.is_empty() {
            "docker compose".into()
        } else {
            format!("docker compose {}", subcommand)
        };
        return route(
            "infra-cli",
            "infrastructure",
            &key,
            "docker compose command",
        );
    }
    route_two_token("infra-cli", "infrastructure", tokens, "docker command")
}
// END_route_docker

// START_CONTRACT_route_system
// PURPOSE: Route text-heavy system commands to stable adapter keys
// INPUTS: { tokens: &[String] }
// OUTPUTS: { RouteDecision }
// LINKS:
//   → NFR-003 (traces_to) - search/listing/log output can dominate LLM context
// START_route_system
fn route_system(tokens: &[String]) -> RouteDecision {
    let family = if matches!(tokens[0].as_str(), "rg" | "grep" | "find") {
        "search"
    } else if matches!(tokens[0].as_str(), "journalctl" | "systemctl") {
        "logs"
    } else {
        "system"
    };
    route("system-text", family, &tokens[0], "system text command")
}
// END_route_system

// START_CONTRACT_route_two_token
// PURPOSE: Build a route key from command plus first subcommand when present
// INPUTS: { adapter: &str }, { family: &str }, { tokens: &[String] }, { reason: &str }
// OUTPUTS: { RouteDecision }
// LINKS:
//   → NFR-003 (traces_to) - stable route keys group similar command economics
// START_route_two_token
fn route_two_token(adapter: &str, family: &str, tokens: &[String], reason: &str) -> RouteDecision {
    let key = match tokens.get(1) {
        Some(subcommand) if !subcommand.starts_with('-') => format!("{} {}", tokens[0], subcommand),
        _ => tokens[0].clone(),
    };
    route(adapter, family, &key, reason)
}
// END_route_two_token

// START_CONTRACT_route
// PURPOSE: Construct a proxied route decision
// INPUTS: { adapter: &str }, { family: &str }, { route_key: &str }, { reason: &str }
// OUTPUTS: { RouteDecision }
// LINKS:
//   → NFR-003 (traces_to) - proxied route decisions preserve adapter metadata
// START_route
fn route(adapter: &str, family: &str, route_key: &str, reason: &str) -> RouteDecision {
    RouteDecision {
        should_proxy: true,
        adapter: adapter.into(),
        family: family.into(),
        route_key: route_key.into(),
        filter_key: route_key.into(),
        reason: reason.into(),
    }
}
// END_route

// START_CONTRACT_passthrough
// PURPOSE: Construct a non-specialized passthrough route decision
// INPUTS: { reason: &str }
// OUTPUTS: { RouteDecision }
// LINKS:
//   → NFR-002 (traces_to) - unknown commands must remain executable without adapter failures
// START_passthrough
fn passthrough(reason: &str) -> RouteDecision {
    RouteDecision {
        should_proxy: false,
        adapter: "passthrough".into(),
        family: "unknown".into(),
        route_key: "passthrough".into(),
        filter_key: String::new(),
        reason: reason.into(),
    }
}
// END_passthrough

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn routes_rtk_like_command_families() {
        let router = CommandRouter::new();
        let samples = [
            (vec!["cargo", "test"], "rust-cargo", "cargo test"),
            (
                vec!["python", "-m", "pytest"],
                "python-pytest",
                "python -m pytest",
            ),
            (vec!["gh", "pr", "checks"], "vcs-hosting", "gh pr"),
            (
                vec!["docker", "compose", "logs"],
                "infra-cli",
                "docker compose logs",
            ),
            (vec!["terraform", "plan"], "infra-cli", "terraform plan"),
            (vec!["rg", "needle"], "system-text", "rg"),
        ];

        for (raw, adapter, key) in samples {
            let parts = raw
                .iter()
                .map(|value| value.to_string())
                .collect::<Vec<_>>();
            let decision = router.route(&parts);
            assert!(decision.should_proxy, "{raw:?} should proxy");
            assert_eq!(decision.adapter, adapter);
            assert_eq!(decision.route_key, key);
        }
    }

    #[test]
    fn avoids_recursive_proxying() {
        let router = CommandRouter::new();
        let parts = ["syn", "proxy", "--", "cargo", "test"]
            .iter()
            .map(|value| value.to_string())
            .collect::<Vec<_>>();
        let decision = router.route(&parts);

        assert!(!decision.should_proxy);
        assert_eq!(decision.adapter, "passthrough");
    }

    #[test]
    fn exposes_adapter_catalogue() {
        let adapters = CommandRouter::new().supported_adapters();

        assert!(adapters.iter().any(|item| item.adapter == "rust-cargo"));
        assert!(adapters.iter().any(|item| item.adapter == "python-pytest"));
        assert!(adapters.len() >= 8);
    }
}
