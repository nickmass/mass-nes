use crate::mapper::{self};

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
    pub mapper: u32,
    pub submapper: Option<u32>,
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
            if header[11] & 0x0f != 0 {
                64 << (header[11] & 0x0f)
            } else {
                0
            }
        } else {
            if chr_rom_bytes == 0 {
                0x2000
            } else {
                0
            }
        };

        let mapper = ((header[6] >> 4) | (header[7] & 0xF0)) as u32;

        let (mapper, submapper) = if nes_2 {
            let mapper_hi = (header[8] & 0xf) as u32;
            let mapper = (mapper_hi << 8) | mapper;
            let submapper = (header[8] >> 4) as u32;

            (mapper, Some(submapper))
        } else {
            (mapper, None)
        };

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
            mapper,
            submapper,
        };

        let format = if nes_2 { "NES 2.0" } else { "iNES" };
        let mapper = if nes_2 {
            format!("{}:{}", mapper, submapper.unwrap_or(0))
        } else {
            format!("{}", mapper)
        };

        tracing::debug!(
            "{} PRGROM: {}, CHRROM: {}, PRGRAM: {}, CHRRAM: {}, Mapper: {}",
            format,
            prg_rom_bytes,
            chr_rom_bytes,
            prg_ram_bytes,
            chr_ram_bytes,
            mapper
        );
        Ok(cartridge)
    }

    fn load_fds<T: std::io::Read>(_file: &mut T) -> Result<Cartridge, CartridgeError> {
        tracing::debug!("FDS");
        Err(CartridgeError::NotSupported)
    }

    fn load_unif<T: std::io::Read>(_file: &mut T) -> Result<Cartridge, CartridgeError> {
        tracing::debug!("UNIF");
        Err(CartridgeError::NotSupported)
    }

    fn get_rom_type(rom: &[u8]) -> Option<RomType> {
        let ines_header = b"NES\x1a";
        if rom.starts_with(ines_header) {
            return Some(RomType::Ines);
        }

        let fds_header = b"FDS\x1a";
        if rom.starts_with(fds_header) {
            return Some(RomType::Fds);
        }

        let unif_header = b"UNIF";
        if rom.starts_with(unif_header) {
            return Some(RomType::Unif);
        }

        None
    }

    pub fn build_mapper(self) -> mapper::RcMapper {
        mapper::ines(self)
    }
}
