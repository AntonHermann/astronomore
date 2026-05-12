use tracing::level_filters::LevelFilter;
use tracing_subscriber::EnvFilter;

fn main() -> miette::Result<()> {
    let env_filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::INFO.into())
        .from_env_lossy();
    tracing_subscriber::fmt().with_env_filter(env_filter).init();

    astronomore::run()
}
