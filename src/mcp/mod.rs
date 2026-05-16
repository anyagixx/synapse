// MODULE_CONTRACT
// MODULE_ID: M-MCP
// PURPOSE: MCP module declaration — exports lsp and server sub-modules
// SCOPE: Module declarations only (lsp, server)
// DEPENDS: M-MCP-SERVER, M-MCP-LSP
// LINKS: N/A

// START_MODULE_MAP
// lsp — LSP client bridge module
// server — MCP JSON-RPC server module
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v2.0.0 — GRACE markup added]
// END_CHANGE_SUMMARY

// START_public_api
pub mod lsp;
pub mod server;
// END_public_api
