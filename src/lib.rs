#![feature(array_map)]
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
use std::time::Duration;
use std::thread::sleep;


// When the `wee_alloc` feature is enabled, this uses `wee_alloc` as the global
// allocator.
//
// If you don't want to use `wee_alloc`, you can safely delete this.
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;


const FRAMERATE: f64 = 100.;
const NUM_AGENTS: usize = 20;
const DIFFUSE_RADIUS: i32 = 1; // diffuse in 3x3 square
const SENSOR_RADIUS: f64 = 2.;
const SENSOR_ANGLE: f64 = PI/4.;
const SENSOR_DISTANCE: f64 = 8.;
const TURN_ANGLE: f64 = PI/12.;
const VELOCITY: f64 = 1.2;

#[derive(Debug)]
struct Agent {
    pos_x: f64,
    pos_y: f64,
    vel: f64,
    heading: f64,   // radians
    prev: i32,
    lef: i32,
    rig: i32,
    fwd: i32,
}
impl Agent {
    fn update(&mut self, data: &Vec2d, size_w: usize, size_h: usize, rand: f64) -> i32 {
        assert!(0. <= rand && rand < 1.);
        let [lef, fwd, rig] = [(self.pos_x + SENSOR_DISTANCE * (self.heading - SENSOR_ANGLE).cos(),
                            self.pos_y + SENSOR_DISTANCE * (self.heading - SENSOR_ANGLE).sin()),
                           (self.pos_x + SENSOR_DISTANCE * (self.heading               ).cos(),
                            self.pos_y + SENSOR_DISTANCE * (self.heading               ).sin()),
                           (self.pos_x + SENSOR_DISTANCE * (self.heading + SENSOR_ANGLE).cos(),
                            self.pos_y + SENSOR_DISTANCE * (self.heading + SENSOR_ANGLE).sin()),
        ].map(|(cy, cx)| {
            let mut sum = 0i32;
            // TODO: circular
            for y in (cy-SENSOR_RADIUS).round() as i32..(cy+SENSOR_RADIUS).round() as i32 {
                for x in (cx-SENSOR_RADIUS).round() as i32..(cx+SENSOR_RADIUS).round() as i32 {
                    sum += data[(y, x)] as i32
                }
            }
            sum
        });

        self.prev = 0;

        // TODO: use the actual random algo
        if      fwd > lef && fwd > rig {}
        else if fwd < lef && fwd < rig { 
            if rand < lef as f64 / (lef + rig) as f64 {
                self.heading += TURN_ANGLE;
            } else {
                self.heading -= TURN_ANGLE;
            }
        } else if lef > rig {
            self.prev = -1;
            self.heading += TURN_ANGLE;
        } else if rig > lef {
            self.prev = 1;
            self.heading -= TURN_ANGLE;
        }

        self.lef = lef; self.rig = rig; self.fwd = fwd;

        // TODO: sensor checks
        self.pos_y = (self.pos_y + self.vel * self.heading.sin()).rem_euclid(size_h as f64);
        self.pos_x = (self.pos_x + self.vel * self.heading.cos()).rem_euclid(size_w as f64);
        self.prev
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
    rng: ThreadRng,
}
impl Dish {
    fn new(size_w: usize, size_h: usize) -> Dish {
        println!("new dish with size {} by {}", size_w, size_h);
        let doc = web_sys::window().unwrap().document().unwrap();

        let mut rng = thread_rng();

        //let agents = { // rect random
        //    let dist_y = Normal::new(0., size_h as f64).expect("Couldn't create normal distribution!");
        //    let dist_x = Normal::new(0., size_w as f64).expect("Couldn't create normal distribution!");
        //    let dist_hd = Uniform::from(0f64..PI*2.);
        //    iter::repeat(()).take(NUM_AGENTS)
        //        .map(|()| Agent {
        //        pos_y: dist_y.sample(&mut rng),
        //        pos_x: dist_x.sample(&mut rng),
        //        vel: VELOCITY,
        //        heading: dist_hd.sample(&mut rng),
        //    }).collect()
        //};

        let agents = { // circular
            let circle_radius = (size_w.min(size_h)* 2/ 10) as f64;
            let dist_hd = Uniform::from(0f64..PI*2.);
            iter::repeat(()).take(NUM_AGENTS)
                .map(|()| {
                let hd = dist_hd.sample(&mut rng);
                Agent {
                    pos_y: circle_radius*hd.sin() + size_h as f64/ 4.,
                    pos_x: circle_radius*hd.cos() + size_w as f64/ 4.,
                    vel: VELOCITY,
                    heading: (hd + PI/2.).rem_euclid(PI*2.),
                    prev: 0, lef: 0, rig: 0, fwd: 0,
                }
                }).collect()
        };

        Dish { size_w, size_h,
               agents,
               data:     Vec2d::new(size_w, size_h),
               data_alt: Vec2d::new(size_w, size_h),
               canvas: doc.get_element_by_id("slime-canvas").unwrap()
                    .dyn_into::<web_sys::HtmlCanvasElement>()
                    .map_err(|_| ()).unwrap(),
               rng,
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
                    ctx.fill_rect((x*10 - 5) as f64, (y*10 - 5) as f64, 10., 10.);
                }
            }
        }
        ctx.set_line_width(2.);
        for agent in &self.agents {
            ctx.set_fill_style(&JsValue::from_str("green"));
            ctx.fill_rect(agent.pos_x*10.-2., agent.pos_y*10.-2., 4., 4.);
            // other sensors
            ctx.set_fill_style(&JsValue::from_str(if agent.prev > 0 { "#ff000033" } else { "#0000ff33" }));
            ctx.fill_rect((agent.pos_x + SENSOR_DISTANCE * (agent.heading + SENSOR_ANGLE).cos() - SENSOR_RADIUS) *10.,
                          (agent.pos_y + SENSOR_DISTANCE * (agent.heading + SENSOR_ANGLE).sin() - SENSOR_RADIUS) *10.,
                          (SENSOR_RADIUS * 2. + 1.)*10., (SENSOR_RADIUS * 2. + 1.)*10.);
            ctx.set_fill_style(&JsValue::from_str("white"));
            ctx.fill_text(&format!("left: {}", agent.lef),
                          (agent.pos_x + SENSOR_DISTANCE * (agent.heading + SENSOR_ANGLE).cos() - SENSOR_RADIUS) *10.,
                          (agent.pos_y + SENSOR_DISTANCE * (agent.heading + SENSOR_ANGLE).sin() - SENSOR_RADIUS) *10.);

            ctx.set_fill_style(&JsValue::from_str(if agent.prev < 0 { "#ff000033" } else { "#0000ff33" }));
            ctx.fill_rect((agent.pos_x + SENSOR_DISTANCE * (agent.heading - SENSOR_ANGLE).cos() - SENSOR_RADIUS) *10.,
                          (agent.pos_y + SENSOR_DISTANCE * (agent.heading - SENSOR_ANGLE).sin() - SENSOR_RADIUS) *10.,
                          (SENSOR_RADIUS * 2. + 1.)*10., (SENSOR_RADIUS * 2. + 1.)*10.);
            ctx.set_fill_style(&JsValue::from_str("white"));
            ctx.fill_text(&format!("right: {}", agent.rig),
                          (agent.pos_x + SENSOR_DISTANCE * (agent.heading - SENSOR_ANGLE).cos() - SENSOR_RADIUS) *10.,
                          (agent.pos_y + SENSOR_DISTANCE * (agent.heading - SENSOR_ANGLE).sin() - SENSOR_RADIUS) *10.);

            ctx.set_fill_style(&JsValue::from_str(if agent.prev == 0 { "#ff000033" } else { "#00ff0033" }));
            ctx.fill_rect((agent.pos_x + SENSOR_DISTANCE * (agent.heading).cos() - SENSOR_RADIUS) *10.,
                          (agent.pos_y + SENSOR_DISTANCE * (agent.heading).sin() - SENSOR_RADIUS) *10., (SENSOR_RADIUS * 2. + 1.)*10., (SENSOR_RADIUS * 2. + 1.)*10.);
            ctx.set_fill_style(&JsValue::from_str("white"));
            ctx.fill_text(&format!("center: {}", agent.fwd),
                          (agent.pos_x + SENSOR_DISTANCE * (agent.heading).cos() - SENSOR_RADIUS) *10.,
                          (agent.pos_y + SENSOR_DISTANCE * (agent.heading).sin() - SENSOR_RADIUS) *10.);

            ctx.begin_path();
            ctx.set_stroke_style(&JsValue::from_str("blue"));
            ctx.move_to((agent.pos_x + SENSOR_DISTANCE * (agent.heading + SENSOR_ANGLE).cos()) *10.,
                        (agent.pos_y + SENSOR_DISTANCE * (agent.heading + SENSOR_ANGLE).sin()) *10.);
            ctx.line_to(agent.pos_x*10., agent.pos_y*10.);
            ctx.line_to((agent.pos_x + SENSOR_DISTANCE * (agent.heading - SENSOR_ANGLE).cos()) *10.,
                        (agent.pos_y + SENSOR_DISTANCE * (agent.heading - SENSOR_ANGLE).sin()) *10.);
            ctx.stroke();
            // center line
            ctx.set_stroke_style(&JsValue::from_str("green"));
            ctx.begin_path();
            ctx.move_to(agent.pos_x*10., agent.pos_y*10.);
            ctx.line_to((agent.pos_x + SENSOR_DISTANCE * agent.heading.cos()) *10.,
                        (agent.pos_y + SENSOR_DISTANCE * agent.heading.sin()) *10.);
            ctx.stroke();
        }
        for i in 0..2e5 as i32 {
            console::log_1(&JsValue::from_str("nuffin"));
        }
    }
}
impl Dish {
    fn update(&mut self, updates: u32) {
        self.diffuse();
        self.decay();
        let dist = Uniform::new(0., 1.);
        for agent in &mut self.agents { // NTFS: probably expensive; parallelize
            agent.update(&self.data, self.size_w, self.size_h, self.rng.sample(dist));
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
                self.data[(y, x)] = (self.data[(y, x)] as f64 * 0.97) as u8;
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
    
    let sim = Dish::new((width/10) as usize, (height/10) as usize);
    //let sim = Dish::new(300, 100);
    game_loop(sim, 5, 0.2, |g| {
        // update fn
        g.game.update(g.number_of_updates());
    }, |g| {
        // render fn
        g.game.render(g.number_of_updates());
    });

    Ok(())
}
