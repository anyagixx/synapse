// MODULE_CONTRACT
// MODULE_ID: M-AGENT-CONSOLE
// PURPOSE: Embedded tester-agent console facade with a deterministic tool registry for application-level GRACE testing
// SCOPE: AgentConsole registry, ToolHandler trait export, built-in test tool registration, tool lookup, and request routing
// DEPENDS: M-AGENT-CONSOLE-TOOLS
// LINKS:
//   -> V-M-AGENT-CONSOLE (verified_by) - console custom tool and built-in tool registration tests
//   -> M-GRACE-TESTING (uses) - test guide runner may use console URLs and tool semantics

// START_MODULE_MAP
// AgentConsole - Registry and dispatcher for tester-agent tools
// tools - Built-in console tools and ToolHandler trait
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v1.0.0 - Added embedded tester-agent console registry]
// END_CHANGE_SUMMARY

use std::collections::BTreeMap;

pub mod tools;

pub use tools::{BuiltinToolHandler, ToolHandler};

// START_public_api

// START_AgentConsole
pub struct AgentConsole {
    tools: BTreeMap<String, Box<dyn ToolHandler>>,
}
// END_AgentConsole

impl Default for AgentConsole {
    fn default() -> Self {
        Self::new()
    }
}

impl AgentConsole {
    // START_CONTRACT_AgentConsole::new
    // PURPOSE: Create an empty tester-agent console registry
    // OUTPUTS: { AgentConsole }
    // START_agent_console_new
    pub fn new() -> Self {
        Self {
            tools: BTreeMap::new(),
        }
    }
    // END_agent_console_new

    // START_CONTRACT_AgentConsole::with_builtin_tools
    // PURPOSE: Create a console with standard GRACE tester tools registered
    // OUTPUTS: { AgentConsole }
    // START_agent_console_with_builtin_tools
    pub fn with_builtin_tools() -> Self {
        let mut console = Self::new();
        for tool in tools::builtin_tools() {
            console.register_tool(tool);
        }
        console
    }
    // END_agent_console_with_builtin_tools

    // START_CONTRACT_AgentConsole::register_tool
    // PURPOSE: Register or replace one named tester-agent tool
    // INPUTS: { handler: Box<dyn ToolHandler> }
    // SIDE_EFFECTS: updates the in-memory tool registry
    // START_agent_console_register_tool
    pub fn register_tool(&mut self, handler: Box<dyn ToolHandler>) {
        self.tools.insert(handler.name().to_string(), handler);
    }
    // END_agent_console_register_tool

    // START_CONTRACT_AgentConsole::process_request
    // PURPOSE: Route a tester-agent tool request to the matching handler
    // INPUTS: { tool_name: &str }, { params: &str }
    // OUTPUTS: { Result<String, String> }
    // START_agent_console_process_request
    pub fn process_request(&self, tool_name: &str, params: &str) -> Result<String, String> {
        let handler = self
            .tools
            .get(tool_name)
            .ok_or_else(|| format!("Unknown agent console tool: {tool_name}"))?;
        handler.execute(params)
    }
    // END_agent_console_process_request

    // START_CONTRACT_AgentConsole::tool_names
    // PURPOSE: Return registered tool names in deterministic order
    // OUTPUTS: { Vec<String> }
    // START_agent_console_tool_names
    pub fn tool_names(&self) -> Vec<String> {
        self.tools.keys().cloned().collect()
    }
    // END_agent_console_tool_names
}

// END_public_api

#[cfg(test)]
mod tests {
    use super::*;

    struct EchoTool;

    impl ToolHandler for EchoTool {
        fn name(&self) -> &'static str {
            "echo"
        }

        fn description(&self) -> &'static str {
            "Echo params"
        }

        fn execute(&self, params: &str) -> Result<String, String> {
            Ok(format!("echo:{params}"))
        }
    }

    #[test]
    fn test_agent_console_registers_custom_tool() {
        let mut console = AgentConsole::new();
        console.register_tool(Box::new(EchoTool));
        assert_eq!(
            console.process_request("echo", "payload"),
            Ok("echo:payload".into())
        );
        assert_eq!(console.tool_names(), vec!["echo".to_string()]);
    }

    #[test]
    fn test_agent_console_registers_builtin_tools() {
        let console = AgentConsole::with_builtin_tools();
        assert!(console.tool_names().contains(&"get_logs".to_string()));
        assert!(console.process_request("get_system_state", "{}").is_ok());
    }
}
