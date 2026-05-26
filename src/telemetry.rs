// MODULE_CONTRACT
// MODULE_ID: M-TELEMETRY
// PURPOSE: Optional OpenTelemetry initialization for Synapse tracing.
// SCOPE: stderr JSON tracing setup, default-off OTLP export, sampling parsing, resource attribute mapping, and exporter shutdown guard.
// DEPENDS: M-CONFIG
// LINKS:
//   -> docs/phases/Phase-93.xml (implements) - OpenTelemetry OTLP export
//   -> NFR-002 (traces_to) - telemetry remains optional and default-off

// START_MODULE_MAP
// TelemetryGuard - Shutdown handle for optional OTLP exporter
// SamplingMode - Supported sampler modes
// init_stderr_tracing - Initialize default JSON stderr tracing
// init_telemetry - Initialize tracing subscriber and optional OTLP layer
// sampling_mode - Parse telemetry.sampling config
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v1.0.0 - Added Phase-93 OpenTelemetry initialization]
// END_CHANGE_SUMMARY

use crate::config::{Config, TelemetryConfig};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

// START_public_api

// START_TelemetryGuard
pub struct TelemetryGuard {
    #[cfg(feature = "telemetry")]
    provider: Option<opentelemetry_sdk::trace::TracerProvider>,
}
// END_TelemetryGuard

impl TelemetryGuard {
    // START_CONTRACT_TelemetryGuard::disabled
    // PURPOSE: Return a no-op telemetry guard.
    // OUTPUTS: { TelemetryGuard }
    // START_telemetry_guard_disabled
    fn disabled() -> Self {
        Self {
            #[cfg(feature = "telemetry")]
            provider: None,
        }
    }
    // END_telemetry_guard_disabled

    // START_CONTRACT_TelemetryGuard::shutdown
    // PURPOSE: Flush and shut down the OTLP tracer provider when telemetry is enabled.
    // OUTPUTS: { anyhow::Result<()> }
    // SIDE_EFFECTS: may flush exporter batches
    // START_telemetry_guard_shutdown
    pub fn shutdown(self) -> anyhow::Result<()> {
        #[cfg(feature = "telemetry")]
        if let Some(provider) = self.provider {
            provider
                .shutdown()
                .map_err(|error| anyhow::anyhow!("telemetry shutdown failed: {error:?}"))?;
        }
        Ok(())
    }
    // END_telemetry_guard_shutdown
}

// START_SamplingMode
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SamplingMode {
    AlwaysOn,
    AlwaysOff,
    Ratio(f64),
}
// END_SamplingMode

// START_CONTRACT_init_telemetry
// PURPOSE: Initialize stderr JSON tracing and optional OTLP export from runtime config.
// INPUTS: { config: &Config }
// OUTPUTS: { anyhow::Result<TelemetryGuard> }
// SIDE_EFFECTS: installs the global tracing subscriber
// START_init_telemetry
pub fn init_telemetry(config: &Config) -> anyhow::Result<TelemetryGuard> {
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));

    if !config.telemetry.enabled {
        init_stderr_tracing(env_filter)?;
        return Ok(TelemetryGuard::disabled());
    }

    init_enabled_telemetry(&config.telemetry, env_filter)
}
// END_init_telemetry

// START_CONTRACT_sampling_mode
// PURPOSE: Parse telemetry.sampling into a bounded sampler mode.
// INPUTS: { value: &str }
// OUTPUTS: { anyhow::Result<SamplingMode> }
// START_sampling_mode
pub fn sampling_mode(value: &str) -> anyhow::Result<SamplingMode> {
    let trimmed = value.trim().to_ascii_lowercase();
    match trimmed.as_str() {
        "always_on" | "always-on" | "on" => Ok(SamplingMode::AlwaysOn),
        "always_off" | "always-off" | "off" => Ok(SamplingMode::AlwaysOff),
        value if value.starts_with("ratio:") => {
            let ratio = value["ratio:".len()..]
                .parse::<f64>()
                .map_err(|_| anyhow::anyhow!("telemetry.sampling ratio must be numeric"))?;
            if !(0.0..=1.0).contains(&ratio) {
                anyhow::bail!("telemetry.sampling ratio must be between 0.0 and 1.0");
            }
            Ok(SamplingMode::Ratio(ratio))
        }
        _ => anyhow::bail!("telemetry.sampling must be always_on, always_off, or ratio:<0..1>"),
    }
}
// END_sampling_mode

// END_public_api

// START_CONTRACT_init_stderr_tracing
// PURPOSE: Initialize default JSON stderr tracing without OTLP export.
// INPUTS: { env_filter: EnvFilter }
// OUTPUTS: { anyhow::Result<()> }
// SIDE_EFFECTS: installs tracing subscriber
// START_init_stderr_tracing
fn init_stderr_tracing(env_filter: tracing_subscriber::EnvFilter) -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(env_filter)
        .with(
            tracing_subscriber::fmt::layer()
                .json()
                .with_target(true)
                .with_writer(std::io::stderr),
        )
        .try_init()
        .map_err(|error| anyhow::anyhow!("tracing initialization failed: {error}"))
}
// END_init_stderr_tracing

#[cfg(feature = "telemetry")]
// START_CONTRACT_init_enabled_telemetry_otlp
// PURPOSE: Initialize OTLP tracing when the telemetry feature is compiled.
// INPUTS: { config: &TelemetryConfig }, { env_filter: EnvFilter }
// OUTPUTS: { anyhow::Result<TelemetryGuard> }
// SIDE_EFFECTS: installs tracing subscriber and configures global tracer provider
// START_init_enabled_telemetry_otlp
fn init_enabled_telemetry(
    config: &TelemetryConfig,
    env_filter: tracing_subscriber::EnvFilter,
) -> anyhow::Result<TelemetryGuard> {
    use opentelemetry::trace::TracerProvider as _;
    use opentelemetry::KeyValue;
    use opentelemetry_otlp::WithExportConfig;
    use opentelemetry_sdk::trace::{Sampler, TracerProvider as SdkTracerProvider};

    let sampler = match sampling_mode(&config.sampling)? {
        SamplingMode::AlwaysOn => Sampler::AlwaysOn,
        SamplingMode::AlwaysOff => Sampler::AlwaysOff,
        SamplingMode::Ratio(ratio) => Sampler::TraceIdRatioBased(ratio),
    };
    let mut attrs = vec![KeyValue::new("service.name", config.service_name.clone())];
    attrs.extend(
        config
            .resource
            .iter()
            .map(|(key, value)| KeyValue::new(key.clone(), value.clone())),
    );
    let exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .with_endpoint(config.endpoint.clone())
        .build()?;
    let provider = SdkTracerProvider::builder()
        .with_sampler(sampler)
        .with_resource(opentelemetry_sdk::Resource::new(attrs))
        .with_batch_exporter(exporter, opentelemetry_sdk::runtime::Tokio)
        .build();
    let tracer = provider.tracer(config.service_name.clone());
    let telemetry_layer = tracing_opentelemetry::layer().with_tracer(tracer);
    tracing_subscriber::registry()
        .with(env_filter)
        .with(
            tracing_subscriber::fmt::layer()
                .json()
                .with_target(true)
                .with_writer(std::io::stderr),
        )
        .with(telemetry_layer)
        .try_init()
        .map_err(|error| anyhow::anyhow!("tracing initialization failed: {error}"))?;
    opentelemetry::global::set_tracer_provider(provider.clone());
    tracing::info!(
        service = config.service_name,
        endpoint = config.endpoint,
        batch_interval_ms = config.batch_interval_ms,
        "OpenTelemetry initialized"
    );
    Ok(TelemetryGuard {
        provider: Some(provider),
    })
}
// END_init_enabled_telemetry_otlp

#[cfg(not(feature = "telemetry"))]
// START_CONTRACT_init_enabled_telemetry_stderr_fallback
// PURPOSE: Fall back to stderr tracing when config enables telemetry but the feature is not compiled.
// INPUTS: { config: &TelemetryConfig }, { env_filter: EnvFilter }
// OUTPUTS: { anyhow::Result<TelemetryGuard> }
// SIDE_EFFECTS: installs tracing subscriber
// START_init_enabled_telemetry_stderr_fallback
fn init_enabled_telemetry(
    config: &TelemetryConfig,
    env_filter: tracing_subscriber::EnvFilter,
) -> anyhow::Result<TelemetryGuard> {
    init_stderr_tracing(env_filter)?;
    tracing::warn!(
        endpoint = config.endpoint,
        "telemetry.enabled=true but binary was built without telemetry feature"
    );
    Ok(TelemetryGuard::disabled())
}
// END_init_enabled_telemetry_stderr_fallback

#[cfg(test)]
mod tests {
    use super::*;

    // START_CONTRACT_sampling_mode_parses_supported_values
    // PURPOSE: Verify sampler parsing accepts supported values and rejects invalid ratios.
    // START_sampling_mode_parses_supported_values
    #[test]
    fn sampling_mode_parses_supported_values() {
        assert_eq!(sampling_mode("always_on").unwrap(), SamplingMode::AlwaysOn);
        assert_eq!(sampling_mode("off").unwrap(), SamplingMode::AlwaysOff);
        assert_eq!(
            sampling_mode("ratio:0.25").unwrap(),
            SamplingMode::Ratio(0.25)
        );
        assert!(sampling_mode("ratio:2.0").is_err());
    }
    // END_sampling_mode_parses_supported_values
}
