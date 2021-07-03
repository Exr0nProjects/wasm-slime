use game_loop::game_loop;

use wasm_bindgen::prelude::*;
use web_sys::{ console, window, Node };
use wasm_bindgen::JsCast;
use js_sys::{ Array, Function };

use rand::prelude::thread_rng;
use rand::distributions::{Distribution, Uniform};
use rand_distr::Normal;

use std::f64::consts::PI;
use core::ops::{ Index, IndexMut };
use std::collections::VecDeque;
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

#[wasm_bindgen]
pub fn gameloop() {
    console::log_1(&JsValue::from_str(&format!("amazing: {}", 3)));

    //web_sys::window().unwrap().request_animation_frame(tick.as_ref().unchecked_ref());

    //web_sys::window().unwrap().request_animation_frame(cb.as_ref().unchecked_ref());

}

#[derive(Debug)]
struct Agent {
    pos_x: f64,
    pos_y: f64,
    vel: f64,
    heading: f64,   // radians
}
impl Agent {
    //fn in_rect_rng(size_w: usize, size_h: usize, rng: ThreadRng) -> Agent {
    //    Agent { pos_x: Normal::from(0, size_w).sample(&mut rng) }
    //    // TODO
    //}
    fn update(&mut self, data: &Vec2d, size_w: usize, size_h: usize) {
        // TODO: sensor checks
        self.pos_y = (self.pos_y + self.vel * self.heading.sin()).rem_euclid(size_h as f64);
        self.pos_x = (self.pos_x + self.vel * self.heading.sin()).rem_euclid(size_w as f64);
    }
    fn deposit(&self) -> (i32, i32, u8) {
        (self.pos_y.round() as i32, self.pos_x.round() as i32, 255)
    }
}

//#[derive(Debug)]
//struct Pixel {
//    val: u8,
//}
//struct Pixel(u8);

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
    //data: Vec<Vec<Pixel>>,
    //trail: Vec<Vec<u8>>,
}
impl Dish {
    fn new(size_w: usize, size_h: usize) -> Dish {
        let doc = web_sys::window().unwrap().document().unwrap();

        let mut rng = thread_rng();
        let dist_y = Normal::new(0., size_h as f64).expect("Couldn't create normal distribution!");
        let dist_x = Normal::new(0., size_w as f64).expect("Couldn't create normal distribution!");
        let dist_hd = Uniform::from(0f64..PI*2.);
        let agents = iter::repeat(()).take(100)
            .map(|()| Agent {
                pos_y: dist_y.sample(&mut rng),
                pos_x: dist_x.sample(&mut rng),
                vel: 10.,
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
        // TODO: lets not get the canvas from scratch every time
        //let window = web_sys::window().unwrap();
        //let document = window.document().unwrap();
        //let canvas = document.get_element_by_id("slime-canvas").unwrap();
        //let canvas: web_sys::HtmlCanvasElement = canvas
        //    .dyn_into::<web_sys::HtmlCanvasElement>()
        //    .map_err(|_| ())
        //    .unwrap();
        let ctx = self.canvas
            .get_context("2d")
            .unwrap()
            .unwrap()
            .dyn_into::<web_sys::CanvasRenderingContext2d>()
            .unwrap();

        self.canvas.set_width(self.size_w as u32);
        self.canvas.set_height(self.size_h as u32);

        ctx.clear_rect(0., 0., self.canvas.width() as f64, self.canvas.height() as f64);

        ctx.set_fill_style(&JsValue::from_str("white"));
        //console::log_1(&JsValue::from_str(&format!("amazing: {}", updates % 50)));
        //let updates = (updates % 50) as f64;
        //ctx.fill_rect(self.size_w as f64/2. - updates/2., self.size_h as f64/2. - updates/2., updates, updates);
        console::log_1(&JsValue::from_str(&format!("num agents {}", self.agents.len())));
        for y in 0..self.size_w as i32 {
            for x in 0..self.size_h as i32 {
                if self.data[(y, x)] > 0 {
                    console::log_1(&JsValue::from_str(&format!("#{:02x?}{0:02x?}{0:02x?}", self.data[(y, x)])));
                    ctx.set_fill_style(&JsValue::from_str(
                            &format!("#{:02x?}{0:02x?}{0:02x?}", self.data[(y, x)])
                        ));
                    ctx.fill_rect(y as f64, x as f64, 1., 1.);
                }
            }
        }
    }
}
impl Dish {
    fn update(&mut self, updates: u32) {
        for agent in &mut self.agents { // NTFS: probably expensive; parallelize
            agent.update(&self.data, self.size_w, self.size_h);
        }
        for agent in &self.agents {
            let (y, x, val) = agent.deposit();
            self.data[(y, x)].saturating_add(val);
        }
        self.diffuse();
        self.decay();
    }
    fn diffuse(&mut self) {
        // lets not use rolling because maybe we'll want a larger diffuse kernal, easier to just
        // swap
        //let mut buf = VecDeque::<u8>::new();
        //buf.reserve_exact((self.size_w + 1) as usize);  // rolling array for calculating average in 3x3
        //
        //buf.push_front(self.data[(-1, -1)]);
        //// TODO: insert bottom row 
        //buf.push_back (self.data[]);
        //
        //// TODO copy in for init
        //for y in 0..self.size_h {
        //    for x in 0..self.size_w {
        //        buf.push_back(self.trail[y][x]);
        //        self.trail[y][x] = (
        //            self.trail[y][x] 
        //          + buf[0]
        //          + buf[1]
        //          + buf[2]
        //          + buf[buf.len()-2]
        //          + self.trail[y+1][x  ] 
        //          + self.trail[y+1][x+1] 
        //          + self.trail[y  ][x+1] 
        //          + self.trail[y+1][x-1]
        //          )/9;
        //        buf.pop_front();
        //    } 
        //}
        for cy in 0..self.size_h as i32 {
            for cx in 0..self.size_w as i32 {
                let mut sum = 0i32;
                for y in cy-DIFFUSE_RADIUS..cy+DIFFUSE_RADIUS + 1 {
                    for x in cy-DIFFUSE_RADIUS..cy+DIFFUSE_RADIUS + 1 {
                        sum += self.data[(y, x)] as i32;
                    }
                }
                self.data_alt[(cy, cx)] = (sum / (DIFFUSE_RADIUS * 2 + 1).pow(2)).min(u8::MAX as i32) as u8;
            }
        }
        swap(&mut self.data, &mut self.data_alt);
    }
    fn decay(&mut self) {
        //for row in &mut self.trail {
        //    for pix in row.iter_mut() {
        //        *pix /= 2;
        //    }
        //}
        for y in 0..self.size_h as i32 {
            for x in 0..self.size_w as i32 {
                self.data[(y, x)] /= 2;
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
    

    //let sim = Dish::new(width as usize, height as usize);
    let sim = Dish::new(300, 100);
    game_loop(sim, 240, 0.1, |g| {
        // update fn
        g.game.update(g.number_of_updates());
    }, |g| {
        // render fn
        g.game.render(g.number_of_updates());
    });

    Ok(())
}
