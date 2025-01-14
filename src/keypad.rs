use sdl2::keyboard::Keycode;
pub type Key = Keycode;

pub struct Keypad {
    pub keypad: [bool; 16],
    pub new_key_pressed: bool,
    pub last_key: Option<Keycode>,
}

impl Keypad {
    pub fn new() -> Keypad {
        Keypad {
            keypad: [false; 16],
            new_key_pressed: false,
            last_key: None,
        }
    }

    pub fn check_key_down_and_reset(&mut self, key: Key) -> bool {
        if self.last_key.is_some() && self.last_key.unwrap() == key {
            self.last_key = None;
            return true;
        }
        false
    }

    pub fn key_down(&mut self, key: Keycode) {
        self.new_key_pressed = true;
        self.last_key = Some(key);

        match key {
            Keycode::Num1 => self.keypad[0x1] = true,
            Keycode::Num2 => self.keypad[0x2] = true,
            Keycode::Num3 => self.keypad[0x3] = true,
            Keycode::Num4 => self.keypad[0xC] = true,
            Keycode::Q => self.keypad[0x4] = true,
            Keycode::W => self.keypad[0x5] = true,
            Keycode::E => self.keypad[0x6] = true,
            Keycode::R => self.keypad[0xD] = true,
            Keycode::A => self.keypad[0x7] = true,
            Keycode::S => self.keypad[0x8] = true,
            Keycode::D => self.keypad[0x9] = true,
            Keycode::F => self.keypad[0xE] = true,
            Keycode::Z => self.keypad[0xA] = true,
            Keycode::X => self.keypad[0x0] = true,
            Keycode::C => self.keypad[0xB] = true,
            Keycode::V => self.keypad[0xF] = true,
            _ => self.new_key_pressed = false,
        }
    }

    pub fn key_up(&mut self, key: Keycode) {
        self.last_key = None;
        match key {
            Keycode::Num1 => self.keypad[0x1] = false,
            Keycode::Num2 => self.keypad[0x2] = false,
            Keycode::Num3 => self.keypad[0x3] = false,
            Keycode::Num4 => self.keypad[0xC] = false,
            Keycode::Q => self.keypad[0x4] = false,
            Keycode::W => self.keypad[0x5] = false,
            Keycode::E => self.keypad[0x6] = false,
            Keycode::R => self.keypad[0xD] = false,
            Keycode::A => self.keypad[0x7] = false,
            Keycode::S => self.keypad[0x8] = false,
            Keycode::D => self.keypad[0x9] = false,
            Keycode::F => self.keypad[0xE] = false,
            Keycode::Z => self.keypad[0xA] = false,
            Keycode::X => self.keypad[0x0] = false,
            Keycode::C => self.keypad[0xB] = false,
            Keycode::V => self.keypad[0xF] = false,
            _ => {}
        }

        if self.keypad.iter().all(|&key_state| !key_state) {
            self.new_key_pressed = false;
        }
    }
}