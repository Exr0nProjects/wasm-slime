use wasm_bindgen::prelude::*;
use web_sys::{ console, window, Node };
use js_sys::{ Array };


// When the `wee_alloc` feature is enabled, this uses `wee_alloc` as the global
// allocator.
//
// If you don't want to use `wee_alloc`, you can safely delete this.
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;


// This is like the `main` function, except for JavaScript.
#[wasm_bindgen(start)]
pub fn main_js() -> Result<(), JsValue> {
    // This provides better error messages in debug mode.
    // It's disabled in release mode so it doesn't bloat up the file size.
    #[cfg(debug_assertions)]
    console_error_panic_hook::set_once();


    // Your code goes here!
    if let Some(win) = window() {
        console::log_1(&JsValue::from_str("Got window!"));
        if let Some(doc) = win.document() {
            console::log_1(&JsValue::from_str("Got document!"));
            if let Ok(Some(body)) = doc.query_selector("body") {
                let p: Node = doc.create_element("p")?.into();
                p.set_text_content(Some("Hello from Rust, WebAssembly, and Webpack!"));
                body.append_with_node_1(&p);
                console::log_1(&JsValue::from_str("should have appended paragraph.. :thinking:"));
            }
        }
    }


    //match window() {
    //    Some(win) => {
    //        console::log_1(&JsValue::from_str("Got window!"));
    //    },
    //    _ => {
    //        console::log_1(&JsValue::from_str("Couldn't get window!"));
    //    }
    //}
    //let p = document.create_element("p")?.into();
    //p.set_text_content(Some("Hello from Rust, WebAssembly, and Webpack!"));

    Ok(())
}
