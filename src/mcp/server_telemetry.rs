// MODULE_CONTRACT
// MODULE_ID: M-MCP-SERVER-TELEMETRY
// PURPOSE: Build bounded OpenTelemetry span metadata for MCP tools/list and tools/call paths.
// SCOPE: MCP tools/list span creation, tools/call span creation, budget/context/status/duration field recording.
// DEPENDS: M-MCP-SERVER, M-TRACKING
// LINKS:
//   -> docs/phases/Phase-93.xml (implements) - MCP telemetry hot-path spans
//   -> M-TELEMETRY (depends) - spans are exported by telemetry initialization

// START_MODULE_MAP
// tools_list_span - Create a bounded tools/list span
// tools_call_span - Create a bounded tools/call span
// record_budget - Record budget level metadata on a tools/call span
// record_call_result - Record result metadata on a tools/call span
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v1.0.0 - Added Phase-93 MCP span helpers]
// END_CHANGE_SUMMARY

use crate::tracking::{mcp_metrics::McpCallStatus, BudgetStatus};

// START_public_api

// START_CONTRACT_tools_list_span
// PURPOSE: Create a short-lived span for a tools/list response with bounded disclosure metadata.
// INPUTS: { profile: &str }, { style: &str }, { total_available: usize }, { total_visible: usize }
// OUTPUTS: { tracing::Span }
// START_tools_list_span
pub(crate) fn tools_list_span(
    profile: &str,
    style: &str,
    total_available: usize,
    total_visible: usize,
) -> tracing::Span {
    tracing::info_span!(
        "mcp.tools_list",
        profile,
        style,
        total_available,
        total_visible
    )
}
// END_tools_list_span

// START_CONTRACT_tools_call_span
// PURPOSE: Create a tools/call span without recording raw arguments.
// INPUTS: { tool: &str }
// OUTPUTS: { tracing::Span }
// START_tools_call_span
pub(crate) fn tools_call_span(tool: &str) -> tracing::Span {
    tracing::info_span!(
        "mcp.tools_call",
        tool,
        budget = tracing::field::Empty,
        context_pressure = tracing::field::Empty,
        status = tracing::field::Empty,
        duration_ms = tracing::field::Empty
    )
}
// END_tools_call_span

// START_CONTRACT_record_budget
// PURPOSE: Record budget gate level when a tools/call path is budget-aware.
// INPUTS: { span: &tracing::Span }, { status: Option<&BudgetStatus> }
// OUTPUTS: { () }
// START_record_budget
pub(crate) fn record_budget(span: &tracing::Span, status: Option<&BudgetStatus>) {
    if let Some(status) = status {
        span.record("budget", tracing::field::debug(&status.status));
    }
}
// END_record_budget

// START_CONTRACT_record_call_result
// PURPOSE: Record result status, duration, and response-attached context pressure level.
// INPUTS: { span: &tracing::Span }, { response: &serde_json::Value }, { status: &McpCallStatus }, { duration_ms: u64 }
// OUTPUTS: { () }
// START_record_call_result
pub(crate) fn record_call_result(
    span: &tracing::Span,
    response: &serde_json::Value,
    status: &McpCallStatus,
    duration_ms: u64,
) {
    span.record(
        "context_pressure",
        response["result"]["_context_pressure"]["level"]
            .as_str()
            .unwrap_or("low"),
    )
    .record("status", tracing::field::debug(status))
    .record("duration_ms", duration_ms);
}
// END_record_call_result

// END_public_api
