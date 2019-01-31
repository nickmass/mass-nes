use crate::mapper;
use crate::mapper::Mapper;
use crate::ppu::Ppu;
use crate::system::SystemState;
use std::convert::From;
use std::error;
use std::fmt;
use std::io;

#[derive(Debug)]
pub enum CartridgeError {
    InvalidFileType,
    NotSupported,
    CorruptedFile,
    IoError(io::Error),
}

impl fmt::Display for CartridgeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl error::Error for CartridgeError {
    fn description(&self) -> &str {
        match *self {
            CartridgeError::InvalidFileType => "Unrecognized rom file format",
            CartridgeError::NotSupported => "Rom file format not supported",
            CartridgeError::CorruptedFile => "Rom file is corrupt",
            CartridgeError::IoError(ref x) => x.description(),
        }
    }
}

impl From<io::Error> for CartridgeError {
    fn from(err: io::Error) -> CartridgeError {
        CartridgeError::IoError(err)
    }
}

enum RomType {
    Ines,
    Fds,
    Unif,
}

pub enum Mirroring {
    Horizontal,
    Vertical,
    FourScreen,
}

pub struct Cartridge {
    pub chr_ram_bytes: usize,
    pub prg_ram_bytes: usize,
    pub prg_rom: Vec<u8>,
    pub chr_rom: Vec<u8>,
    pub mirroring: Mirroring,
    mapper_number: u8,
}

impl Cartridge {
    pub fn load<T: io::Read>(file: &mut T) -> Result<Cartridge, CartridgeError> {
        let mut buf = Vec::new();

        let _file_size = file.read_to_end(&mut buf)?;

        match Cartridge::get_rom_type(&buf) {
            Some(RomType::Ines) => Cartridge::load_ines(&buf),
            Some(RomType::Fds) => Cartridge::load_fds(&buf),
            Some(RomType::Unif) => Cartridge::load_unif(&buf),
            None => Err(CartridgeError::InvalidFileType),
        }
    }

    fn load_ines(rom: &Vec<u8>) -> Result<Cartridge, CartridgeError> {
        println!("INES");
        if rom.len() < 16 {
            return Err(CartridgeError::CorruptedFile);
        }

        let prg_rom_bytes = (rom[4] as u32) * 2u32.pow(14);
        let chr_rom_bytes = (rom[5] as u32) * 2u32.pow(13);

        let chr_ram_bytes = if chr_rom_bytes == 0 { 0x2000 } else { 0 };

        let mapper_number = (rom[6] >> 4) | (rom[7] & 0xF0);

        let mut data_start: usize = 16;

        if rom[6] & 0x04 != 0 {
            data_start = data_start + 512;
        }

        let mut mirroring = Mirroring::Horizontal;
        if rom[6] & 0x08 != 0 {
            mirroring = Mirroring::FourScreen;
        } else if rom[6] & 0x01 != 0 {
            mirroring = Mirroring::Vertical;
        }

        let mut prg_ram_bytes = 0;
        if rom[6] & 0x02 != 0 {
            prg_ram_bytes = 0x2000;
        }

        let prg_rom_end: usize = data_start + prg_rom_bytes as usize;
        let chr_rom_end: usize = prg_rom_end + chr_rom_bytes as usize;
        if rom.len() < chr_rom_end {
            return Err(CartridgeError::CorruptedFile);
        }

        let cartridge = Cartridge {
            prg_ram_bytes: prg_ram_bytes,
            chr_ram_bytes: chr_ram_bytes,
            prg_rom: rom[data_start..prg_rom_end].to_vec(),
            chr_rom: rom[prg_rom_end..chr_rom_end].to_vec(),
            mirroring: mirroring,
            mapper_number: mapper_number,
        };

        println!(
            "PRGROM: {}, CHRROM: {}, Mapper: {}",
            prg_rom_bytes, chr_rom_bytes, mapper_number
        );
        Ok(cartridge)
    }

    fn load_fds(_rom: &Vec<u8>) -> Result<Cartridge, CartridgeError> {
        println!("FDS");
        Err(CartridgeError::NotSupported)
    }

    fn load_unif(_rom: &Vec<u8>) -> Result<Cartridge, CartridgeError> {
        println!("UNIF");
        Err(CartridgeError::NotSupported)
    }

    fn get_rom_type(rom: &Vec<u8>) -> Option<RomType> {
        let ines_header = [0x4E, 0x45, 0x53, 0x1A];
        if rom.starts_with(&ines_header) {
            return Some(RomType::Ines);
        }

        let fds_header = [0x46, 0x44, 0x53, 0x1A];
        if rom.starts_with(&fds_header) {
            return Some(RomType::Fds);
        }

        let unif_header = [0x55, 0x4E, 0x49, 0x46];
        if rom.starts_with(&unif_header) {
            return Some(RomType::Unif);
        }

        None
    }

    pub fn get_mapper(&self, state: &mut SystemState, ppu: &Ppu) -> Box<dyn Mapper> {
        match self.mirroring {
            Mirroring::Horizontal => ppu.nametables.set_horizontal(state),
            Mirroring::Vertical => ppu.nametables.set_vertical(state),
            _ => {}
        }
        mapper::ines(self.mapper_number, state, self)
    }
}