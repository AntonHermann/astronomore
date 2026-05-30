use miette::IntoDiagnostic;

/// Loads a UTF-8 text file from a path (native) or URL (WASM).
#[tracing::instrument(err)]
pub async fn load_str(path: &str) -> miette::Result<String> {
    let bytes = load_bytes(path).await?;
    String::from_utf8(bytes).into_diagnostic()
}

/// Loads raw bytes from a file path (native) or URL (WASM).
///
/// On native the path is resolved relative to the working directory.
/// On WASM it is used as a URL passed to the browser's Fetch API.
#[tracing::instrument(err)]
pub async fn load_bytes(path: &str) -> miette::Result<Vec<u8>> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::fs::read(path)
            .into_diagnostic()
            .inspect(|v| tracing::debug!(len = v.len(), "asset loaded"))
    }
    #[cfg(target_arch = "wasm32")]
    {
        fetch_bytes(path).await
    }
}

#[cfg(target_arch = "wasm32")]
async fn fetch_bytes(url: &str) -> miette::Result<Vec<u8>> {
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;

    let window = web_sys::window().ok_or_else(|| miette::miette!("No window object"))?;

    // Always revalidate with the server so stale cached shader files are never
    // used after a deployment. "no-cache" sends a conditional request (ETag /
    // Last-Modified); the server returns 304 if unchanged, otherwise the fresh
    // body — without forcing a full download every time.
    let opts = web_sys::RequestInit::new();
    opts.set_cache(web_sys::RequestCache::NoCache);
    let request = web_sys::Request::new_with_str_and_init(url, &opts)
        .map_err(|e| miette::miette!("Request creation failed: {:?}", e))?;

    let response: web_sys::Response = JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|e| miette::miette!("fetch failed: {:?}", e))?
        .dyn_into()
        .map_err(|_| miette::miette!("Response cast failed"))?;

    if !response.ok() {
        return Err(miette::miette!(
            "HTTP {} while fetching {}",
            response.status(),
            url
        ));
    }

    let array_buffer = JsFuture::from(
        response
            .array_buffer()
            .map_err(|e| miette::miette!("array_buffer() failed: {:?}", e))?,
    )
    .await
    .map_err(|e| miette::miette!("ArrayBuffer await failed: {:?}", e))?;

    Ok(js_sys::Uint8Array::new(&array_buffer).to_vec())
}
