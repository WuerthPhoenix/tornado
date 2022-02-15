use crate::elastic_apm::{get_current_service_name, ApmTracingConfig};
use crate::LoggerError;
use opentelemetry::propagation::{Extractor, Injector, TextMapPropagator};
use opentelemetry::sdk::propagation::TraceContextPropagator;
use opentelemetry::sdk::trace::{config, SamplingDecision, SamplingResult, ShouldSample, Tracer};
use opentelemetry::sdk::Resource;
use opentelemetry::trace::{Link, SpanKind, TraceContextExt, TraceId, TraceState};
use opentelemetry::{Context, KeyValue};
use opentelemetry_otlp::{ExportConfig, Protocol, WithExportConfig};
use opentelemetry_semantic_conventions as otel_sem_cov;
use serde_json::{Map, Value};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tonic::metadata::MetadataMap;

// This sampler is needed to allow to always construct the OpenTelemetry context
// even in the case that we do not want to export the traces to APM.
// Having a sampler based on the setting the "apm_output" atomic bool allows us to not export
// the traces to APM while still constructing the trace context which is needed to always
// generate and manage the trace_id of the Events
#[derive(Debug)]
struct TornadoSampler {
    pub should_sample: Arc<AtomicBool>,
}

impl TornadoSampler {
    pub fn new(should_sample: Arc<AtomicBool>) -> Self {
        Self { should_sample }
    }
}
impl ShouldSample for TornadoSampler {
    fn should_sample(
        &self,
        parent_context: Option<&Context>,
        _trace_id: TraceId,
        _name: &str,
        _span_kind: &SpanKind,
        _attributes: &[KeyValue],
        _links: &[Link],
    ) -> SamplingResult {
        let decision = if self.should_sample.load(Ordering::Relaxed) {
            SamplingDecision::RecordAndSample
        } else {
            SamplingDecision::Drop
        };
        // This logic is taken from https://github.com/open-telemetry/opentelemetry-rust/blob/main/opentelemetry/src/sdk/trace/sampler.rs
        SamplingResult {
            decision,
            // No extra attributes ever set by the SDK samplers.
            attributes: Vec::new(),
            // all sampler in SDK will not modify trace state.
            trace_state: match parent_context {
                Some(ctx) => ctx.span().span_context().trace_state().clone(),
                None => TraceState::default(),
            },
        }
    }
}

pub fn get_opentelemetry_tracer(
    apm_tracing_config: &ApmTracingConfig,
    apm_output_enabled: Arc<AtomicBool>,
) -> Result<Tracer, LoggerError> {
    let mut tonic_metadata = MetadataMap::new();
    if let Some(apm_server_api_credentials) = &apm_tracing_config.apm_server_api_credentials {
        tonic_metadata.insert(
            "authorization",
            apm_server_api_credentials.to_authorization_header_value().parse()
                .map_err(|err| LoggerError::LoggerRuntimeError {
                    message: format!("Logger - Error while constructing the authorization header for tonic client. Error: {}", err)
                })?,
        );
    };

    let export_config = ExportConfig {
        endpoint: apm_tracing_config.apm_server_url.clone(),
        protocol: Protocol::Grpc,
        timeout: Duration::from_secs(10),
    };

    let tornado_sampler = TornadoSampler::new(apm_output_enabled);
    let hostname = sys_info::hostname().unwrap_or("localhost".to_owned());
    opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(
            opentelemetry_otlp::new_exporter()
                .tonic()
                .with_export_config(export_config)
                .with_metadata(tonic_metadata),
        )
        .with_trace_config(config().with_sampler(tornado_sampler).with_resource(Resource::new(
            vec![
                otel_sem_cov::resource::SERVICE_NAME.string(get_current_service_name()?),
                otel_sem_cov::resource::HOST_NAME.string(hostname.clone()),
                otel_sem_cov::resource::SERVICE_INSTANCE_ID.string(hostname),
            ],
        )))
        .install_batch(opentelemetry::runtime::Tokio)
        .map_err(|err| LoggerError::LoggerRuntimeError {
            message: format!(
                "Logger - Error while installing the OpenTelemetry Tracer. Error: {:?}",
                err
            ),
        })
}

pub struct TelemetryContextInjector(pub Map<String, Value>);
pub struct TelemetryContextExtractor<'a>(pub &'a Map<String, Value>);

impl Injector for TelemetryContextInjector {
    fn set(&mut self, key: &str, value: String) {
        self.0.insert(key.to_owned(), Value::String(value));
    }
}

impl TelemetryContextInjector {
    pub fn get_trace_context_map(
        trace_context: &Context,
        trace_context_propagator: &TraceContextPropagator,
    ) -> Map<String, Value> {
        let map = serde_json::Map::new();
        let mut injector = TelemetryContextInjector(map);
        trace_context_propagator.inject_context(trace_context, &mut injector);
        injector.0
    }
}

impl Extractor for TelemetryContextExtractor<'_> {
    fn get(&self, key: &str) -> Option<&str> {
        self.0.get(key).and_then(|val| val.as_str())
    }

    fn keys(&self) -> Vec<&str> {
        self.0.keys().map(String::as_str).collect()
    }
}

impl TelemetryContextExtractor<'_> {
    pub fn get_trace_context(
        trace_context: &Map<String, Value>,
        trace_context_propagator: &TraceContextPropagator,
    ) -> Context {
        trace_context_propagator.extract(&TelemetryContextExtractor(trace_context))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::elastic_apm::{ApmServerApiCredentials, ApmTracingConfig};

    #[tokio::test]
    async fn should_get_opentelemetry_tracer() {
        let tracing_config = ApmTracingConfig {
            apm_output: true,
            apm_server_url: "apm.example.com".to_string(),
            apm_server_api_credentials: Some(ApmServerApiCredentials {
                id: "myid".to_string(),
                key: "mykey".to_string(),
            }),
        };
        let tracer = get_opentelemetry_tracer(&tracing_config, Arc::new(AtomicBool::new(true)));
        assert!(tracer.is_ok());
    }
}
