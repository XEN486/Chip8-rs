mod cpu;
mod display;
mod keypad;
mod rle;

use cpu::Chip8;
use display::Display;

fn main() {
    println!("Hello, world!");
    let mut display: Display = Display::new(64, 32, 26);
    display.init_renderer();

    let mut cpu: Chip8 = Chip8::new("font.bin", "bigfont.bin", "test.ch8", display, None);
    cpu.run(std::time::Duration::from_nanos(1_428_571)); // run the CPU at 700hz
}