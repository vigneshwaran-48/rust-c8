use std::{
    collections::HashMap,
    fs::File,
    io::{BufReader, Error, ErrorKind, Read},
    thread,
    time::Duration,
};

use rand::random;
use rodio::{OutputStream, Sink, Source, source::SineWave};
use sdl2::event::Event;
use sdl2::keyboard::Keycode;

use super::Display;

const WIDTH: usize = 64;
const HEIGHT: usize = 32;

const STACK_SIZE: usize = 30;

pub struct Chip {
    memory: [u8; 4096],
    pc: u16,
    display: Display,
    registers: [u8; 16],
    i: u16,
    dt: u8, // Delay Timer
    st: u8, // Sound Timer
    waiting_for_key: bool,
    waiting_key_register: usize,
    screen: [u8; WIDTH * HEIGHT],
    stack: Vec<u16>,
    keypad: [bool; 16],
    keypad_map: HashMap<Keycode, usize>,
}

impl Chip {
    pub fn new() -> Self {
        let display = Display::init().expect("Error while initializing display");

        let keypad_map: HashMap<Keycode, usize> = [
            (Keycode::Num1, 0x1),
            (Keycode::Num2, 0x2),
            (Keycode::Num3, 0x3),
            (Keycode::Num4, 0xC),
            (Keycode::Q, 0x4),
            (Keycode::W, 0x5),
            (Keycode::E, 0x6),
            (Keycode::R, 0xD),
            (Keycode::A, 0x7),
            (Keycode::S, 0x8),
            (Keycode::D, 0x9),
            (Keycode::F, 0xE),
            (Keycode::Z, 0xA),
            (Keycode::X, 0x0),
            (Keycode::C, 0xB),
            (Keycode::V, 0xF),
        ]
        .into();

        let mut memory = [0; 4096];
        Self::load_fonts(&mut memory);

        Self {
            memory,
            pc: 0x200,
            display,
            registers: [0; 16],
            i: 0,
            dt: 0,
            st: 0,
            waiting_for_key: false,
            waiting_key_register: 0x0,
            screen: [0; WIDTH * HEIGHT],
            stack: vec![],
            keypad: [false; 16],
            keypad_map,
        }
    }

    pub fn load(&mut self, rom_path: &str) -> Result<(), Error> {
        let mut file = BufReader::new(File::open(rom_path)?);
        let _ = file.read(&mut self.memory[0x200..])?;
        Ok(())
    }

    pub fn execute_instruction(&mut self) -> Result<(), Error> {
        if (self.pc + 1) >= 4096 {
            return Ok(());
        }
        let high = self.memory[self.pc as usize] as u16;
        let low = self.memory[(self.pc + 1) as usize] as u16;
        let instruction = (high << 8) | low;

        let nibble = instruction & 0xF000;

        self.pc += 2;

        match nibble {
            // System Instructions
            0x0000 => match instruction {
                // Clear
                0x00E0 => {
                    self.display.clear_screen()?;
                    self.screen = [0; WIDTH * HEIGHT];
                }
                // Return from subroutine
                0x00EE => match self.stack.pop() {
                    Some(address) => {
                        self.pc = address;
                    }
                    None => {
                        return Err(Error::new(
                            ErrorKind::Other,
                            "Trying to return from the main stack",
                        ));
                    }
                },
                _ => {}
            },

            // Jump
            0x1000 => {
                let jump_addr = instruction & 0x0FFF;
                self.pc = jump_addr;
            }
            // Call
            0x2000 => {
                if self.stack.len() + 1 >= STACK_SIZE {
                    return Err(Error::new(ErrorKind::Other, "Stack overflow"));
                }
                self.stack.push(self.pc);

                let address = instruction & 0x0FFF;
                self.pc = address;
            }
            // Skip if equal to value
            0x3000 => {
                let x = (instruction & 0x0F00) >> 8;
                let value = instruction & 0x00FF;
                if self.registers[x as usize] == value as u8 {
                    self.pc += 2;
                }
            }
            // Skip if not equal to value
            0x4000 => {
                let x = (instruction & 0x0F00) >> 8;
                let value = instruction & 0x00FF;
                if self.registers[x as usize] != value as u8 {
                    self.pc += 2;
                }
            }
            // Skip if both register values equal
            0x5000 => {
                let x = (instruction & 0x0F00) >> 8;
                let y = (instruction & 0x00F0) >> 4;
                if self.registers[x as usize] == self.registers[y as usize] {
                    self.pc += 2;
                }
            }
            // Set the value to register
            0x6000 => {
                let register = ((instruction & 0x0F00) >> 8) as usize;
                let value = (instruction & 0x00FF) as u8;
                self.registers[register] = value;
            }
            // Add the value to register
            0x7000 => {
                let register = ((instruction & 0x0F00) >> 8) as usize;
                let value = (instruction & 0x00FF) as u8;
                self.registers[register] = value.overflowing_add(self.registers[register]).0;
            }
            // Register operations
            0x8000 => {
                let operation = instruction & 0x000F;
                let x = (instruction & 0x0F00) >> 8;
                let y = (instruction & 0x00F0) >> 4;
                match operation {
                    // Load
                    0x0000 => {
                        self.registers[x as usize] = self.registers[y as usize];
                    }
                    // Bitwise OR
                    0x0001 => {
                        self.registers[x as usize] |= self.registers[y as usize];
                    }
                    // Bitwise AND
                    0x0002 => {
                        self.registers[x as usize] &= self.registers[y as usize];
                    }
                    // Bitwise XOR
                    0x0003 => {
                        self.registers[x as usize] ^= self.registers[y as usize];
                    }
                    // Add with carry
                    0x0004 => {
                        // Could have used Rust's overflowing_add() But I need to implement it by
                        // myself.
                        let sum =
                            self.registers[x as usize] as u16 + self.registers[y as usize] as u16;
                        self.registers[x as usize] = (sum & 0xFF) as u8; // Short for 0x00FF
                        self.registers[0xF] = if sum > 0xFF { 1 } else { 0 }
                    }
                    // Subtract with borrow
                    0x0005 => {
                        let x_value = self.registers[x as usize];
                        let y_value = self.registers[y as usize];
                        if x_value >= y_value {
                            self.registers[0xF] = 1;
                            self.registers[x as usize] = (x_value as u16 - y_value as u16) as u8;
                        } else {
                            self.registers[0xF] = 0;
                            self.registers[x as usize] =
                                (256 + x_value as u16 - y_value as u16) as u8; // Wrap around if
                            // result goes negative
                        }
                    }
                    // Right Shift By 1
                    0x0006 => {
                        self.registers[0xF] = self.registers[x as usize] & 0x01; // Getting Least
                        // Significant Bit
                        self.registers[x as usize] >>= 1;
                    }
                    // Subtract register x from register y
                    0x0007 => {
                        let x_value = self.registers[x as usize];
                        let y_value = self.registers[y as usize];
                        if y_value >= x_value {
                            self.registers[0xF] = 1;
                            self.registers[x as usize] = (y_value as u16 - x_value as u16) as u8;
                        } else {
                            self.registers[0xF] = 0;
                            self.registers[x as usize] =
                                (256 + x_value as u16 - y_value as u16) as u8; // Wrap around if
                            // result goes negative
                        }
                    }
                    // Left Shift By 1
                    0x0008 => {
                        self.registers[0xF] = (self.registers[x as usize] & 0x80) >> 7; // Getting
                        // Most Significant Bit. (0x80 in binary is 10000000)
                        self.registers[x as usize] <<= 1;
                    }
                    _ => {}
                }
            }
            //Skip if both register values not equal
            0x9000 => {
                let x = (instruction & 0x0F00) >> 8;
                let y = (instruction & 0x00F0) >> 4;
                if self.registers[x as usize] != self.registers[y as usize] {
                    self.pc += 2;
                }
            }
            // Set the value to I register
            0xA000 => {
                self.i = instruction & 0x0FFF;
            }
            // Jump to address with offset.
            0xB000 => {
                let address = instruction & 0x0FFF;
                self.pc = address.wrapping_add(self.registers[0x0] as u16);
            }
            // Generate random number
            0xC000 => {
                let x = (instruction & 0x0F00) >> 8;
                let mask = (instruction & 0x00FF) as u8;
                let rand_num: u8 = random();
                self.registers[x as usize] = rand_num & mask;
            }
            // Draw to the screen from the given position
            0xD000 => {
                let x = (instruction & 0x0F00) >> 8;
                let y = (instruction & 0x00F0) >> 4;
                let n = instruction & 0x000F;

                let x = self.registers[x as usize];
                let y = self.registers[y as usize];

                for row in 0..n {
                    let sprite_row = self.memory[(self.i + row) as usize];

                    for column in 0..8 {
                        let pixel = (sprite_row >> (7 - column)) & 1;

                        let screen_x = (x as u16 + column) as usize % WIDTH; // Handling overflow modulo
                        let screen_y = (y as u16 + row) as usize % HEIGHT; // Handling overflow modulo
                        let pixel_index: usize = screen_y * WIDTH + screen_x;

                        if pixel == 1 && self.screen[pixel_index] == 1 {
                            self.registers[0xF] = 1;
                        }
                        self.screen[pixel_index] ^= pixel;
                    }
                }
                self.display.draw(&self.screen).unwrap();
            }
            // Keyboard input
            0xE000 => {
                let operation = instruction & 0x00FF;
                let x = (instruction & 0x0F00) >> 8;

                match operation {
                    // Skip instruction if key pressed
                    0x009E => {
                        if self.keypad[self.registers[x as usize] as usize] {
                            self.pc += 2;
                        }
                    }
                    0x00A1 => {
                        if !self.keypad[self.registers[x as usize] as usize] {
                            self.pc += 2;
                        }
                    }
                    _ => {}
                }
            }
            // Timers and Sound
            0xF000 => {
                let x = (instruction & 0x0F00) >> 8;
                let operation = instruction & 0x00FF;
                match operation {
                    // Set delay timer to register
                    0x0007 => {
                        self.registers[x as usize] = self.dt;
                    }
                    // Wait for key input
                    0x000A => {
                        self.waiting_for_key = true;
                        self.waiting_key_register = x as usize;
                    }
                    // Set register value to delay timer
                    0x0015 => {
                        self.dt = self.registers[x as usize];
                    }
                    // Set register value to sound timer.
                    0x0018 => {
                        // Need to implement the actual sound capability later.
                        self.st = self.registers[x as usize];
                    }

                    // Memory Operations

                    // Increment I register with register value
                    0x001E => {
                        self.i += self.registers[x as usize] as u16;
                    }
                    // Set I register to vx's digit start address
                    0x0029 => {
                        self.i = (self.registers[x as usize] as u16) * 5; // Each digit sprite is 5 bytes long. (If digit is 2 in vx then 2 x 5 = 10. the sprite start address for the digit 2 is 10)
                    }
                    // Store BCD representation of digit
                    0x0033 => {
                        let digit = self.registers[x as usize];
                        self.memory[self.i as usize] = digit / 100;
                        self.memory[self.i as usize + 1] = (digit % 100) / 10;
                        self.memory[self.i as usize + 2] = digit % 10;
                    }
                    // Store register v0 to vx values from register I location.
                    0x0055 => {
                        // In future we can implement Super Chip 8 behaviour of incrementing I register.
                        for i in 0..x + 1 {
                            self.memory[(self.i + i) as usize] = self.registers[i as usize];
                        }
                    }
                    // Read values from I location to v0 to vx registers.
                    0x0065 => {
                        // In future we can implement Super Chip 8 behaviour of incrementing I register.
                        for i in 0..x + 1 {
                            self.registers[i as usize] = self.memory[(self.i + i) as usize];
                        }
                    }
                    _ => {}
                }
            }
            _ => {
                println!("Unmatched instructoin {nibble}");
            }
        }
        Ok(())
    }

    pub fn start_loop(&mut self) -> Result<(), String> {
        let mut event_pump = self.display.event_pump()?;

        'running: loop {
            if self.dt > 0 {
                self.dt -= 1;
            }

            if self.st > 0 {
                Self::beep();
                self.st -= 1;
            }

            for event in event_pump.poll_iter() {
                match event {
                    Event::KeyUp {
                        keycode: Some(key), ..
                    } => {
                        if let Some(key_index) = self.keypad_map.get(&key) {
                            self.keypad[*key_index] = false;
                        }
                    }
                    Event::KeyDown {
                        keycode: Some(key), ..
                    } => {
                        if key == Keycode::Escape {
                            break 'running;
                        }
                        if let Some(key_index) = self.keypad_map.get(&key) {
                            self.keypad[*key_index] = true;

                            // Fx0A instruction handling
                            if self.waiting_for_key {
                                self.registers[self.waiting_key_register] = *key_index as u8;
                                self.waiting_for_key = false;
                            }
                        }
                    }
                    Event::Quit { .. } => break 'running,
                    _ => {}
                }
            }
            // Fx0A instruction handling
            if !self.waiting_for_key {
                self.execute_instruction()
                    .map_err(|e| format!("Failed to execute instruction: {}", e))?;
            }
            thread::sleep(Duration::from_millis(2));
        }
        Ok(())
    }

    fn load_fonts(memory: &mut [u8]) {
        let font_data: [u8; 80] = [
            0xF0, 0x90, 0x90, 0x90, 0xF0, // 0
            0x20, 0x60, 0x20, 0x20, 0x70, // 1
            0xF0, 0x10, 0xF0, 0x80, 0xF0, // 2
            0xF0, 0x10, 0xF0, 0x10, 0xF0, // 3
            0x90, 0x90, 0xF0, 0x10, 0x10, // 4
            0xF0, 0x80, 0xF0, 0x10, 0xF0, // 5
            0xF0, 0x80, 0xF0, 0x90, 0xF0, // 6
            0xF0, 0x10, 0x20, 0x40, 0x40, // 7
            0xF0, 0x90, 0xF0, 0x90, 0xF0, // 8
            0xF0, 0x90, 0xF0, 0x10, 0xF0, // 9
            0xF0, 0x90, 0xF0, 0x90, 0x90, // A
            0xE0, 0x90, 0xE0, 0x90, 0xE0, // B
            0xF0, 0x80, 0x80, 0x80, 0xF0, // C
            0xE0, 0x90, 0x90, 0x90, 0xE0, // D
            0xF0, 0x80, 0xF0, 0x80, 0xF0, // E
            0xF0, 0x80, 0xF0, 0x80, 0x80, // F
        ];

        // Load font data into memory starting at 0x000
        memory[..font_data.len()].copy_from_slice(&font_data);
    }

    fn beep() {
        thread::spawn(|| {
            let (_stream, stream_handle) =
                OutputStream::try_default().expect("Unable to get system sound device");
            let sink = Sink::try_new(&stream_handle).expect("Error while creating sink");

            let source = SineWave::new(440.0)
                .amplify(0.2)
                .take_duration(Duration::from_millis(50));
            sink.append(source);
            sink.sleep_until_end();
        });
    }
}
