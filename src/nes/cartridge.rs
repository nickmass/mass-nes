use std::io;
use std::convert::From;

pub struct Cartridge;

pub enum CartridgeError {
    InvalidFileType,
    IoError(io::Error),
}

impl From<io::Error> for CartridgeError {
    fn from(err: io::Error) -> CartridgeError {
        CartridgeError::IoError(err)
    }
}


enum RomType {
    Ines,
}

impl Cartridge {
    pub fn load<T: io::Read>(file: &mut T) -> Result<Cartridge, CartridgeError> {
        let mut buf = Vec::new();

        let file_size = try!(file.read_to_end(&mut buf));
        
        match Cartridge::get_rom_type(&buf) {
            Some(RomType::Ines) =>  Ok(Cartridge::load_ines(&buf)),
            None => Err(CartridgeError::InvalidFileType),
        }
    }
    
    fn load_ines(rom: &Vec<u8>) -> Cartridge {
        println!("INES");
        Cartridge
    }

    fn get_rom_type(rom: &Vec<u8>) -> Option<RomType> {
        let ines_header = [0x4E, 0x45, 0x53, 0x1A];
        if rom.starts_with(&ines_header) {
            return Some(RomType::Ines);
        }

        None
    }
}
