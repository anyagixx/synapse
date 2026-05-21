// MODULE_CONTRACT
// MODULE_ID: M-AGENT-CONSOLE-TOOLS
// PURPOSE: Built-in embedded tester-agent console tools for logs, actions, state, read-only database queries, and endpoint calls
// SCOPE: ToolHandler trait, BuiltinToolHandler enum, built-in tool factory, deterministic placeholder execution semantics
// DEPENDS: N/A
// LINKS:
//   -> V-M-AGENT-CONSOLE (verified_by) - built-in tool registration and custom tool tests

// START_MODULE_MAP
// ToolHandler - Trait implemented by tester-agent console tools
// BuiltinToolHandler - Standard GRACE test console tools
// builtin_tools - Factory for standard tools
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v1.0.0 - Added built-in tester-agent console tools]
// END_CHANGE_SUMMARY

// START_public_api

// START_ToolHandler
pub trait ToolHandler: Send + Sync {
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;
    fn execute(&self, params: &str) -> Result<String, String>;
}
// END_ToolHandler

// START_BuiltinToolHandler
#[derive(Debug, Clone, Copy)]
pub enum BuiltinToolHandler {
    GetLogs,
    ExecuteAction,
    GetSystemState,
    QueryDatabase,
    CallEndpoint,
}
// END_BuiltinToolHandler

impl ToolHandler for BuiltinToolHandler {
    // START_CONTRACT_BuiltinToolHandler::name
    // PURPOSE: Return stable MCP-style tool name for the built-in console tool
    // OUTPUTS: { &'static str }
    // START_builtin_tool_name
    fn name(&self) -> &'static str {
        match self {
            Self::GetLogs => "get_logs",
            Self::ExecuteAction => "execute_action",
            Self::GetSystemState => "get_system_state",
            Self::QueryDatabase => "query_database",
            Self::CallEndpoint => "call_endpoint",
        }
    }
    // END_builtin_tool_name

    // START_CONTRACT_BuiltinToolHandler::description
    // PURPOSE: Return tester-facing description for the built-in console tool
    // OUTPUTS: { &'static str }
    // START_builtin_tool_description
    fn description(&self) -> &'static str {
        match self {
            Self::GetLogs => {
                "Return structured LOG entries matching module, level, ref, and time filters."
            }
            Self::ExecuteAction => {
                "Execute one application action exposed for tester-agent workflows."
            }
            Self::GetSystemState => "Return current high-level application state snapshot.",
            Self::QueryDatabase => "Execute read-only diagnostic database queries.",
            Self::CallEndpoint => "Call an application HTTP endpoint through the test console.",
        }
    }
    // END_builtin_tool_description

    // START_CONTRACT_BuiltinToolHandler::execute
    // PURPOSE: Execute deterministic placeholder behavior for built-in console tools
    // INPUTS: { params: &str }
    // OUTPUTS: { Result<String, String> }
    // START_builtin_tool_execute
    fn execute(&self, params: &str) -> Result<String, String> {
        let escaped = params
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;");
        Ok(format!(
            "<AgentConsoleTool name=\"{}\" status=\"available\"><Description>{}</Description><Params>{}</Params></AgentConsoleTool>",
            self.name(),
            self.description(),
            escaped
        ))
    }
    // END_builtin_tool_execute
}

// START_CONTRACT_builtin_tools
// PURPOSE: Return all standard tester-agent console tools
// OUTPUTS: { Vec<Box<dyn ToolHandler>> }
// START_builtin_tools
pub fn builtin_tools() -> Vec<Box<dyn ToolHandler>> {
    vec![
        Box::new(BuiltinToolHandler::GetLogs),
        Box::new(BuiltinToolHandler::ExecuteAction),
        Box::new(BuiltinToolHandler::GetSystemState),
        Box::new(BuiltinToolHandler::QueryDatabase),
        Box::new(BuiltinToolHandler::CallEndpoint),
    ]
}
// END_builtin_tools

// END_public_api
