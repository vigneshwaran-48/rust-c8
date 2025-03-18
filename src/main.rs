use std::path::Path;

mod chip;
use chip::Chip;

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
}
