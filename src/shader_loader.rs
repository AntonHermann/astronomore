/// Loads, parses, and validates WGSL shaders with miette-formatted error messages.
///
/// On failure, errors include source-highlighted spans pointing to the exact
/// location of the syntax or validation problem — similar to Rust compiler output.
use miette::{LabeledSpan, NamedSource, SourceSpan};

/// A WGSL compilation or validation error with source-span information for miette.
pub struct WgslError {
    message: String,
    src: NamedSource<String>,
    labels: Vec<LabeledSpan>,
}

impl std::fmt::Debug for WgslError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WgslError")
            .field("message", &self.message)
            .finish()
    }
}

impl std::fmt::Display for WgslError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for WgslError {}

impl miette::Diagnostic for WgslError {
    fn source_code(&self) -> Option<&dyn miette::SourceCode> {
        Some(&self.src)
    }

    fn labels(&self) -> Option<Box<dyn Iterator<Item = LabeledSpan> + '_>> {
        Some(Box::new(self.labels.iter().cloned()))
    }
}

/// Parses and validates WGSL source with naga, returning `()` on success or a
/// `WgslError` with source-highlighted spans on failure.
pub fn validate_wgsl(filename: &str, source: &str) -> miette::Result<()> {
    let module = naga::front::wgsl::parse_str(source)
        .map_err(|e| parse_error_to_diagnostic(filename, source.to_owned(), &e))?;

    naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::all(),
    )
    .validate(&module)
    .map_err(|e| validation_error_to_diagnostic(filename, source.to_owned(), &e))?;

    Ok(())
}

/// Creates a `wgpu::ShaderModule` from validated WGSL source.
///
/// Call [`validate_wgsl`] first to get a miette-formatted error on failure;
/// wgpu's own error reporting is far less readable.
pub fn make_shader_module(device: &wgpu::Device, label: &str, source: &str) -> wgpu::ShaderModule {
    device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some(label),
        source: wgpu::ShaderSource::Wgsl(source.into()),
    })
}

fn span_to_miette(span: naga::Span) -> SourceSpan {
    span.to_range()
        .map(|r| SourceSpan::new(r.start.into(), r.len()))
        .unwrap_or_else(|| SourceSpan::new(0.into(), 0))
}

fn parse_error_to_diagnostic(
    filename: &str,
    source: String,
    e: &naga::front::wgsl::ParseError,
) -> WgslError {
    let labels = e
        .labels()
        .map(|(span, msg)| LabeledSpan::new_with_span(Some(msg.to_owned()), span_to_miette(span)))
        .collect();
    WgslError {
        message: e.message().to_owned(),
        src: NamedSource::new(filename, source),
        labels,
    }
}

fn validation_error_to_diagnostic(
    filename: &str,
    source: String,
    e: &naga::WithSpan<naga::valid::ValidationError>,
) -> WgslError {
    let labels = e
        .spans()
        .map(|(span, msg)| LabeledSpan::new_with_span(Some(msg.clone()), span_to_miette(*span)))
        .collect();
    WgslError {
        message: e.to_string(),
        src: NamedSource::new(filename, source),
        labels,
    }
}
