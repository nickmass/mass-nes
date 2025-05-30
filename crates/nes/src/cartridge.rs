use crate::debug::Debug;
use crate::mapper::{self, Nametable, SaveWram};
use crate::memory::RomBlock;

use std::{fmt, io, rc::Rc};

#[derive(Debug)]
pub enum CartridgeError {
    InvalidFileType,
    NotSupported,
    BiosRequired(&'static str),
    IoError(io::Error),
}

impl fmt::Display for CartridgeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            CartridgeError::InvalidFileType => write!(f, "Unrecognized rom file format"),
            CartridgeError::NotSupported => write!(f, "Rom file format not supported"),
            CartridgeError::BiosRequired(bios_name) => {
                write!(f, "This rom requires a bios file named '{bios_name}'")
            }
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
}

impl CartMirroring {
    pub fn ppu_fetch(&self, address: u16) -> Nametable {
        if address & 0x2000 != 0 {
            match self {
                CartMirroring::Horizontal if address & 0x800 != 0 => Nametable::InternalA,
                CartMirroring::Horizontal => Nametable::InternalB,
                CartMirroring::Vertical if address & 0x400 != 0 => Nametable::InternalA,
                CartMirroring::Vertical => Nametable::InternalB,
            }
        } else {
            Nametable::External
        }
    }
}

impl From<CartMirroring> for mapper::Mirroring {
    fn from(value: CartMirroring) -> Self {
        match value {
            CartMirroring::Horizontal => mapper::Mirroring::Horizontal,
            CartMirroring::Vertical => mapper::Mirroring::Vertical,
        }
    }
}

pub struct INes {
    pub chr_ram_bytes: usize,
    pub prg_ram_bytes: usize,
    pub prg_rom: RomBlock,
    pub chr_rom: RomBlock,
    pub mirroring: CartMirroring,
    pub alternative_mirroring: bool,
    pub mapper: u32,
    pub submapper: Option<u32>,
    pub wram: Option<SaveWram>,
    pub battery: bool,
}

pub struct Fds {
    pub disk_sides: Vec<Vec<u8>>,
    pub bios: Vec<u8>,
}

pub enum CartridgeInfo {
    Cartridge,
    Fds { total_sides: usize },
}

pub enum Cartridge {
    INes(INes),
    Fds(Fds),
}

impl Cartridge {
    pub fn load<T: io::Read, S: AsRef<str>>(
        file: &mut T,
        wram: Option<SaveWram>,
        bios: Option<&mut T>,
        file_name: S,
    ) -> Result<Cartridge, CartridgeError> {
        let mut ident = [0; 4];
        file.read_exact(&mut ident)?;
        let file_name = file_name.as_ref();

        match Cartridge::get_rom_type(&ident, file_name) {
            Some(RomType::Ines) => Cartridge::load_ines(file, ident, wram),
            Some(RomType::Fds) => Cartridge::load_fds(file, ident, bios),
            Some(RomType::Unif) => Cartridge::load_unif(file),
            None => Err(CartridgeError::InvalidFileType),
        }
    }

    fn load_ines<T: io::Read>(
        file: &mut T,
        ident: [u8; 4],
        mut wram: Option<SaveWram>,
    ) -> Result<Cartridge, CartridgeError> {
        let mut header = [0; 16];
        header[0..4].copy_from_slice(&ident);
        file.read_exact(&mut header[4..])?;

        let nes_2 = header[7] & 0xc == 0x8;

        let prg_hi = if nes_2 { header[9] as usize & 0xf } else { 0 };
        let prg_rom_bytes = if prg_hi == 0xf {
            let mul = ((header[4] & 3) * 2 + 1) as usize;
            let exp = (header[4] >> 2) as u32;
            2usize.pow(exp) * mul
        } else {
            (header[4] as usize | (prg_hi << 8)) << 14
        };

        let chr_hi = if nes_2 { header[9] as usize >> 4 } else { 0 };
        let chr_rom_bytes = if chr_hi == 0xf {
            let mul = ((header[5] & 3) * 2 + 1) as usize;
            let exp = (header[5] >> 2) as u32;
            2usize.pow(exp) * mul
        } else {
            (header[5] as usize | (chr_hi << 8)) << 13
        };

        let chr_ram_bytes = if nes_2 {
            if header[11] & 0x0f != 0 {
                64 << (header[11] & 0x0f)
            } else {
                0
            }
        } else {
            if chr_rom_bytes == 0 { 0x2000 } else { 0 }
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

        let alternative_mirroring = header[6] & 0x08 != 0;
        let mirroring = if header[6] & 0x01 != 0 {
            CartMirroring::Vertical
        } else {
            CartMirroring::Horizontal
        };

        let mut prg_ram_bytes = 0;
        let mut battery = false;
        if header[6] & 0x02 != 0 {
            battery = true;
            if mapper == 5 {
                prg_ram_bytes = 64 * 1024;
            } else {
                prg_ram_bytes = 8 * 1024;
            }
        } else {
            wram = None;
        }

        // This is big simplification but it is better for roms to have an
        // incorrect amount of ram vs. no ram at all
        if nes_2 && header[10] != 0 {
            let volatile = header[10] as usize & 0xf;
            let non_volatile = header[10] as usize >> 4;
            let volatile = if volatile > 0 { 64 << volatile } else { 0 };
            let non_volatile = if non_volatile > 0 {
                64 << non_volatile
            } else {
                0
            };
            prg_ram_bytes = volatile + non_volatile;
        }

        let mut prg_rom = vec![0; prg_rom_bytes];
        let mut chr_rom = vec![0; chr_rom_bytes];

        file.read_exact(&mut prg_rom)?;
        file.read_exact(&mut chr_rom)?;

        let cartridge = INes {
            chr_ram_bytes,
            prg_ram_bytes,
            prg_rom: RomBlock::new(prg_rom),
            chr_rom: RomBlock::new(chr_rom),
            mirroring,
            alternative_mirroring,
            mapper,
            submapper,
            wram,
            battery,
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

        let mut header_str = String::with_capacity(16 * 3);
        use std::fmt::Write;
        for (idx, &n) in header.iter().enumerate() {
            if idx < 3 {
                let _ = write!(&mut header_str, "{} ", n as char);
            } else {
                let _ = write!(&mut header_str, "{:02x} ", n);
            }
        }

        tracing::debug!("Header: [ {}]", header_str);
        Ok(Cartridge::INes(cartridge))
    }

    fn load_fds<T: std::io::Read, B: std::io::Read>(
        file: &mut T,
        ident: [u8; 4],
        bios: Option<B>,
    ) -> Result<Cartridge, CartridgeError> {
        let Some(mut bios_rom) = bios else {
            return Err(CartridgeError::BiosRequired("disksys.rom"));
        };

        let mut buffer = Vec::new();
        if &ident == b"FDS\x1a" {
            // Skip header
            let mut header = [0; 16];
            file.read_exact(&mut header[4..])?;
        } else {
            buffer.extend(&ident);
        }

        file.read_to_end(&mut buffer)?;

        let mut bios = vec![0; 1024 * 8];
        bios_rom.read_exact(&mut bios)?;

        let mut disk_sides = Vec::new();
        while disk_sides.len() * 65500 < buffer.len() {
            let mut side = vec![0; 28300 / 8];

            let offset = disk_sides.len() * 65500;
            let mut i = 0;
            while i < 65500 {
                let idx = i + offset;
                let block_type = buffer[idx];
                let block_len = match block_type {
                    1 => 56,
                    2 => 2,
                    3 => 16,
                    4 => 1 + buffer[idx - 3] as usize + buffer[idx - 2] as usize * 0x100,
                    _ => break,
                };

                side.push(0x80);
                side.extend_from_slice(&buffer[idx..idx + block_len]);
                side.push(0x4d);
                side.push(0x62);
                side.extend((0..976 / 8).map(|_| 0));
                i += block_len;
            }

            if side.len() < 65500 {
                side.resize(65500, 0);
            }
            disk_sides.push(side);
        }

        tracing::debug!("FDS Disk Sides: {}", disk_sides.len());

        let fds = Fds { disk_sides, bios };

        Ok(Cartridge::Fds(fds))
    }

    fn load_unif<T: std::io::Read>(_file: &mut T) -> Result<Cartridge, CartridgeError> {
        tracing::debug!("UNIF");
        Err(CartridgeError::NotSupported)
    }

    fn get_rom_type(rom: &[u8], file_name: &str) -> Option<RomType> {
        let ines_header = b"NES\x1a";
        if rom.starts_with(ines_header) {
            return Some(RomType::Ines);
        }

        let ascii_ext = &file_name.as_bytes()[file_name.len() - 4..];
        let fds_header = b"FDS\x1a";
        if rom.starts_with(fds_header) || ascii_ext.eq_ignore_ascii_case(b".fds") {
            return Some(RomType::Fds);
        }

        let unif_header = b"UNIF";
        if rom.starts_with(unif_header) {
            return Some(RomType::Unif);
        }

        None
    }

    pub fn build_mapper(self, debug: Rc<Debug>) -> mapper::RcMapper {
        match self {
            Cartridge::INes(ines) => mapper::ines(ines, debug),
            Cartridge::Fds(fds) => mapper::fds(fds),
        }
    }

    pub fn info(&self) -> CartridgeInfo {
        match self {
            Cartridge::INes(_) => CartridgeInfo::Cartridge,
            Cartridge::Fds(fds) => CartridgeInfo::Fds {
                total_sides: fds.disk_sides.len(),
            },
        }
    }
}
