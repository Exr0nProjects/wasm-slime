#![feature(array_map)]
mod utils;

use wasm_bindgen::prelude::*;

use game_loop::game_loop;

use wasm_bindgen::prelude::*;
use web_sys::{ console, window, Node };
use wasm_bindgen::JsCast;

use rand::prelude::{ thread_rng, ThreadRng, Rng };
use rand::distributions::{Distribution, Uniform};
use rand_distr::Normal;

use std::f64::consts::PI;
use core::ops::{ Index, IndexMut };
use std::mem::swap;
use std::iter;
use std::collections::VecDeque; // NTFS OPTM: replace with queues = "1.1.0" CircularBuffer
use std::time::Duration;
use std::thread::sleep;


// When the `wee_alloc` feature is enabled, use `wee_alloc` as the global
// allocator.
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[wasm_bindgen]
pub fn greet() {
    let window = web_sys::window().unwrap();
    let document = window.document().unwrap();
    let canvas = document.get_element_by_id("slime-canvas").unwrap();
    let canvas: web_sys::HtmlCanvasElement = canvas
        .dyn_into::<web_sys::HtmlCanvasElement>()
        .map_err(|_| ())
        .unwrap();

}
