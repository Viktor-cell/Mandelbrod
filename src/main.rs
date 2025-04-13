use raylib::prelude::*;
use std::sync::mpsc;
use std::thread;

struct Complex {
    real: f64,
    imag: f64,
}

#[derive(Copy, Clone)]
struct Pixel {
    x: i32,
    y: i32,
    escapes: i32,
}

const MAX_THREADS: i32 = 64;
const ACCURACY: i32 = 1;
const ITERS: i32 = 100;

impl Complex {
    fn add(&mut self, complex: &Complex) {
        self.real += complex.real;
        self.imag += complex.imag;
    }

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

fn main() {
    let x_start = -3.0;
    let x_stop = 2.0;
    let y_start = -2.0;
    let y_stop = 2.0;
    let screen_part = 200.0;

    let (mut rl_handle, thread) = init()
        .size(
            (screen_part * (x_stop - x_start)) as i32,
            (screen_part * (y_stop - y_start)) as i32,
        )
        .build();

    while !rl_handle.window_should_close() {
        let mut draw_handle = rl_handle.begin_drawing(&thread);
        let fps = draw_handle.get_fps();

        draw_handle.clear_background(Color::BLACK);

        let mandelbrod = mandelbrod(
            (screen_part * (x_stop - x_start)) as i32,
            (screen_part * (y_stop - y_start)) as i32,
            x_start,
            x_stop,
            y_start,
            y_stop,
        );

        draw_pixel_mandelbrod(&mandelbrod[..], &mut draw_handle);

        draw_handle.draw_text(format!("fps: {}", fps).as_str(), 3, 3, 10, Color::WHEAT);
    }
}

fn draw_pixel_mandelbrod(p: &[Pixel], draw_handle: &mut RaylibDrawHandle) {
    p.iter().for_each(|p| {
        let alpha: f32 = if p.escapes < 1 {
            0.0
        } else {
            //p.escapes.ilog2() as f32 / ITERS.ilog2() as f32
            p.escapes as f32 / ITERS as f32
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
    let mut z: Complex = Complex {
        real: 0.0,
        imag: 0.0,
    };
    for i in 0..ITERS {
        if z.mag() > 16.0 {
            p.escapes = i;
            return;
        }
        z.square();
        z.add(&c);
    }
    p.escapes = 0;
}

// mpsc
fn mandelbrod(
    scr_w: i32,
    scr_h: i32,
    x_start: f64,
    x_stop: f64,
    y_start: f64,
    y_stop: f64,
) -> Vec<Pixel> {
    let mut threads: Vec<thread::JoinHandle<()>> = Vec::new();
    let rows_per_thread = (scr_h as f32 / MAX_THREADS as f32).ceil() as i32;

    let (tx, rx) = mpsc::channel();

    for i in 0..=MAX_THREADS {
        let tx_clone = tx.clone();
        threads.push(thread::spawn(move || {

            let mut temp_data = vec![];

            let start_y = i * rows_per_thread;
            let end_y = (i + 1) * rows_per_thread;

            for y in (start_y..end_y).step_by(ACCURACY as usize) {

                for x in (0..scr_w).step_by(ACCURACY as usize) {

                    let c = Complex {
                        real: x_start + x as f64 / scr_w as f64 * (x_stop - x_start),
                        imag: y_start + y as f64 / scr_h as f64 * (y_stop - y_start),
                    };

                    let mut p = Pixel { x, y, escapes: 0 };

                    belongs_to_set(c, &mut p);
                    temp_data.push(p);
                }
            }
            tx_clone.send(temp_data).unwrap();
        }))
    }

    let mut canvas: Vec<Pixel> = Vec::new();
    drop(tx);
    for rec in rx {
        canvas.extend(rec);
    }

    for thread in threads {
        thread.join().unwrap();
    }

    canvas
}
