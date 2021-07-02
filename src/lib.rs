use game_loop::game_loop;

use wasm_bindgen::prelude::*;
use web_sys::{ console, window, Node };
use wasm_bindgen::JsCast;
use js_sys::{ Array, Function };

use std::f64::consts::PI;


// When the `wee_alloc` feature is enabled, this uses `wee_alloc` as the global
// allocator.
//
// If you don't want to use `wee_alloc`, you can safely delete this.
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;


const FRAMERATE: f64 = 100.;

#[wasm_bindgen]
pub fn gameloop() {
    console::log_1(&JsValue::from_str(&format!("amazing: {}", 3)));

    //web_sys::window().unwrap().request_animation_frame(tick.as_ref().unchecked_ref());

    //web_sys::window().unwrap().request_animation_frame(cb.as_ref().unchecked_ref());

}

struct Agent {
    pos_x: f64,
    pos_y: f64,
    heading: f64,   // radians
}

struct Dish {
    size_w: u32,
    size_h: u32,
    agents: Vec<Agent>,
}
impl Dish {
    fn new(size_w: u32, size_h: u32) -> Dish {
        Dish { size_w, size_h, agents: Vec::new() }
    }
    fn render(&self, updates: u32) {
        // TODO: lets not get the canvas from scratch every time
        let window = web_sys::window().unwrap();
        let document = window.document().unwrap();
        let canvas = document.get_element_by_id("slime-canvas").unwrap();
        let canvas: web_sys::HtmlCanvasElement = canvas
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .map_err(|_| ())
            .unwrap();
        let ctx = canvas
            .get_context("2d")
            .unwrap()
            .unwrap()
            .dyn_into::<web_sys::CanvasRenderingContext2d>()
            .unwrap();

        ctx.clear_rect(0., 0., canvas.width() as f64, canvas.height() as f64);

        ctx.set_fill_style(&JsValue::from_str("green"));
        console::log_1(&JsValue::from_str(&format!("amazing: {}", updates % 50)));
        let updates = (updates % 50) as f64;
        ctx.fill_rect(self.size_w as f64/2. - updates/2., self.size_h as f64/2. - updates/2., updates, updates);
    }
}

// This is like the `main` function, except for JavaScript.
#[wasm_bindgen(start)]
pub fn main_js() -> Result<(), JsValue> {
    // This provides better error messages in debug mode.
    // It's disabled in release mode so it doesn't bloat up the file size.
    #[cfg(debug_assertions)]
    console_error_panic_hook::set_once();

    // Your code goes here!
    // carried by https://rustwasm.github.io/wasm-bindgen/examples/2d-canvas.html
    let window = web_sys::window().unwrap();
    let document = window.document().unwrap();
    let canvas = document.get_element_by_id("slime-canvas").unwrap();
    let canvas: web_sys::HtmlCanvasElement = canvas
        .dyn_into::<web_sys::HtmlCanvasElement>()
        .map_err(|_| ())
        .unwrap();
    let [win_wid, win_hei] = {
        let x = document.document_element().unwrap();
        [x.client_width(), x.client_height()]
    };

    let ctx = canvas
        .get_context("2d")
        .unwrap()
        .unwrap()
        .dyn_into::<web_sys::CanvasRenderingContext2d>()
        .unwrap();

    let [width, height] = [canvas.width(), canvas.height()];
     
    // TODO: handle window resizing

    //ctx.set_fill_style(&JsValue::from_str("green"));
    //ctx.fill_rect(width/2. - 50., height/2. - 50., 100., 100.);
    //
    //if let Some(win) = window() {
    //    console::log_1(&JsValue::from_str("Got window!"));
    //    if let Some(doc) = win.document() {
    //        console::log_1(&JsValue::from_str("Got document!"));
    //        if let Ok(Some(body)) = doc.query_selector("body") {
    //            let p: Node = doc.create_element("p")?.into();
    //            p.set_text_content(Some("Hello from Rust, WebAssembly, and Webpack!"));
    //            body.prepend_with_node_1(&p);
    //            console::log_1(&JsValue::from_str("should have appended paragraph.. :thinking:"));
    //        }
    //    }
    //}


    //let tick = Closure::wrap(Box::new(move || {
    //        console::log_1(&JsValue::from_str(&format!("amazing: {}", 3)));
    //    }) as Box<dyn FnMut()>);
    //
    ////let tick = Closure::wrap(Box::new(gameloop));
    //
    //// when rust has a java moment
    //let interval_id = window.set_interval_with_callback_and_timeout_and_arguments_0(
    //    tick.as_ref().unchecked_ref(), (1000. / FRAMERATE) as i32); // https://docs.rs/wasm-bindgen/0.2.74/wasm_bindgen/closure/struct.Closure.html
    

    let sim = Dish::new(width, height);
    game_loop(sim, 240, 0.1, |g| {
        // update fn
    }, |g| {
        // render fn
        g.game.render(g.number_of_updates());
    });

    Ok(())
}
