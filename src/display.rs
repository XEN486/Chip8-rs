use crate::keypad::Keypad;
use sdl2::audio::{AudioCallback, AudioDevice, AudioSpecDesired};
use sdl2::pixels::Color;
use sdl2::render::Canvas;
use sdl2::video::Window;
use sdl2::EventPump;
use sdl2::event::Event;

struct DisplaySDL {
    canvas: Option<Canvas<Window>>,
    event_pump: Option<EventPump>,
    audio_device: Option<AudioDevice<SquareWave>>,
    window: Option<Window>,
}

impl DisplaySDL {
    pub fn new() -> DisplaySDL {
        DisplaySDL {
            canvas: None,
            event_pump: None,
            audio_device: None,
            window: None,
        }
    }
}

pub struct Display {
    pub display: Vec<u32>,  // Each u32 holds 32 pixels (1 bit per pixel)
    pub width: u16,
    pub height: u16,
    pub keypad: Keypad,
    pub scale: u16,
    pub original_scale: u16,
    beep: bool,
    sdl: DisplaySDL,
}

struct SquareWave {
    phase_inc: f32,
    phase: f32,
    volume: f32,
}

impl AudioCallback for SquareWave {
    type Channel = f32;

    fn callback(&mut self, out: &mut [Self::Channel]) {
        for x in out.iter_mut() {
            self.phase = (self.phase + self.phase_inc) % 1.0;
            *x = if self.phase < 0.5 { self.volume } else { -self.volume };
        }
    }
}

impl Display {
    pub fn new(width: u16, height: u16, scale: u16) -> Display {
        Display {
            display: vec![0; ((width * height) as usize + 31) / 32],  // Initialize with enough u32s to hold all bits
            width,
            height,
            keypad: Keypad::new(),
            scale,
            original_scale: scale,
            beep: false,
            sdl: DisplaySDL::new(),
        }
    }

    pub fn clear(&mut self) {
        let num_u32s = ((self.width * self.height) as usize + 31) / 32;
        self.display = vec![0; num_u32s];  // 32 bits per u32
    }

    pub fn get_pixel(&self, x: u16, y: u16) -> u8 {
        let index = (y * self.width + x) as usize;
        let u32_index = index / 32;
        let bit_index = index % 32;

        // Shift the bit into the least significant bit and mask with 1
        ((self.display[u32_index] >> (31 - bit_index)) & 1) as u8
    }

    pub fn set_pixel(&mut self, x: u16, y: u16, v: u8) {
        let index = (y * self.width + x) as usize;
        let u32_index = index / 32;
        let bit_index = index % 32;

        if v == 1 {
            self.display[u32_index] |= 1 << (31 - bit_index); // Set bit
        } else {
            self.display[u32_index] &= !(1 << (31 - bit_index)); // Clear bit
        }
    }

    pub fn init_renderer(&mut self) {
        let sdl_context = sdl2::init().unwrap();
        let video_subsystem = sdl_context.video().unwrap();
        let audio_subsystem = sdl_context.audio().unwrap();

        let window = video_subsystem
            .window(
                "Rust Chip-8",
                (self.width * self.scale) as u32,
                (self.height * self.scale) as u32,
            )
            .position_centered()
            .build()
            .unwrap();

        let canvas = window.into_canvas().build().unwrap();
        self.sdl.window = Some(canvas.window().clone());
        self.sdl.event_pump = Some(sdl_context.event_pump().unwrap());
        self.sdl.canvas = Some(canvas);

        let spec = AudioSpecDesired {
            freq: Some(44100),
            channels: Some(1),
            samples: None,
        };

        let audio_device = audio_subsystem.open_playback(None, &spec, |spec| {
            SquareWave {
                phase_inc: 440.0 / spec.freq as f32,
                phase: 0.0,
                volume: 0.05,
            }
        }).unwrap();

        self.sdl.audio_device = Some(audio_device);
    }

    pub fn set_beep(&mut self, flag: bool) {
        if self.beep == flag {
            return;
        }

        self.beep = flag;

        if let Some(ref audio_device) = self.sdl.audio_device {
            if self.beep {
                audio_device.resume();
            } else {
                audio_device.pause();
            }
        }
    }

    pub fn event_loop(&mut self) -> bool {
        if let Some(ref mut event_pump) = self.sdl.event_pump {
            for event in event_pump.poll_iter() {
                match event {
                    Event::Quit { .. } => return true,
                    Event::KeyDown { keycode: Some(key), .. } => self.keypad.key_down(key),
                    Event::KeyUp { keycode: Some(key), .. } => self.keypad.key_up(key),
                    _ => {}
                }
            }
        }

        false
    }

    pub fn resize(&mut self, new_width: u16, new_height: u16, new_scale: u16) {
        self.width = new_width;
        self.height = new_height;
        self.scale = new_scale;
        let num_u32s = ((self.width * self.height) as usize + 31) / 32;
        self.display = vec![0; num_u32s];

        if let Some(ref mut window) = self.sdl.window {
            window
                .set_size((self.width * self.scale) as u32, (self.height * self.scale) as u32)
                .unwrap();
        }
    }

    pub fn shift_up(&mut self) {
        for y in 0..self.height - 1 {
            for x in 0..self.width {
                let idx_current = (y * self.width + x) as usize;
                let idx_next = ((y + 1) * self.width + x) as usize;
                let u32_index_current = idx_current / 32;
                let u32_index_next = idx_next / 32;
                let bit_index_current = idx_current % 32;
                let bit_index_next = idx_next % 32;

                let pixel_next = (self.display[u32_index_next] >> (31 - bit_index_next)) & 1;
                if pixel_next == 1 {
                    self.display[u32_index_current] |= 1 << (31 - bit_index_current);
                } else {
                    self.display[u32_index_current] &= !(1 << (31 - bit_index_current));
                }
            }
        }

        // Clear the last row
        for x in 0..self.width {
            let idx = ((self.height - 1) * self.width + x) as usize;
            let u32_index = idx / 32;
            let bit_index = idx % 32;
            self.display[u32_index] &= !(1 << (31 - bit_index));
        }
    }

    pub fn shift_down(&mut self) {
        for y in (1..self.height).rev() {
            for x in 0..self.width {
                let idx_current = (y * self.width + x) as usize;
                let idx_previous = ((y - 1) * self.width + x) as usize;
                let u32_index_current = idx_current / 32;
                let u32_index_previous = idx_previous / 32;
                let bit_index_current = idx_current % 32;
                let bit_index_previous = idx_previous % 32;

                let pixel_previous = (self.display[u32_index_previous] >> (31 - bit_index_previous)) & 1;
                if pixel_previous == 1 {
                    self.display[u32_index_current] |= 1 << (31 - bit_index_current);
                } else {
                    self.display[u32_index_current] &= !(1 << (31 - bit_index_current));
                }
            }
        }

        // Clear the first row
        for x in 0..self.width {
            let idx = (x) as usize;
            let u32_index = idx / 32;
            let bit_index = idx % 32;
            self.display[u32_index] &= !(1 << (31 - bit_index));
        }
    }

    pub fn shift_left(&mut self) {
        for y in 0..self.height {
            for x in 0..self.width - 1 {
                let idx_current = (y * self.width + x) as usize;
                let idx_next = (y * self.width + (x + 1)) as usize;
                let u32_index_current = idx_current / 32;
                let u32_index_next = idx_next / 32;
                let bit_index_current = idx_current % 32;
                let bit_index_next = idx_next % 32;

                let pixel_next = (self.display[u32_index_next] >> (31 - bit_index_next)) & 1;
                if pixel_next == 1 {
                    self.display[u32_index_current] |= 1 << (31 - bit_index_current);
                } else {
                    self.display[u32_index_current] &= !(1 << (31 - bit_index_current));
                }
            }
        }

        // Clear the last column
        for y in 0..self.height {
            let idx = (y * self.width + (self.width - 1)) as usize;
            let u32_index = idx / 32;
            let bit_index = idx % 32;
            self.display[u32_index] &= !(1 << (31 - bit_index));
        }
    }

    pub fn shift_right(&mut self) {
        for y in 0..self.height {
            for x in (1..self.width).rev() {
                let idx_current = (y * self.width + x) as usize;
                let idx_previous = (y * self.width + (x - 1)) as usize;
                let u32_index_current = idx_current / 32;
                let u32_index_previous = idx_previous / 32;
                let bit_index_current = idx_current % 32;
                let bit_index_previous = idx_previous % 32;

                let pixel_previous = (self.display[u32_index_previous] >> (31 - bit_index_previous)) & 1;
                if pixel_previous == 1 {
                    self.display[u32_index_current] |= 1 << (31 - bit_index_current);
                } else {
                    self.display[u32_index_current] &= !(1 << (31 - bit_index_current));
                }
            }
        }

        // Clear the first column
        for y in 0..self.height {
            let idx = (y * self.width) as usize;
            let u32_index = idx / 32;
            let bit_index = idx % 32;
            self.display[u32_index] &= !(1 << (31 - bit_index));
        }
    }

    pub fn draw(&mut self) {
        if let Some(ref mut canvas) = self.sdl.canvas {
            canvas.set_draw_color(Color::BLACK);
            canvas.clear();

            let mut prev_pixel = 255;
            for y in 0..self.height {
                for x in 0..self.width {
                    let idx = (y * self.width + x) as usize;
                    let u32_index = idx / 32;
                    let bit_index = idx % 32;
                    let pixel = ((self.display[u32_index] >> (31 - bit_index)) & 1) as u8;

                    if pixel != prev_pixel {
                        canvas.set_draw_color(if pixel == 1 { Color::WHITE } else { Color::BLACK });
                        prev_pixel = pixel;
                    }

                    let rect = sdl2::rect::Rect::new(
                        (x * self.scale) as i32,
                        (y * self.scale) as i32,
                        self.scale as u32,
                        self.scale as u32,
                    );
                    canvas.fill_rect(rect).unwrap();
                }
            }

            canvas.present();
        }
    }
}
