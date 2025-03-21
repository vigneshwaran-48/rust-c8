use std::{
    fs::File,
    io::{BufReader, Error, ErrorKind, Read},
    thread,
    time::Duration,
    usize,
};

use sdl2::event::Event;
use sdl2::keyboard::Keycode;

use super::Display;

const WIDTH: usize = 64;
const HEIGHT: usize = 32;

const STACK_SIZE: usize = 16;

pub struct Chip {
    memory: [u8; 4096],
    pc: u16,
    display: Display,
    registers: [u8; 16],
    i: u16,
    screen: [u8; WIDTH * HEIGHT],
    stack: Vec<u16>,
}

impl Chip {
    pub fn new() -> Self {
        let display = Display::init().expect("Error while initializing display");

        Self {
            memory: [0; 4096],
            pc: 0x200,
            display,
            registers: [0; 16],
            i: 0,
            screen: [0; WIDTH * HEIGHT],
            stack: vec![],
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
                self.registers[register] += value;
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
                            self.registers[x as usize] -= self.registers[y as usize];
                        } else {
                            self.registers[0xF] = 0;
                            self.registers[x as usize] = 255 + x_value - y_value; // Wrap around if
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
                            self.registers[x as usize] =
                                self.registers[y as usize] - self.registers[x as usize];
                        } else {
                            self.registers[0xF] = 0;
                            self.registers[x as usize] = 255 + x_value - y_value; // Wrap around if
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
                let x = instruction & 0x0F00;
                let y = instruction & 0x00F0;
                if self.registers[x as usize] != self.registers[y as usize] {
                    self.pc += 2;
                }
            }
            // Set the value to I register
            0xA000 => {
                self.i = instruction & 0x0FFF;
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

                        let screen_x = (x + column) as usize % WIDTH; // Handling overflow modulo
                        let screen_y = (y + row as u8) as usize % HEIGHT; // Handling overflow modulo
                        let pixel_index: usize = screen_y * WIDTH + screen_x;

                        if pixel == 1 && self.screen[pixel_index] == 1 {
                            self.registers[0xF] = 1;
                        }
                        self.screen[pixel_index] ^= pixel;
                    }
                }
                self.display.draw(&self.screen).unwrap();
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
            for event in event_pump.poll_iter() {
                match event {
                    Event::Quit { .. }
                    | Event::KeyDown {
                        keycode: Some(Keycode::Escape),
                        ..
                    } => break 'running,
                    _ => {}
                }
            }
            self.execute_instruction()
                .map_err(|e| format!("Failed to execute instruction: {}", e))?;
            thread::sleep(Duration::from_millis(2));
        }
        Ok(())
    }
}
