use raylib::prelude::*;
use std::sync::mpsc;
use std::thread;
use std::ops::Add;

#[derive(Clone, Copy, Default)]
struct Complex {
    real: f64,
    imag: f64,
}

#[derive(Copy, Clone, Default)]
struct Pixel {
    x: i32,
    y: i32,
    escapes: i32,
}

#[derive(Clone, Copy)]
struct ScreenInfo {
    x_start: f64,
    x_stop: f64,
    y_start: f64,
    y_stop: f64,
    pixels_per_cm: f64,
    screen_width: i32,
    screen_height: i32,
}

impl ScreenInfo {
    fn zoom(&mut self, how_many_times: f64, mouse_pos: Vector2) {
        let view_width = self.x_stop - self.x_start;
        let view_height = self.y_stop - self.y_start;

        let mouse_world_x = self.x_start + mouse_pos.x as f64 / self.screen_width as f64 * view_width;
        let mouse_world_y = self.y_start + mouse_pos.y as f64 / self.screen_height as f64 * view_height;

        let new_width = view_width / how_many_times;
        let new_height = view_height / how_many_times;

        self.x_start = mouse_world_x - new_width / 2.0;
        self.x_stop = mouse_world_x + new_width / 2.0;

        self.y_start = mouse_world_y - new_height / 2.0;
        self.y_stop = mouse_world_y + new_height / 2.0;
    }
}

impl From<(f64, f64, f64, f64, f64)> for ScreenInfo {
    fn from(values: (f64, f64, f64, f64, f64)) -> Self {
        let (x_start, x_stop, y_start, y_stop, pixels_per_cm) = values;

        ScreenInfo {
            x_start,
            x_stop,
            y_start,
            y_stop,
            pixels_per_cm,

            screen_width: (pixels_per_cm * (x_stop - x_start)) as i32,
            screen_height: (pixels_per_cm * (y_stop - y_start)) as i32,
        }
    }
}

const MAX_THREADS: i32 = 64;
const ACCURACY: i32 = 2;
const ITERS: i32 = 10000;

impl Complex {
    fn square(&mut self) {
        let real_part = self.real * self.real - self.imag * self.imag;
        let imag_part = (self.real + self.real) * self.imag;

        self.real = real_part;
        self.imag = imag_part;
    }

    fn mag(&self) -> f64 {
        self.imag * self.imag + self.real * self.real
    }
}

impl Add for Complex {
    type Output = Complex;
    fn add(self, other: Complex) -> Complex {
        Complex {
            real: self.real + other.real,
            imag: self.imag + other.imag,
        }
    }
}

fn main() {
    let mut screen = ScreenInfo::from((-3.0, 2.0, -2.0, 2.0, 200.0));

    let (mut rl_handle, thread) = init()
        .size(screen.screen_width,screen.screen_height)
        .build();

    while !rl_handle.window_should_close() {
        let mouse_wheel_move = rl_handle.get_mouse_wheel_move();

        if mouse_wheel_move != 0.0 {
            screen.zoom(if mouse_wheel_move > 0.0 { 1.25 } else {0.75}, rl_handle.get_mouse_position());
        }

        let mut draw_handle = rl_handle.begin_drawing(&thread);
        draw_handle.clear_background(Color::BLACK);

        let mandelbrod = mandelbrod(screen);
        draw_pixel_mandelbrod(&mandelbrod[..], &mut draw_handle);

        let fps = draw_handle.get_fps();
        draw_handle.draw_text(format!("fps: {}", fps).as_str(), 3, 3, 10, Color::WHEAT);

    }
}

fn draw_pixel_mandelbrod(p: &[Pixel], draw_handle: &mut RaylibDrawHandle) {
    p.iter().for_each(|p| {
        let alpha: f32 = if p.escapes < 1 {
            0.0
        } else {
            p.escapes.ilog2() as f32 / ITERS.ilog2() as f32
            //p.escapes as f32 / ITERS as f32
        };

        let color_shade = (alpha * 255.0) as u8;

        draw_handle.draw_rectangle(
            p.x,
            p.y,
            ACCURACY,
            ACCURACY,
            Color::new(color_shade, color_shade, color_shade, 255),
        );
    });
}
fn belongs_to_set(c: Complex, p: &mut Pixel) {
    let mut z: Complex = Default::default();
    for i in 0..ITERS {
        if z.mag() > 16.0 {
            p.escapes = i;
            return;
        }
        z.square();
        z = z + c;
    }
    p.escapes = 0;
}


fn mandelbrod(screen: ScreenInfo) -> Vec<Pixel> {
    let mut threads: Vec<thread::JoinHandle<()>> = Vec::with_capacity(MAX_THREADS as usize + 1);
    let rows_per_thread = (screen.screen_height as f32 / MAX_THREADS as f32).ceil() as i32;

    let (tx, rx) = mpsc::channel();

    for i in 0..=MAX_THREADS {
        let tx = tx.clone();
        threads.push(thread::spawn(move || {
            let mut temp_data = Vec::with_capacity((rows_per_thread * screen.screen_width) as usize);
            let start_y = i * rows_per_thread;
            let end_y = (i + 1) * rows_per_thread;

            for y in (start_y..end_y).step_by(ACCURACY as usize) {
                for x in (0..screen.screen_width).step_by(ACCURACY as usize) {
                    let c = Complex {
                        real: screen.x_start + x as f64 / screen.screen_width as f64 * (screen.x_stop - screen.x_start),
                        imag: screen.y_start + y as f64 / screen.screen_height as f64 * (screen.y_stop - screen.y_start),
                    };

                    let mut p = Pixel { x, y, escapes: 0 };

                    belongs_to_set(c, &mut p);
                    temp_data.push(p);
                }
            }
            tx.send(temp_data).unwrap();
        }))
    }

    let mut canvas: Vec<Pixel> = Vec::with_capacity((screen.screen_width * screen.screen_height) as usize);
    drop(tx);
    for rec in rx {
        canvas.extend(rec);
    }

    for thread in threads {
        thread.join().unwrap();
    }

    canvas
}
