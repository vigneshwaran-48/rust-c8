use std::{
    fs::File,
    io::{BufReader, Error, ErrorKind, Read},
    thread,
    time::Duration,
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
            0x6000 => {
                let register = ((instruction & 0x0F00) >> 8) as usize;
                let value = (instruction & 0x00FF) as u8;
                self.registers[register] = value;
                println!("Setted register {} value to {}", register, value);
            }
            0x7000 => {
                let register = ((instruction & 0x0F00) >> 8) as usize;
                let value = (instruction & 0x00FF) as u8;
                self.registers[register] += value;
                println!("Added {} to register {}", value, register);
            }
            0xA000 => {
                self.i = instruction & 0x0FFF;
                println!("Setting I register to {}", self.i);
            }
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
