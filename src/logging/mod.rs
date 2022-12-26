use std::env;

use opentelemetry::{
    runtime::Tokio,
    sdk::{
        trace::{self, Tracer},
        Resource,
    },
    KeyValue,
};
use opentelemetry_otlp::WithExportConfig;
use tracing::metadata::LevelFilter;
use tracing_subscriber::{prelude::__tracing_subscriber_SubscriberExt, Registry};

fn get_honeycomb_tracer() -> Tracer {
    let mut map = tonic::metadata::MetadataMap::with_capacity(2);

    map.insert(
        "x-honeycomb-team",
        env::var("HONEYCOMB_API_KEY").unwrap().parse().unwrap(),
    );
    map.insert(
        "x-honeycomb-dataset",
        env::var("HONEYCOMB_DATASET").unwrap().parse().unwrap(),
    );
    let otlp_exporter = opentelemetry_otlp::new_exporter()
        .tonic()
        .with_endpoint("https://api.honeycomb.io")
        .with_metadata(map);
    opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(otlp_exporter)
        .with_trace_config(
            trace::config().with_resource(Resource::new(vec![KeyValue::new(
                opentelemetry_semantic_conventions::resource::SERVICE_NAME,
                "cereal_rewrite".to_string(),
            )])),
        )
        .install_batch(Tokio)
        .unwrap()
}

pub fn configure_tracing() {
    let subscriber = Registry::default() // provide underlying span data store
        .with(LevelFilter::INFO) // filter out low-level debug tracing (eg tokio executor)
        .with(tracing_opentelemetry::layer().with_tracer(get_honeycomb_tracer())) // publish to honeycomb backend
        .with(tracing_subscriber::fmt::Layer::new());
    tracing::subscriber::set_global_default(subscriber).unwrap();
}
