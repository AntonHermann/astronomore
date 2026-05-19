/// Loads raw bytes from a file path (native) or URL (WASM).
///
/// On native the path is resolved relative to the working directory.
/// On WASM it is used as a URL passed to the browser's Fetch API.
pub async fn load_bytes(path: &str) -> miette::Result<Vec<u8>> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        use miette::IntoDiagnostic;
        std::fs::read(path).into_diagnostic()
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
    let response: web_sys::Response = JsFuture::from(window.fetch_with_str(url))
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
