#![feature(array_map)]
use game_loop::game_loop;

use wasm_bindgen::prelude::*;
use web_sys::{ console, window, Node };
use web_sys::{ WebGlProgram, WebGlRenderingContext, WebGlShader, WebGlTexture };
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


// When the `wee_alloc` feature is enabled, this uses `wee_alloc` as the global
// allocator.
//
// If you don' t want to use `wee_alloc`, you can safely delete this.
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;


const FRAMERATE: f64 = 100.;
const WORLD_SIZE: (usize, usize) = (512, 256);
const NUM_AGENTS: usize = 800;
const DIFFUSE_RADIUS: i32 = 1; // diffuse in 3x3 square
const SENSOR_RADIUS: f64 = 2.;
const SENSOR_ANGLE: f64 = PI/3.;
const SENSOR_DISTANCE: f64 = 8.;
const TURN_ANGLE: f64 = PI/12.;
const VELOCITY: f64 = 2.;

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
    fn update(&mut self, data: &Vec2d<u8>, size_w: usize, size_h: usize, rand: f64) -> i32 {
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
struct Vec2d<T: Clone> {
    size_w: usize,
    size_h: usize,
    data: Vec<T>
}
impl<T: Clone> Vec2d<T> {
    fn new(size_w: usize, size_h: usize, fill: T) -> Vec2d<T> {
        Vec2d { size_w, size_h, data: vec![fill; size_h * size_w] }
    }
    fn resize(&mut self) {} // TODO
    fn for_each<F>(&mut self, f: F) where F: FnMut(&mut T) {
        self.data.iter_mut().for_each(f);
    }
}

// TODO: https://stackoverflow.com/questions/57203009/implementing-slice-for-custom-type (for iter_mut)
impl<T: Clone> Index<(i32, i32)> for Vec2d<T> {
    type Output = T;
    fn index(&self, index: (i32, i32)) -> &Self::Output {
        &self.data[index.0.rem_euclid(self.size_h as i32) as usize * self.size_w
                 + index.1.rem_euclid(self.size_w as i32) as usize]
    }
}
impl<T: Clone> IndexMut<(i32, i32)> for Vec2d<T> {
    fn index_mut(&mut self, index: (i32, i32)) -> &mut Self::Output {
        &mut self.data[index.0.rem_euclid(self.size_h as i32) as usize * self.size_w
                     + index.1.rem_euclid(self.size_w as i32) as usize]
    }
}

impl<T: Clone> IntoIterator for Vec2d<T> {
    type Item = T;
    type IntoIter = std::vec::IntoIter<Self::Item>;
    fn into_iter(self) -> Self::IntoIter {
        self.data.into_iter()
    }
}

#[derive(Debug)]
struct Dish {
    size_w: usize,
    size_h: usize,

    agents: Vec<Agent>,
    data: Vec2d<u8>,
    data_alt: Vec2d<u8>,
    visited: Vec2d<bool>,               // inq, for SPFA style update
    active_cells: VecDeque<(i32, i32)>, // invariant: contains all active cells at beginning of render()

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
        
        let mut active_cells = VecDeque::new();
        active_cells.reserve(NUM_AGENTS);

        let agents = { // circular
            let circle_radius = (size_w.min(size_h)* 2/ 10) as f64;
            let dist_hd = Uniform::from(0f64..PI*2.);
            iter::repeat(()).take(NUM_AGENTS)
                .map(|()| {
                let hd = dist_hd.sample(&mut rng);

                let y = circle_radius*hd.sin() + size_h as f64/ 4.;
                let x = circle_radius*hd.cos() + size_w as f64/ 4.;
                active_cells.push_back((y.round() as i32, x.round() as i32));

                Agent {
                    pos_y: y,
                    pos_x: x,
                    vel: VELOCITY,
                    heading: (hd + PI/2.).rem_euclid(PI*2.),
                    prev: 0, lef: 0, rig: 0, fwd: 0,
                }
                }).collect()
        };

        Dish { size_w, size_h,
               agents,
               data:     Vec2d::new(size_w, size_h, 0u8),
               data_alt: Vec2d::new(size_w, size_h, 0u8),
               visited:  Vec2d::new(size_w, size_h, false),
               active_cells,
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

        for y in 0..self.size_w as i32 {
            for x in 0..self.size_h as i32 {
                if self.data[(y, x)] > 0 {
                    ctx.set_fill_style(&JsValue::from_str(
                            &format!("#{:02x?}{0:02x?}{0:02x?}", self.data[(y, x)])
                        ));
                    //ctx.fill_rect((x*10 - 5) as f64, (y*10 - 5) as f64, 10., 10.);
                    ctx.fill_rect((x) as f64, (y) as f64, 1., 1.);
                }
            }
        }
        //ctx.set_line_width(2.);
        //for agent in &self.agents {
        //    ctx.set_fill_style(&JsValue::from_str("green"));
        //    //ctx.fill_rect(agent.pos_x*10.-2., agent.pos_y*10.-2., 4., 4.);
        //    
        //
        //    //// standard (but broken)
        //    //ctx.set_fill_style(&JsValue::from_str(if agent.prev > 0 { "#ff000033" } else { "#0000ff33" }));
        //    //ctx.fill_rect(agent.pos_x + SENSOR_DISTANCE * agent.heading + SENSOR_ANGLE.cos() - SENSOR_RADIUS ,
        //    //              agent.pos_y + SENSOR_DISTANCE * agent.heading + SENSOR_ANGLE.sin() - SENSOR_RADIUS ,
        //    //              SENSOR_RADIUS * 2. + 1., SENSOR_RADIUS * 2. + 1.);
        //    //ctx.set_fill_style(&JsValue::from_str("white"));
        //    //ctx.fill_text(&format!("left: {}", agent.lef),
        //    //              agent.pos_x + SENSOR_DISTANCE * agent.heading + SENSOR_ANGLE.cos() - SENSOR_RADIUS ,
        //    //              agent.pos_y + SENSOR_DISTANCE * agent.heading + SENSOR_ANGLE.sin() - SENSOR_RADIUS );
        //    //
        //    //ctx.set_fill_style(&JsValue::from_str(if agent.prev < 0 { "#ff000033" } else { "#0000ff33" }));
        //    //ctx.fill_rect(agent.pos_x + SENSOR_DISTANCE * agent.heading - SENSOR_ANGLE.cos() - SENSOR_RADIUS ,
        //    //              agent.pos_y + SENSOR_DISTANCE * agent.heading - SENSOR_ANGLE.sin() - SENSOR_RADIUS ,
        //    //              SENSOR_RADIUS * 2. + 1., SENSOR_RADIUS * 2. + 1.);
        //    //ctx.set_fill_style(&JsValue::from_str("white"));
        //    //ctx.fill_text(&format!("right: {}", agent.rig),
        //    //              agent.pos_x + SENSOR_DISTANCE * agent.heading - SENSOR_ANGLE.cos() - SENSOR_RADIUS ,
        //    //              agent.pos_y + SENSOR_DISTANCE * agent.heading - SENSOR_ANGLE.sin() - SENSOR_RADIUS );
        //    //
        //    //ctx.set_fill_style(&JsValue::from_str(if agent.prev == 0 { "#ff000033" } else { "#00ff0033" }));
        //    //ctx.fill_rect(agent.pos_x + SENSOR_DISTANCE * agent.heading.cos() - SENSOR_RADIUS ,
        //    //              agent.pos_y + SENSOR_DISTANCE * agent.heading.sin() - SENSOR_RADIUS ,
        //    //              SENSOR_RADIUS * 2. + 1., SENSOR_RADIUS * 2. + 1.);
        //    //ctx.set_fill_style(&JsValue::from_str("white"));
        //    //ctx.fill_text(&format!("center: {}", agent.fwd),
        //    //              agent.pos_x + SENSOR_DISTANCE * agent.heading.cos() - SENSOR_RADIUS ,
        //    //              agent.pos_y + SENSOR_DISTANCE * agent.heading.sin() - SENSOR_RADIUS );
        //    //
        //    //ctx.begin_path();
        //    //ctx.set_stroke_style(&JsValue::from_str("blue"));
        //    //ctx.move_to(agent.pos_x + SENSOR_DISTANCE * (agent.heading + SENSOR_ANGLE).cos() ,
        //    //            agent.pos_y + SENSOR_DISTANCE * (agent.heading + SENSOR_ANGLE).sin() );
        //    //ctx.line_to(agent.pos_x, agent.pos_y);
        //    //ctx.line_to(agent.pos_x + SENSOR_DISTANCE * (agent.heading - SENSOR_ANGLE).cos() ,
        //    //            agent.pos_y + SENSOR_DISTANCE * (agent.heading - SENSOR_ANGLE).sin() );
        //    //ctx.stroke();
        //    //// center line
        //    //ctx.set_stroke_style(&JsValue::from_str("green"));
        //    //ctx.begin_path();
        //    //ctx.move_to(agent.pos_x, agent.pos_y);
        //    //ctx.line_to(agent.pos_x + SENSOR_DISTANCE * agent.heading.cos() ,
        //    //            agent.pos_y + SENSOR_DISTANCE * agent.heading.sin() );
        //    //ctx.stroke();
        //
        //    // times 10
        //    //// other sensors
        //    //ctx.set_fill_style(&JsValue::from_str(if agent.prev > 0 { "#ff000033" } else { "#0000ff33" }));
        //    //ctx.fill_rect((agent.pos_x + SENSOR_DISTANCE * (agent.heading + SENSOR_ANGLE).cos() - SENSOR_RADIUS) *10.,
        //    //              (agent.pos_y + SENSOR_DISTANCE * (agent.heading + SENSOR_ANGLE).sin() - SENSOR_RADIUS) *10.,
        //    //              (SENSOR_RADIUS * 2. + 1.)*10., (SENSOR_RADIUS * 2. + 1.)*10.);
        //    //ctx.set_fill_style(&JsValue::from_str("white"));
        //    //ctx.fill_text(&format!("left: {}", agent.lef),
        //    //              (agent.pos_x + SENSOR_DISTANCE * (agent.heading + SENSOR_ANGLE).cos() - SENSOR_RADIUS) *10.,
        //    //              (agent.pos_y + SENSOR_DISTANCE * (agent.heading + SENSOR_ANGLE).sin() - SENSOR_RADIUS) *10.);
        //    //
        //    //ctx.set_fill_style(&JsValue::from_str(if agent.prev < 0 { "#ff000033" } else { "#0000ff33" }));
        //    //ctx.fill_rect((agent.pos_x + SENSOR_DISTANCE * (agent.heading - SENSOR_ANGLE).cos() - SENSOR_RADIUS) *10.,
        //    //              (agent.pos_y + SENSOR_DISTANCE * (agent.heading - SENSOR_ANGLE).sin() - SENSOR_RADIUS) *10.,
        //    //              (SENSOR_RADIUS * 2. + 1.)*10., (SENSOR_RADIUS * 2. + 1.)*10.);
        //    //ctx.set_fill_style(&JsValue::from_str("white"));
        //    //ctx.fill_text(&format!("right: {}", agent.rig),
        //    //              (agent.pos_x + SENSOR_DISTANCE * (agent.heading - SENSOR_ANGLE).cos() - SENSOR_RADIUS) *10.,
        //    //              (agent.pos_y + SENSOR_DISTANCE * (agent.heading - SENSOR_ANGLE).sin() - SENSOR_RADIUS) *10.);
        //    //
        //    //ctx.set_fill_style(&JsValue::from_str(if agent.prev == 0 { "#ff000033" } else { "#00ff0033" }));
        //    //ctx.fill_rect((agent.pos_x + SENSOR_DISTANCE * (agent.heading).cos() - SENSOR_RADIUS) *10.,
        //    //              (agent.pos_y + SENSOR_DISTANCE * (agent.heading).sin() - SENSOR_RADIUS) *10., (SENSOR_RADIUS * 2. + 1.)*10., (SENSOR_RADIUS * 2. + 1.)*10.);
        //    //ctx.set_fill_style(&JsValue::from_str("white"));
        //    //ctx.fill_text(&format!("center: {}", agent.fwd),
        //    //              (agent.pos_x + SENSOR_DISTANCE * (agent.heading).cos() - SENSOR_RADIUS) *10.,
        //    //              (agent.pos_y + SENSOR_DISTANCE * (agent.heading).sin() - SENSOR_RADIUS) *10.);
        //    //
        //    //ctx.begin_path();
        //    //ctx.set_stroke_style(&JsValue::from_str("blue"));
        //    //ctx.move_to((agent.pos_x + SENSOR_DISTANCE * (agent.heading + SENSOR_ANGLE).cos()) *10.,
        //    //            (agent.pos_y + SENSOR_DISTANCE * (agent.heading + SENSOR_ANGLE).sin()) *10.);
        //    //ctx.line_to(agent.pos_x*10., agent.pos_y*10.);
        //    //ctx.line_to((agent.pos_x + SENSOR_DISTANCE * (agent.heading - SENSOR_ANGLE).cos()) *10.,
        //    //            (agent.pos_y + SENSOR_DISTANCE * (agent.heading - SENSOR_ANGLE).sin()) *10.);
        //    //ctx.stroke();
        //    //// center line
        //    //ctx.set_stroke_style(&JsValue::from_str("green"));
        //    //ctx.begin_path();
        //    //ctx.move_to(agent.pos_x*10., agent.pos_y*10.);
        //    //ctx.line_to((agent.pos_x + SENSOR_DISTANCE * agent.heading.cos()) *10.,
        //    //            (agent.pos_y + SENSOR_DISTANCE * agent.heading.sin()) *10.);
        //    //ctx.stroke();
        //}
        ////for i in 0..2e5 as i32 {
        ////    console::log_1(&JsValue::from_str("nuffin"));
        ////}
    }
    fn render_webgl(&self, updates: u32) {
        use WebGlRenderingContext as GLC;
        let ctx = self.canvas
            .get_context("webgl")
            .unwrap()
            .unwrap()
            .dyn_into::<web_sys::WebGlRenderingContext>()
            .unwrap();

        ctx.clear_color(0., 0., 0.2, 1.);
        ctx.clear(WebGlRenderingContext::COLOR_BUFFER_BIT);

        let vert_shader = compile_shader(
            &ctx,
            WebGlRenderingContext::VERTEX_SHADER,
            r#"
            varying vec4 vpass;
            attribute vec4 position;
            void main() {
                gl_Position = position;
                vpass = gl_Position * 0.5 + 0.5;
            }
            "#,
            ).expect("couldn't compile vert shader");
        
        //let frag_shader = compile_shader(
        //    &ctx,
        //    WebGlRenderingContext::FRAGMENT_SHADER,
        //    r#"
        //    void main() {
        //        gl_FragColor = vec4(0.2, 0.2, 0.5, 1.0);
        //    }
        //    "#,
        //    ).expect("couldn't compile frag shader");
        let frag_shader = compile_shader(
            &ctx,
            WebGlRenderingContext::FRAGMENT_SHADER,
            r#"
            precision mediump float;

            varying vec4 vpass;
            //varying highp vec2 vTextureCoord;

            uniform sampler2D state;

            void main() {
                //gl_FragColor = vec4(0.2, 0.2, 0.5, 1.0);
                //gl_FragColor = texture2D(state, gl_FragCoord.xy)
                gl_FragColor = vpass;
            }
            "#,
            ).expect("couldn't compile frag shader");
        
        let trail_map_program = link_program(&ctx, &vert_shader, &frag_shader).expect("couldn't link webgl program");

        //let copy_program = 0; // TODO
        ctx.use_program(Some(&trail_map_program));

        let plane_verts: [f32; 4*3] = [-0.8, -0.8, 0., -0.8, 0.8, 0., 0.8, 0.8, 0., 0.8, -0.8, 0.];
        //let plane_verts: [f32; 9] = [-0.7, -0.7, 0.0, 0.7, -0.7, 0.0, 0.0, 0.7, 0.0];
        let plane_buf = ctx.create_buffer().ok_or("failed to create buffer").unwrap();
        ctx.bind_buffer(WebGlRenderingContext::ARRAY_BUFFER, Some(&plane_buf));
        unsafe {
            let vert_array = js_sys::Float32Array::view(&plane_verts);
        
            ctx.buffer_data_with_array_buffer_view(
                WebGlRenderingContext::ARRAY_BUFFER,
                &vert_array,
                WebGlRenderingContext::STATIC_DRAW,
                );
        }
        //ctx.vertex_attrib_pointer_with_i32(GLC::ARRAY_BUFFER, 3, WebGlRenderingContext::FLOAT, false, 0, 0);
        //ctx.enable_vertex_attrib_array(GLC::ARRAY_BUFFER);
        ctx.vertex_attrib_pointer_with_i32(0, 3, WebGlRenderingContext::FLOAT, false, 0, 0);
        ctx.enable_vertex_attrib_array(0);

        let state_idx = ctx.get_uniform_location(&trail_map_program, "state");
        let create_texture = || -> WebGlTexture {
            //use WebGlRenderingContext::{ TEXTURE_2D, TEXTURE_WRAP_S, TEXTURE_WRAP_T, TEXTURE_MIN_FILTER, TEXTURE_MAG_FILTER, REPEAT, NEAREST };
            // https://nullprogram.com/blog/2014/06/10/
            let tex = ctx.create_texture().expect("couldn't create texture");
            ctx.bind_texture(GLC::TEXTURE_2D, Some(&tex));
            ctx.tex_parameteri(GLC::TEXTURE_2D, GLC::TEXTURE_WRAP_S,     GLC::REPEAT  as i32);// TODO: why need convert, seems sus
            ctx.tex_parameteri(GLC::TEXTURE_2D, GLC::TEXTURE_WRAP_T,     GLC::REPEAT  as i32);
            ctx.tex_parameteri(GLC::TEXTURE_2D, GLC::TEXTURE_MIN_FILTER, GLC::NEAREST as i32);
            ctx.tex_parameteri(GLC::TEXTURE_2D, GLC::TEXTURE_MAG_FILTER, GLC::NEAREST as i32);
            ctx.tex_image_2d_with_i32_and_i32_and_i32_and_format_and_type_and_opt_u8_array(
                GLC::TEXTURE_2D, 0, GLC::LUMINANCE as i32,
                self.size_w as i32, self.size_h as i32,
                0, GLC::RGBA, GLC::UNSIGNED_BYTE, None).expect("couldnt initialize texture");
            tex
        };

        //let step = || {
        //    ctx.bind_framebuffer(GLC::FRAMEBUFFER, framebuffer);
        //    ctx.framebuffer_texture_2d(GLC::FRAMEBUFFER, GLC::COLOR_ATTACHMENT0,
        //                               GLC::TEXTURE_2d, back_texture, 0);
        //    ctx.viewport(0, 0, self.size_w as i32, self.size_h as i32);
        //    ctx.bind_texture(GLC::TEXTURE_2D, front_texture);   // TODO: TEXTURE_2D_ARRAY?
        //    trail_map_program.use()
        //        .attrib('quad', this.buffers.quad, 2)
        //        .uniform('state', 0, true)
        //        .uniform('scale', this.statesize)
        //        .draw(gl.TRIANGLE_STRIP, 4);
        //
        //    mem::swap(front_texture, back_texture);
        //};
        //
        //let draw = || {
        //    ctx.bind_framebuffer(GLC::FRAMEBUFFER, None);
        //    ctx.viewport(0, 0, self.size_w as i32, self.size_h as i32);  
        //    ctx.bind_texture(GLC::TEXTURE_2D, front_texture);
        //
        //    trail_map_program.copy.use() // TODO: a program to copy the state to the display size
        //        .attrib('quad', this.buffers.quad, 2)
        //        .uniform('state', 0, true)
        //        .uniform('scale', this.statesize)
        //        .draw(gl.TRIANGLE_STRIP, 4);
        //};

        ctx.draw_arrays(
            WebGlRenderingContext::TRIANGLE_FAN,
            //WebGlRenderingContext::TRIANGLES,
            0,
            (plane_verts.len() / 3) as i32,
        );
        
        //let vertices: [f32; 9] = [-0.7, -0.7, 0.0, 0.7, -0.7, 0.0, 0.0, 0.7, 0.0];
        //
        //let buffer = ctx.create_buffer().ok_or("failed to create buffer").unwrap();
        //ctx.bind_buffer(WebGlRenderingContext::ARRAY_BUFFER, Some(&buffer));
        //
        //// Note that `Float32Array::view` is somewhat dangerous (hence the
        //// `unsafe`!). This is creating a raw view into our module's
        //// `WebAssembly.Memory` buffer, but if we allocate more pages for ourself
        //// (aka do a memory allocation in Rust) it'll cause the buffer to change,
        //// causing the `Float32Array` to be invalid.
        ////
        //// As a result, after `Float32Array::view` we have to be very careful not to
        //// do any memory allocations before it's dropped.
        //unsafe {
        //    let vert_array = js_sys::Float32Array::view(&vertices);
        //
        //    ctx.buffer_data_with_array_buffer_view(
        //        WebGlRenderingContext::ARRAY_BUFFER,
        //        &vert_array,
        //        WebGlRenderingContext::STATIC_DRAW,
        //        );
        //}
        //
        //
        ////ctx.clear_color(0.0, 0.0, 0.0, 1.0);
        //ctx.clear(WebGlRenderingContext::COLOR_BUFFER_BIT);
        //
        //ctx.draw_arrays(
        //    WebGlRenderingContext::TRIANGLES,
        //    0,
        //    (vertices.len() / 3) as i32,
        //);
    }
}
impl Dish {
    fn update(&mut self, updates: u32) {
        let dist = Uniform::new(0., 1.);
        for agent in &mut self.agents { // NTFS: probably expensive; parallelize
            agent.update(&self.data, self.size_w, self.size_h, self.rng.sample(dist));
        }
        for agent in &self.agents {
            let (y, x, val) = agent.deposit();
            self.data[(y, x)] = self.data[(y, x)].saturating_add(val);
            self.active_cells.push_back((y, x));
        }
        self.diffuse();
        self.decay();
    }
    fn diffuse_nsquared(&mut self) {
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
    fn diffuse(&mut self) {
        // SPFA style
        self.visited.for_each(|x| *x = false); // should hopefully compile to memset: https://users.rust-lang.org/t/fastest-way-to-zero-an-array/39222
        self.data_alt.for_each(|x| *x = 0);

        for c in &self.active_cells {
            self.visited[*c] = true;
        }
        let mut active_next = VecDeque::new();
        loop {
            if let Some((cy, cx)) = self.active_cells.pop_front() {
                let mut sum = 0i32;
                for y in cy-DIFFUSE_RADIUS..=cy+DIFFUSE_RADIUS {
                    for x in cx-DIFFUSE_RADIUS..=cx+DIFFUSE_RADIUS {
                        if !self.visited[(y, x)] && self.data[(cy, cx)] > 0 {
                            self.visited[(y, x)] = true;
                            self.active_cells.push_back((y, x));
                        }
                        sum += self.data[(y, x)] as i32;
                    }
                }
                self.data_alt[(cy, cx)] = (sum / (DIFFUSE_RADIUS * 2 + 1).pow(2)).min(u8::MAX as i32) as u8;
                if self.data_alt[(cy, cx)] > 0 { active_next.push_back((cy, cx)) }
            } else { break }
        }
        swap(&mut self.active_cells, &mut active_next);
        swap(&mut self.data, &mut self.data_alt);
    }
    fn decay_nsquared(&mut self) {
        for y in 0..self.size_h as i32 {
            for x in 0..self.size_w as i32 {
                self.data[(y, x)] = (self.data[(y, x)] as f64 * 0.97) as u8;
            }
        }
    }
    fn decay(&mut self) {
        for c in &self.active_cells {
            self.data[*c] = (self.data[*c] as f64 * 0.97) as u8;
        }
    }
}

// This is like the `main` function, except for JavaScript.
#[wasm_bindgen(start)]
pub fn main_js() -> Result<(), JsValue> {
    // This provides better error messages in debug mode.
    // It'>s disabled in release mode so it doesn't bloat up the file size.
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
    
    //let sim = Dish::new((width/10) as usize, (height/10) as usize);
    //let sim = Dish::new(width as usize, height as usize);
    let sim = Dish::new(WORLD_SIZE.0, WORLD_SIZE.1);
    game_loop(sim, 40, 0.02, |g| {
        // update fn
        g.game.update(g.number_of_updates());
    }, |g| {
        // render fn
        g.game.render_webgl(g.number_of_updates());
        //g.game.render(g.number_of_updates());
    });

    Ok(())
}




// BEGIN YOINK https://rustwasm.github.io/wasm-bindgen/examples/webgl.html
 pub fn compile_shader(
    context: &WebGlRenderingContext,
    shader_type: u32,
    source: &str,
) -> Result<WebGlShader, String> {
    let shader = context
        .create_shader(shader_type)
        .ok_or_else(|| String::from("Unable to create shader object"))?;
    context.shader_source(&shader, source);
    context.compile_shader(&shader);

    if context
        .get_shader_parameter(&shader, WebGlRenderingContext::COMPILE_STATUS)
        .as_bool()
        .unwrap_or(false)
    {
        Ok(shader)
    } else {
        Err(context
            .get_shader_info_log(&shader)
            .unwrap_or_else(|| String::from("Unknown error creating shader")))
    }
}

pub fn link_program(
    context: &WebGlRenderingContext,
    vert_shader: &WebGlShader,
    frag_shader: &WebGlShader,
) -> Result<WebGlProgram, String> {
    let program = context
        .create_program()
        .ok_or_else(|| String::from("Unable to create shader object"))?;

    context.attach_shader(&program, vert_shader);
    context.attach_shader(&program, frag_shader);
    context.link_program(&program);

    if context
        .get_program_parameter(&program, WebGlRenderingContext::LINK_STATUS)
        .as_bool()
        .unwrap_or(false)
    {
        Ok(program)
    } else {
        Err(context
            .get_program_info_log(&program)
            .unwrap_or_else(|| String::from("Unknown error creating program object")))
    }
}
// END YOINK
