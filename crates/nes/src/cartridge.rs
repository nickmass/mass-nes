use crate::mapper::{self, Mapper};

use std::rc::Rc;
use std::{fmt, io};

#[derive(Debug)]
pub enum CartridgeError {
    InvalidFileType,
    NotSupported,
    IoError(io::Error),
}

impl fmt::Display for CartridgeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            CartridgeError::InvalidFileType => write!(f, "Unrecognized rom file format"),
            CartridgeError::NotSupported => write!(f, "Rom file format not supported"),
            CartridgeError::IoError(ref x) => write!(f, "Cartridge io error: {}", x),
        }
    }
}

impl From<io::Error> for CartridgeError {
    fn from(err: io::Error) -> CartridgeError {
        CartridgeError::IoError(err)
    }
}

#[derive(Debug, Copy, Clone)]
enum RomType {
    Ines,
    Fds,
    Unif,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum CartMirroring {
    Horizontal,
    Vertical,
    FourScreen,
}

impl From<CartMirroring> for mapper::Mirroring {
    fn from(value: CartMirroring) -> Self {
        match value {
            CartMirroring::Horizontal => mapper::Mirroring::Horizontal,
            CartMirroring::Vertical => mapper::Mirroring::Vertical,
            CartMirroring::FourScreen => mapper::Mirroring::Custom,
        }
    }
}

pub struct Cartridge {
    pub chr_ram_bytes: usize,
    pub prg_ram_bytes: usize,
    pub prg_rom: Vec<u8>,
    pub chr_rom: Vec<u8>,
    pub mirroring: CartMirroring,
    mapper_number: u8,
}

impl Cartridge {
    pub fn load<T: io::Read>(file: &mut T) -> Result<Cartridge, CartridgeError> {
        let mut ident = [0; 4];
        file.read_exact(&mut ident)?;

        match Cartridge::get_rom_type(&ident) {
            Some(RomType::Ines) => Cartridge::load_ines(file),
            Some(RomType::Fds) => Cartridge::load_fds(file),
            Some(RomType::Unif) => Cartridge::load_unif(file),
            None => Err(CartridgeError::InvalidFileType),
        }
    }

    fn load_ines<T: io::Read>(file: &mut T) -> Result<Cartridge, CartridgeError> {
        let mut header = [0; 16];
        file.read_exact(&mut header[4..])?;

        let nes_2 = header[7] & 0xc == 0x8;

        let prg_hi = if nes_2 { header[9] as usize & 0xf } else { 0 };
        let prg_rom_bytes = (header[4] as usize | (prg_hi << 8)) << 14;

        let chr_hi = if nes_2 {
            (header[9] as usize >> 4) & 0xf
        } else {
            0
        };
        let chr_rom_bytes = (header[5] as usize | (chr_hi << 8)) << 13;

        let chr_ram_bytes = if nes_2 {
            64 << (header[11] & 0x0f)
        } else {
            if chr_rom_bytes == 0 {
                0x2000
            } else {
                0
            }
        };

        let mapper_number = (header[6] >> 4) | (header[7] & 0xF0);

        if header[6] & 0x04 != 0 {
            // skip trainer
            file.read_exact(&mut [0; 512])?;
        }

        let mirroring = if header[6] & 0x08 != 0 {
            CartMirroring::FourScreen
        } else if header[6] & 0x01 != 0 {
            CartMirroring::Vertical
        } else {
            CartMirroring::Horizontal
        };

        let mut prg_ram_bytes = 0;
        if header[6] & 0x02 != 0 {
            prg_ram_bytes = 0x2000;
        }

        let mut prg_rom = vec![0; prg_rom_bytes];
        let mut chr_rom = vec![0; chr_rom_bytes];

        file.read_exact(&mut prg_rom)?;
        file.read_exact(&mut chr_rom)?;

        let cartridge = Cartridge {
            chr_ram_bytes,
            prg_ram_bytes,
            prg_rom,
            chr_rom,
            mirroring,
            mapper_number,
        };

        let format = if nes_2 { "NES 2.0" } else { "iNES" };

        eprintln!(
            "{} PRGROM: {}, CHRROM: {}, PRGRAM: {}, CHRRAM:{}, Mapper: {}",
            format, prg_rom_bytes, chr_rom_bytes, prg_ram_bytes, chr_ram_bytes, mapper_number
        );
        Ok(cartridge)
    }

    fn load_fds<T: std::io::Read>(_file: &mut T) -> Result<Cartridge, CartridgeError> {
        println!("FDS");
        Err(CartridgeError::NotSupported)
    }

    fn load_unif<T: std::io::Read>(_file: &mut T) -> Result<Cartridge, CartridgeError> {
        println!("UNIF");
        Err(CartridgeError::NotSupported)
    }

    fn get_rom_type(rom: &[u8]) -> Option<RomType> {
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

    pub fn build_mapper(self) -> Rc<dyn Mapper> {
        mapper::ines(self.mapper_number, self)
    }
}
