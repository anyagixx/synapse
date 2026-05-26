// MODULE_CONTRACT
// MODULE_ID: M-MCP
// PURPOSE: MCP module declaration — exports lsp, persistent LSP manager, pipeline, server sub-modules, telemetry helpers, and user tool support
// SCOPE: Module declarations for MCP server facade, MCP stdio pipeline, LSP manager, private server helper modules including budget, pressure, telemetry, and tool recommendation, plus local user-defined MCP tools
// DEPENDS: M-MCP-PIPELINE, M-MCP-SERVER, M-MCP-SERVER-CASCADE-TOOLS, M-MCP-SERVER-CODE-TOOLS, M-MCP-SERVER-GRACE-TOOLS, M-MCP-SERVER-RUN-TOOLS, M-MCP-SERVER-RESPONSE, M-MCP-SERVER-TELEMETRY, M-MCP-SERVER-TOOLS, M-MCP-USER-TOOLS, M-MCP-LSP
// LINKS: N/A

// START_MODULE_MAP
// lsp — LSP client bridge module
// lsp_manager — Persistent LSP process pool
// pipeline — Bounded MCP stdio read/handler/write pipeline
// server — MCP JSON-RPC server module
// server_* — Private MCP server helper modules
// user_tools — Local user-defined MCP tool loader and executor
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v2.31.0 - Added MCP server telemetry helper module]
// END_CHANGE_SUMMARY

// START_CONTRACT_public_api
// PURPOSE: Export MCP server and LSP bridge modules
// OUTPUTS: { lsp module }, { server module }
// START_public_api
mod server_budget_tools;
mod server_cascade_tools;
mod server_code_tools;
mod server_contract_tools;
mod server_grace_tools;
mod server_pressure;
mod server_response;
mod server_run_tools;
mod server_telemetry;
mod server_tools;
mod server_tools_pressure;
mod tool_recommend;
pub(crate) mod user_tools;

pub mod lsp;
pub mod lsp_manager;
pub(crate) mod pipeline;
pub mod server;
// END_public_api
