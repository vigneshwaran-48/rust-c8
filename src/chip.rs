use std::{
    fs::File,
    io::{BufReader, Error, Read},
};

pub struct Chip {
    ram: [u8; 4096],
}

impl Chip {
    pub fn new() -> Self {
        Self { ram: [0; 4096] }
    }

    pub fn load(&mut self, rom_path: &str) -> Result<(), Error> {
        println!("Loading");
        let mut file = BufReader::new(File::open(rom_path)?);
        let _ = file.read(&mut self.ram[0x200..])?;

        println!("Loaded rom into memory");

        println!("1 => {:0x}, 2 => b{:0x}, 3 => {:0x}", self.ram[0x200], self.ram[0x201], self.ram[0x201]); // Checking
        Ok(())
    }
}
