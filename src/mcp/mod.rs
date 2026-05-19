// MODULE_CONTRACT
// MODULE_ID: M-MCP
// PURPOSE: MCP module declaration — exports lsp and server sub-modules
// SCOPE: Module declarations for MCP server facade and private server helper modules
// DEPENDS: M-MCP-SERVER, M-MCP-SERVER-CODE-TOOLS, M-MCP-SERVER-GRACE-TOOLS, M-MCP-SERVER-RESPONSE, M-MCP-SERVER-TOOLS, M-MCP-LSP
// LINKS: N/A

// START_MODULE_MAP
// lsp — LSP client bridge module
// server — MCP JSON-RPC server module
// server_* — Private MCP server helper modules
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v2.7.0 — Added private MCP server helper modules]
// END_CHANGE_SUMMARY

// START_CONTRACT_public_api
// PURPOSE: Export MCP server and LSP bridge modules
// OUTPUTS: { lsp module }, { server module }
// START_public_api
mod server_code_tools;
mod server_grace_tools;
mod server_response;
mod server_tools;

pub mod lsp;
pub mod server;
// END_public_api
