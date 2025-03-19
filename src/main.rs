use std::path::Path;
use std::thread;
use std::time::Duration;

use sdl2::event::Event;
use sdl2::keyboard::Keycode;

mod chip;
use chip::Chip;

mod display;
use display::Display;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() <= 1 {
        panic!("Required <ROM> file!");
    }
    let rom = &args[1];
    if !Path::new(rom).is_file() {
        panic!("Rom file {} not exists", rom)
    }

    let mut chip = Chip::new();
    chip.load(rom).expect("Error while loading rom");

    let display = &mut Display::init().expect("Error while initializing display");

    let mut event_pump = display
        .event_pump()
        .expect("Error while getting event pump");

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
        chip.execute_instruction()
            .expect("Error while executing instruction");
        thread::sleep(Duration::from_millis(2));
    }
}
