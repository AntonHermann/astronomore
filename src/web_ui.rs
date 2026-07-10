//! Web-only HTML overlay controls layered over the wgpu canvas.
//!
//! The egui date fields cannot open a mobile virtual keyboard (winit's
//! `set_ime_allowed` is a no-op on wasm), and egui widgets scale with
//! `pixels_per_point`, so an extreme UI scale can leave the scale slider
//! itself unreachable. Both problems disappear once the controls live in the
//! DOM instead of inside the canvas: a native `<input type="date">` opens the
//! platform date picker/keyboard, and a toggleable `<input type="range">`
//! keeps a fixed, reachable size no matter how egui is scaled — the escape
//! hatch a scaled egui slider can't offer.
//!
//! The elements are declared statically in `index.html`; this module wires
//! their events into a shared [`WebUiHandle`] that [`crate::State`] drains
//! once per frame.

use std::cell::RefCell;
use std::rc::Rc;

use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;

/// Edits produced by the HTML controls since the last drain.
#[derive(Default)]
pub struct WebUiEvents {
    /// A `(year, month, day)` picked in the date input.
    pub date: Option<(i32, u8, u8)>,
    /// A UI scale chosen with the range slider.
    pub ui_scale: Option<f32>,
}

/// Shared handle to the pending HTML-control edits.
pub type WebUiHandle = Rc<RefCell<WebUiEvents>>;

const DATE_ID: &str = "date-input";
const SCALE_ID: &str = "scale-input";
const SCALE_VALUE_ID: &str = "scale-value";
const SCALE_TOGGLE_ID: &str = "scale-toggle";
const SCALE_PANEL_ID: &str = "scale-panel";

/// Wire up the HTML overlay controls and return a handle the render loop drains.
///
/// `initial_date` seeds the date picker; `initial_scale` seeds the slider and
/// its live readout. Missing elements are skipped so a stripped-down
/// `index.html` degrades gracefully instead of panicking.
pub fn setup(initial_date: (i32, u8, u8), initial_scale: f32) -> WebUiHandle {
    let handle: WebUiHandle = Rc::new(RefCell::new(WebUiEvents::default()));
    let Some(document) = web_sys::window().and_then(|w| w.document()) else {
        return handle;
    };

    // --- Date picker: <input type="date"> emits "YYYY-MM-DD" on `change`. ---
    if let Some(input) = element::<web_sys::HtmlInputElement>(&document, DATE_ID) {
        let (y, m, d) = initial_date;
        input.set_value(&format!("{y:04}-{m:02}-{d:02}"));

        let handle_cb = handle.clone();
        let input_cb = input.clone();
        let closure = Closure::<dyn FnMut(web_sys::Event)>::new(move |_| {
            if let Some(date) = parse_date(&input_cb.value()) {
                handle_cb.borrow_mut().date = Some(date);
            }
        });
        let _ = input.add_event_listener_with_callback("change", closure.as_ref().unchecked_ref());
        closure.forget();
    }

    // --- Scale slider: live-drives egui's pixels-per-point via the `input` event. ---
    if let Some(input) = element::<web_sys::HtmlInputElement>(&document, SCALE_ID) {
        input.set_value(&format!("{initial_scale}"));
        set_scale_label(&document, initial_scale);

        let handle_cb = handle.clone();
        let input_cb = input.clone();
        let doc_cb = document.clone();
        let closure = Closure::<dyn FnMut(web_sys::Event)>::new(move |_| {
            if let Ok(scale) = input_cb.value().parse::<f32>() {
                handle_cb.borrow_mut().ui_scale = Some(scale);
                set_scale_label(&doc_cb, scale);
            }
        });
        let _ = input.add_event_listener_with_callback("input", closure.as_ref().unchecked_ref());
        closure.forget();
    }

    // --- Toggle button: shows/hides the scale panel (pure DOM class flip). ---
    if let (Some(button), Some(panel)) = (
        element::<web_sys::HtmlElement>(&document, SCALE_TOGGLE_ID),
        element::<web_sys::HtmlElement>(&document, SCALE_PANEL_ID),
    ) {
        let closure = Closure::<dyn FnMut(web_sys::Event)>::new(move |_| {
            let _ = panel.class_list().toggle("hidden");
        });
        let _ = button.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref());
        closure.forget();
    }

    handle
}

/// Fetch an element by id and cast it to `T`, or `None` if it is absent.
fn element<T: JsCast>(document: &web_sys::Document, id: &str) -> Option<T> {
    document
        .get_element_by_id(id)
        .and_then(|el| el.dyn_into().ok())
}

/// Parse an ISO `YYYY-MM-DD` string (as produced by `<input type="date">`).
fn parse_date(value: &str) -> Option<(i32, u8, u8)> {
    let mut parts = value.split('-');
    let year = parts.next()?.parse().ok()?;
    let month = parts.next()?.parse().ok()?;
    let day = parts.next()?.parse().ok()?;
    Some((year, month, day))
}

/// Update the numeric readout next to the scale slider.
fn set_scale_label(document: &web_sys::Document, scale: f32) {
    if let Some(label) = document.get_element_by_id(SCALE_VALUE_ID) {
        label.set_text_content(Some(&format!("{scale:.2}")));
    }
}
