use crate::display::Display;
use crate::keypad::Key;
use rand::random;

use std::time::Duration;
use std::fs::File;
use std::io::{self, Read, Write};
use crate::rle::{encode_rle, decode_rle, encode_rle_u32, decode_rle_u32};

struct Timers {
    pub delay: u8,
    pub sound: u8,
}

struct Registers {
    pub pc: u16,
    pub sp: i8,
    pub i: u16,
    pub v: [u8; 16],
}

struct Instruction {
    pub raw: u16,
    pub op: u8,
    pub x: u8,
    pub y: u8,
    pub n: u8,
    pub nn: u8,
    pub nnn: u16,
}

pub struct Quirks {
    pub cosmac_shift: bool,
    pub cosmac_fx1e: bool,
    pub cosmac_fx55: bool,
    pub cosmac_bnnn: bool,
}

pub struct Chip8 {
    registers: Registers,
    timers: Timers,
    stack: [u16; 32],
    memory: [u8; 0xFFFF],
    display: Display,
    quirks: Quirks,
}

impl Chip8 {
    // Creates a new Chip8 instance
    pub fn new(font_path: &str, bigfont_path: &str, program_path: &str, display: Display, quirks: Option<Quirks>) -> Chip8 {
        let quirks: Quirks = quirks.unwrap_or_else(|| Quirks {
            cosmac_shift: false, // Chip8: TRUE
            cosmac_fx1e: false, // Chip8: FALSE
            cosmac_fx55: false, // Chip8: FALSE
            cosmac_bnnn: false, // Chip8: TRUE
        });

        let mut cpu: Chip8 = Chip8 {
            registers: Registers {
                pc: 0x200,
                sp: -1,
                i: 0,
                v: [0; 16],
            },
            timers: Timers {
                delay: 0,
                sound: 0,
            },
            stack: [0; 32],
            memory: [0; 0xFFFF],
            display,
            quirks,
        };

        _ = cpu.read_to_memory(font_path, 0);
        _ = cpu.read_to_memory(bigfont_path, 0x50);
        _ = cpu.read_to_memory(program_path, 0x200);
        cpu
    }

    // Fetch two bytes for memory (an instruction is two bytes)
    fn fetch(&mut self) -> u16 {
        let fetched: u16 = (self.memory[self.registers.pc as usize] as u16) << 8 | self.memory[self.registers.pc as usize + 1] as u16;
        self.registers.pc += 2;

        fetched
    }

    // Decodes a u16 into an Instruction
    fn decode(&self, instruction: u16) -> Instruction {
        Instruction {
            raw: instruction,
            op: (instruction >> 12) as u8,
            x: ((instruction & 0x0F00) >> 8) as u8,
            y: ((instruction & 0x00F0) >> 4) as u8,
            n: (instruction & 0x000F) as u8,
            nn: (instruction & 0x00FF) as u8,
            nnn: instruction & 0x0FFF,
        }
    }

    // Executes an Instruction
    fn execute(&mut self, instruction: Instruction) {
        match instruction.op {
            0x0 => match instruction.raw {
                0x00E0 => self.display.clear(),

                0x00EE => {
                    self.registers.pc = self.stack[self.registers.sp as usize];
                    self.registers.sp -= 1;
                }

                0x00FF => self.display.resize(128, 64, self.display.original_scale / 2),
                0x00FE => self.display.resize(64, 32, self.display.original_scale),
                0x00FB => for _ in 0..4 { self.display.shift_right(); },
                0x00FC => for _ in 0..4 { self.display.shift_left(); },

                0x00FD => {
                    self.registers.pc -= 2;
                },

                _ => if instruction.raw & 0xFFF0 == 0x00C0 {
                    for _ in 0..instruction.raw & 0x000F {
                        self.display.shift_down();
                    }
                } else if instruction.raw & 0x00F0 == 0x00D0 {
                    for _ in 0..instruction.raw & 0x000F {
                        self.display.shift_up();
                    }
                }
            },

            0x1 => self.registers.pc = instruction.nnn,

            0x2 => {
                self.registers.sp += 1;
                self.stack[self.registers.sp as usize] = self.registers.pc;
                self.registers.pc = instruction.nnn;
            }

            0x3 => if self.registers.v[instruction.x as usize] == instruction.nn {
                self.registers.pc += 2;
            }

            0x4 => if self.registers.v[instruction.x as usize] != instruction.nn {
                self.registers.pc += 2;
            }

            0x5 => match instruction.raw & 0x000F {
                0x2 => _ = self.write_flags(instruction.x as usize, instruction.y as usize),
                0x3 => _ = self.read_flags(instruction.x as usize, instruction.y as usize),

                _ => if self.registers.v[instruction.x as usize] == self.registers.v[instruction.y as usize] {
                    self.registers.pc += 2;
                }
            }

            0x6 => self.registers.v[instruction.x as usize] = instruction.nn,
            0x7 => self.registers.v[instruction.x as usize] = self.registers.v[instruction.x as usize].wrapping_add(instruction.nn),

            0x8 => match instruction.raw & 0x000F {
                0x0 => self.registers.v[instruction.x as usize] = self.registers.v[instruction.y as usize],
                0x1 => self.registers.v[instruction.x as usize] |= self.registers.v[instruction.y as usize],
                0x2 => self.registers.v[instruction.x as usize] &= self.registers.v[instruction.y as usize],
                0x3 => self.registers.v[instruction.x as usize] ^= self.registers.v[instruction.y as usize],
                
                0x4 => {
                    let mut added: u16 = self.registers.v[instruction.x as usize] as u16 + self.registers.v[instruction.y as usize] as u16;
                    self.convert_with_carry(&mut added);
                    self.registers.v[instruction.x as usize] = added as u8;
                }

                0x5 => {
                    let x = self.registers.v[instruction.x as usize];
                    let y = self.registers.v[instruction.y as usize];
                    self.registers.v[instruction.x as usize] = x.wrapping_sub(y);
                    self.registers.v[0xF] = (x >= y) as u8;
                }

                0x6 => {
                    if self.quirks.cosmac_shift {
                        self.registers.v[instruction.x as usize] = self.registers.v[instruction.y as usize];
                    }

                    self.registers.v[0xf] = self.registers.v[instruction.x as usize] & 1;
                    self.registers.v[instruction.x as usize] >>= 1;
                }

                0x7 => {
                    let x = self.registers.v[instruction.x as usize];
                    let y = self.registers.v[instruction.y as usize];
                    self.registers.v[instruction.x as usize] = y.wrapping_sub(x);
                    self.registers.v[0xF] = (y >= x) as u8;
                }

                0xE => {
                    if self.quirks.cosmac_shift {
                        self.registers.v[instruction.x as usize] = self.registers.v[instruction.y as usize];
                    }

                    self.registers.v[instruction.x as usize] <<= 1;
                    self.registers.v[0xf] = self.registers.v[instruction.x as usize] & 1;
                }

                _ => self.unknown(instruction),
            }

            0x9 => if self.registers.v[instruction.x as usize] != self.registers.v[instruction.y as usize] {
                self.registers.pc += 2;
            }

            0xA => self.registers.i = instruction.nnn,
            0xB => {
                if self.quirks.cosmac_bnnn {
                    self.registers.pc = instruction.nnn + self.registers.v[0] as u16;
                } else {
                    self.registers.pc = instruction.nnn + self.registers.v[instruction.x as usize] as u16;
                }
            },
            0xC => self.registers.v[instruction.x as usize] = random::<u8>() & instruction.nn,
            0xD => self.draw_sprite(instruction),

            0xE => match instruction.raw & 0x00FF {
                0x9E => if self.display.keypad.keypad[self.registers.v[instruction.x as usize] as usize] {
                    self.registers.pc += 2;
                }

                0xA1 => if !self.display.keypad.keypad[self.registers.v[instruction.x as usize] as usize] {
                    self.registers.pc += 2;
                }

                _ => self.unknown(instruction),
            }

            0xF => match instruction.raw & 0x00FF {
                0x07 => self.registers.v[instruction.x as usize] = self.timers.delay,
                0x15 => self.timers.delay = self.registers.v[instruction.x as usize],
                0x18 => self.timers.sound = self.registers.v[instruction.x as usize],

                0x1E => {
                    self.registers.i += self.registers.v[instruction.x as usize] as u16;

                    if self.registers.i > 0xFFF {
                        self.registers.i %= 0xFFF;

                        if !self.quirks.cosmac_fx1e {
                            self.registers.v[0xf] = 1;
                        }
                    }
                }
                
                0x0A => if !self.display.keypad.new_key_pressed {
                    self.registers.pc -= 2;
                    return;
                }

                0x29 => self.registers.i = self.registers.v[instruction.x as usize] as u16 * 5,
                0x30 => self.registers.i = 0x50 + (self.registers.v[instruction.x as usize] as u16 * 10),
                
                0x33 => {
                    let value = self.registers.v[instruction.x as usize];
                    
                    self.memory[self.registers.i as usize] = value / 100;
                    self.memory[self.registers.i as usize + 1] = (value / 10) % 10;
                    self.memory[self.registers.i as usize + 2] = value % 10;
                }

                0x55 => {
                    let upper_bound: usize = (instruction.x as usize + 1).min(self.registers.v.len());
                    for i in 0..upper_bound {
                        self.memory[self.registers.i as usize + i] = self.registers.v[i];
                    }
                
                    if self.quirks.cosmac_fx55 {
                        self.registers.i += instruction.x as u16 + 1;
                    }
                }
                
                0x65 => {
                    let upper_bound: usize = (instruction.x as usize + 1).min(self.registers.v.len());
                    for i in 0..upper_bound {
                        self.registers.v[i] = self.memory[self.registers.i as usize + i];
                    }
                
                    if self.quirks.cosmac_fx55 {
                        self.registers.i += instruction.x as u16 + 1;
                    }
                }

                0x75 => _ = self.write_flags(0, instruction.x as usize),
                0x85 => _ = self.read_flags(0, instruction.y as usize),

                _ => match instruction.raw & 0xF000 {
                    0x000 => {
                        self.registers.i = self.fetch();
                    }
                    _ => self.unknown(instruction),
                }
            }

            _ => self.unknown(instruction),
        }
    }

    // Converts U16 -> U8 and sets VF as carry
    fn convert_with_carry(&mut self, value: &mut u16) {
        if *value >= 0x100 {
            self.registers.v[0xF] = 1;
            *value &= 0xFF;
        } else {
            self.registers.v[0xF] = 0;
        }
    }

    // Unknown instruction callback
    fn unknown(&self, instruction: Instruction) {
        println!("unknown instruction: {:#06X}", instruction.raw);
    }

    // Dump CPU state to file
    pub fn save_state(&self, path: &str) -> io::Result<()> {
        let mut file: File = File::create(path)?;
        file.write_all("HEAD".as_bytes())?;
        file.write(&[1, 0, 0])?; // file format version
    
        file.write_all("REGS".as_bytes())?; // registers header
        file.write_all(&self.registers.v)?;
        file.write_all(&self.registers.i.to_le_bytes())?;
        file.write_all(&self.registers.pc.to_le_bytes())?;
        file.write_all(&self.registers.sp.to_le_bytes())?;
    
        file.write_all("TIME".as_bytes())?; // timer header
        file.write(&[self.timers.delay])?;
        file.write(&[self.timers.sound])?;
    
        file.write_all("STCK".as_bytes())?; // stack header
        for &num in &self.stack {
            file.write_all(&num.to_le_bytes())?;
        }
    
        file.write_all("RMEM".as_bytes())?; // RLE memory header
        let encoded_memory = encode_rle(&self.memory);
        file.write_all(&encoded_memory)?;

        file.write_all("DISP".as_bytes())?; // display header
        file.write_all(&self.display.width.to_le_bytes())?;
        file.write_all(&self.display.height.to_le_bytes())?;
        file.write_all(&self.display.display.len().to_le_bytes())?;
    
        let encoded_display = encode_rle_u32(&self.display.display);
        file.write_all(&encoded_display)?;
    
        Ok(())
    }

    // Read CPU state from file
    pub fn load_state(&mut self, path: &str) -> io::Result<()> {
        Ok(())
    }

    // Write Vx-Vy -> flags
    fn write_flags(&self, x: usize, y: usize) -> io::Result<()> {
        let mut file = File::create("flags.bin")?;
        file.write_all(&self.registers.v[x..y])?;
        Ok(())
    }

    // Read flags -> Vx-Vy
    fn read_flags(&mut self, x: usize, y: usize) -> io::Result<()> {
        let mut file = File::open("flags.bin")?;
        let mut buffer = &mut self.registers.v[x..y];
        file.read_exact(&mut buffer)?;

        Ok(())
    }

    // DXY0 implementation
    fn draw_dxy0(&mut self, instruction: Instruction) {
        let x: u16 = self.registers.v[instruction.x as usize] as u16 % self.display.width;
        let mut y: u16 = self.registers.v[instruction.y as usize] as u16 % self.display.height;
        self.registers.v[0xF] = 0;

        for i in 0..16 {
            let word: u16 = ((self.memory[self.registers.i as usize + (i * 2)] as u16) << 8) |  (self.memory[self.registers.i as usize + (i * 2) + 1] as u16);
            let mut row_x: u16 = x;
    
            for i in 0..16 {
                if row_x >= self.display.width {
                    break;
                }
    
                let bit = (word >> (15 - i)) & 1;
                if bit == 1 {
                    if self.display.get_pixel(row_x, y) == 1 {
                        self.display.set_pixel(row_x , y, 0);
                        self.registers.v[0xF] = 1;
                    } else {
                        self.display.set_pixel(row_x, y, 1);
                    }
                }
                row_x += 1;
            }
    
            y += 1;
            if y >= self.display.height {
                break;
            }
        }
    }

    // DXYN implementation
    fn draw_sprite(&mut self, instruction: Instruction) {
        if instruction.n == 0 {
            self.draw_dxy0(instruction);
            return;
        }

        let x: u8 = self.registers.v[instruction.x as usize] % self.display.width as u8;
        let mut y: u8 = self.registers.v[instruction.y as usize] % self.display.height as u8;
        
        self.registers.v[0xF] = 0;
    
        for row in 0..instruction.n {
            let byte: u8 = self.memory[(self.registers.i + row as u16) as usize];
    
            let mut row_x = x;
    
            for i in 0..8 {
                if row_x >= self.display.width as u8 {
                    break;
                }
    
                let bit = (byte >> (7 - i)) & 1;
                if bit == 1 {
                    if self.display.get_pixel(row_x as u16, y as u16) == 1 {
                        self.display.set_pixel(row_x as u16, y as u16, 0);
                        self.registers.v[0xF] = 1;
                    } else {
                        self.display.set_pixel(row_x as u16, y as u16, 1);
                    }
                }
                row_x += 1;
            }
    
            y += 1;
            if y >= self.display.height as u8 {
                break;
            }
        }
    }

    // Reads a file into memory at an address
    pub fn read_to_memory(&mut self, file_path: &str, address: u16) -> io::Result<()> {
        let mut file = File::open(file_path)?;
        
        let buffer = &mut self.memory[address as usize..];
        file.read_exact(buffer)?;
    
        Ok(())
    }

    // Runs one step of the Chip8 emulator
    pub fn step(&mut self) {
        let word: u16 = self.fetch();
        let instruction: Instruction = self.decode(word);
        self.execute(instruction);
    }

    // Runs the Chip-8 emulator forever
    pub fn run(&mut self, cpu_target: Duration) {
        let mut last_timer_tick = std::time::Instant::now();
        let mut last_cpu_tick = std::time::Instant::now();
    
        let timer_target = Duration::from_millis(16); // 60 Hz
        let cpu_cycle_duration = cpu_target;         // CPU cycle duration (e.g., 700Hz)
    
        loop {
            let now = std::time::Instant::now();
    
            let next_cpu_tick = last_cpu_tick + cpu_cycle_duration;
            let next_timer_tick = last_timer_tick + timer_target;

            if now >= next_cpu_tick {
                self.step();
                last_cpu_tick = next_cpu_tick; // Update to the next target time
            }

            if now >= next_timer_tick {
                self.timers.delay = self.timers.delay.saturating_sub(1);
                if self.timers.sound > 0 {
                    self.display.set_beep(true);
                    self.timers.sound -= 1;
                } else {
                    self.display.set_beep(false);
                }

                last_timer_tick = next_timer_tick; // Update to the next target time
            }
    
            // Handle events and redraw display
            if self.display.event_loop() {
                break;
            }
            if self.display.keypad.check_key_down_and_reset(Key::KpPeriod) {
                let _ = self.save_state("savestate.sav");
                println!("wrote savestate!");
            } else if self.display.keypad.check_key_down_and_reset(Key::KpEnter) {
                let _ = self.load_state("savestate.sav");
                println!("read savestate!");
            }
            self.display.draw();
    
            // Avoid busy-waiting
            std::thread::yield_now();
        }
    }
}