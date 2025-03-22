use std::{
    fs::File,
    io::{BufReader, Error, Read},
    thread,
    time::Duration,
    usize,
};

use sdl2::event::Event;
use sdl2::keyboard::Keycode;

use super::Display;

pub struct Chip {
    memory: [u8; 4096],
    pc: u16,
    display: Display,
    registers: [u8; 16],
    i: u16,
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
        }
    }

    pub fn load(&mut self, rom_path: &str) -> Result<(), Error> {
        println!("Loading");
        let mut file = BufReader::new(File::open(rom_path)?);
        let _ = file.read(&mut self.memory[0x200..])?;

        println!("Loaded rom into memory");

        println!(
            "1 => {:0x}, 2 => b{:0x}, 3 => {:0x}",
            self.memory[0x200], self.memory[0x201], self.memory[0x201]
        );
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
            0x0000 => match instruction {
                0x00E0 => {
                    println!("Clear screen");
                    self.display.clear_screen()?;
                }
                0x00EE => {
                    println!("Return from subroutine");
                }
                _ => {}
            },
            0x1000 => {
                let jump_addr = instruction & 0x0FFF;
                self.pc = jump_addr;
                // println!("Jumped to {}", jump_addr);
            }
            0x2000 => {
                println!("Call subroutine");
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
                println!("Draw call for {x} {y} {n}");
                println!("Value in I => {}", self.memory[self.i as usize]);
                println!("Value in I => {}", self.i as usize);
                for i in 0..n {
                    println!(
                        "Sprite row {i} value {}",
                        self.memory[(self.i + i) as usize]
                    );
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
