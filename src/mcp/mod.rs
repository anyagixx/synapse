// MODULE_CONTRACT
// MODULE_ID: M-MCP
// PURPOSE: MCP module declaration — exports lsp, persistent LSP manager, and server sub-modules
// SCOPE: Module declarations for MCP server facade, LSP manager, and private server helper modules
// DEPENDS: M-MCP-SERVER, M-MCP-SERVER-CASCADE-TOOLS, M-MCP-SERVER-CODE-TOOLS, M-MCP-SERVER-GRACE-TOOLS, M-MCP-SERVER-RESPONSE, M-MCP-SERVER-TOOLS, M-MCP-LSP
// LINKS: N/A

// START_MODULE_MAP
// lsp — LSP client bridge module
// lsp_manager — Persistent LSP process pool
// server — MCP JSON-RPC server module
// server_* — Private MCP server helper modules
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v2.24.0 - Added persistent LSP manager module]
// END_CHANGE_SUMMARY

// START_CONTRACT_public_api
// PURPOSE: Export MCP server and LSP bridge modules
// OUTPUTS: { lsp module }, { server module }
// START_public_api
mod server_cascade_tools;
mod server_code_tools;
mod server_contract_tools;
mod server_grace_tools;
mod server_response;
mod server_tools;

pub mod lsp;
pub mod lsp_manager;
pub mod server;
// END_public_api
