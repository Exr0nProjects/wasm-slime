use game_loop::game_loop;

use wasm_bindgen::prelude::*;
use web_sys::{ console, window, Node };
use wasm_bindgen::JsCast;

use rand::prelude::thread_rng;
use rand::distributions::{Distribution, Uniform};
use rand_distr::Normal;

use std::f64::consts::PI;
use core::ops::{ Index, IndexMut };
use std::mem::swap;
use std::iter;


// When the `wee_alloc` feature is enabled, this uses `wee_alloc` as the global
// allocator.
//
// If you don't want to use `wee_alloc`, you can safely delete this.
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;


const FRAMERATE: f64 = 100.;
const DIFFUSE_RADIUS: i32 = 1; // diffuse in 3x3 square

#[derive(Debug)]
struct Agent {
    pos_x: f64,
    pos_y: f64,
    vel: f64,
    heading: f64,   // radians
}
impl Agent {
    fn update(&mut self, data: &Vec2d, size_w: usize, size_h: usize) {
        // TODO: sensor checks
        self.pos_y = (self.pos_y + self.vel * self.heading.sin()).rem_euclid(size_h as f64);
        self.pos_x = (self.pos_x + self.vel * self.heading.cos()).rem_euclid(size_w as f64);
    }
    fn deposit(&self) -> (i32, i32, u8) {
        (self.pos_y.round() as i32, self.pos_x.round() as i32, 255)
    }
}

#[derive(Debug)]
struct Vec2d {
    size_w: usize,
    size_h: usize,
    data: Vec<u8>
}
impl Vec2d {
    fn new(size_w: usize, size_h: usize) -> Vec2d {
        Vec2d { size_w, size_h, data: vec![0u8; size_h * size_w] }
    }
}

impl Index<(i32, i32)> for Vec2d {
    type Output = u8;
    fn index(&self, index: (i32, i32)) -> &Self::Output {
        &self.data[index.0.rem_euclid(self.size_h as i32) as usize * self.size_w
                 + index.1.rem_euclid(self.size_w as i32) as usize]
    }
}
impl IndexMut<(i32, i32)> for Vec2d {
    fn index_mut(&mut self, index: (i32, i32)) -> &mut Self::Output {
        &mut self.data[index.0.rem_euclid(self.size_h as i32) as usize * self.size_w
                     + index.1.rem_euclid(self.size_w as i32) as usize]
    }
}

#[derive(Debug)]
struct Dish {
    size_w: usize,
    size_h: usize,
    agents: Vec<Agent>,
    data: Vec2d,
    data_alt: Vec2d,
    canvas: web_sys::HtmlCanvasElement,
}
impl Dish {
    fn new(size_w: usize, size_h: usize) -> Dish {
        println!("new dish with size {} by {}", size_w, size_h);
        let doc = web_sys::window().unwrap().document().unwrap();

        let mut rng = thread_rng();
        let dist_y = Normal::new(0., size_h as f64).expect("Couldn't create normal distribution!");
        let dist_x = Normal::new(0., size_w as f64).expect("Couldn't create normal distribution!");
        let dist_hd = Uniform::from(0f64..PI*2.);
        let agents = iter::repeat(()).take(100)
            .map(|()| Agent {
                pos_y: dist_y.sample(&mut rng),
                pos_x: dist_x.sample(&mut rng),
                vel: 2.,
                heading: dist_hd.sample(&mut rng),
            }).collect();

        Dish { size_w, size_h,
               agents,
               data:     Vec2d::new(size_w, size_h),
               data_alt: Vec2d::new(size_w, size_h),
               canvas: doc.get_element_by_id("slime-canvas").unwrap()
                    .dyn_into::<web_sys::HtmlCanvasElement>()
                    .map_err(|_| ()).unwrap(),
        }
    }
    fn render(&self, updates: u32) {
        let ctx = self.canvas
            .get_context("2d")
            .unwrap()
            .unwrap()
            .dyn_into::<web_sys::CanvasRenderingContext2d>()
            .unwrap();

        //self.canvas.set_width(self.size_w as u32);
        //self.canvas.set_height(self.size_h as u32);

        ctx.clear_rect(0., 0., self.canvas.width() as f64, self.canvas.height() as f64);

        //console::log_1(&JsValue::from_str(&format!("num agents {}", self.agents.len())));
        for y in 0..self.size_w as i32 {
            for x in 0..self.size_h as i32 {
                if self.data[(y, x)] > 0 {
                    //console::log_1(&JsValue::from_str(&format!("{} #{:02x?}{1:02x?}{1:02x?}", self.data[(y, x)], self.data[(y, x)])));
                    ctx.set_fill_style(&JsValue::from_str(
                            &format!("#{:02x?}{0:02x?}{0:02x?}", self.data[(y, x)])
                        ));
                    ctx.fill_rect(x as f64, y as f64, 1., 1.);
                }
            }
        }
    }
}
impl Dish {
    fn update(&mut self, updates: u32) {
        self.diffuse();
        self.decay();
        for agent in &mut self.agents { // NTFS: probably expensive; parallelize
            agent.update(&self.data, self.size_w, self.size_h);
        }
        for agent in &self.agents {
            let (y, x, val) = agent.deposit();
            self.data[(y, x)] = self.data[(y, x)].saturating_add(val);
            //console::log_1(&JsValue::from_str(&format!("val = {} at {}, {}", val, x, y)));
        }
    }
    fn diffuse(&mut self) {
        console::log_1(&JsValue::from_str(&format!("size = {} {}", self.size_w, self.size_h)));
        for cy in 0..self.size_h as i32 {
            for cx in 0..self.size_w as i32 {
                let mut sum = 0i32;
                for y in cy-DIFFUSE_RADIUS..cy+DIFFUSE_RADIUS + 1 {
                    for x in cx-DIFFUSE_RADIUS..cx+DIFFUSE_RADIUS + 1 {
                        sum += self.data[(y, x)] as i32;
                    }
                }
                self.data_alt[(cy, cx)] = (sum / (DIFFUSE_RADIUS * 2 + 1).pow(2)).min(u8::MAX as i32) as u8;
            }
        }
        swap(&mut self.data, &mut self.data_alt);
    }
    fn decay(&mut self) {
        for y in 0..self.size_h as i32 {
            for x in 0..self.size_w as i32 {
                self.data[(y, x)] = (self.data[(y, x)] as f64 * 0.95) as u8;
            }
        }
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
    //let [win_wid, win_hei] = {
    //    let x = document.document_element().unwrap();
    //    [x.client_width(), x.client_height()]
    //};

    let [width, height] = [canvas.client_width(), canvas.client_height()];
     
    // TODO: handle window resizing
    
    let sim = Dish::new(width as usize, height as usize);
    //let sim = Dish::new(300, 100);
    game_loop(sim, 1, 0.1, |g| {
        // update fn
        g.game.update(g.number_of_updates());
    }, |g| {
        // render fn
        g.game.render(g.number_of_updates());
    });

    Ok(())
}
