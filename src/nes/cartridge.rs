use std::io;
use std::convert::From;
use std::error;
use std::fmt;
use nes::system::{System, SystemState};
use nes::bus::{NotAndMask, AndAndMask, DeviceKind, BusKind};
use nes::cpu::Cpu;
use nes::ppu::Ppu;

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

impl  error::Error for CartridgeError {
    fn description(&self) -> &str {
        match *self {
            CartridgeError::InvalidFileType => "Unrecognized rom file format",
            CartridgeError::NotSupported => "Rom file format not supported",
            CartridgeError::CorruptedFile => "Rom file is corrupt",
            CartridgeError::IoError(ref x) => x.description()
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

enum RomMirrioring {
    Horizontal,
    Vertical,
    FourScreen,
}

pub struct Cartridge {
    prg_ram_bytes: usize,
    prg_rom: Vec<u8>,
    chr_rom: Vec<u8>,
    mirroring: RomMirrioring,
}

impl Cartridge {
    pub fn load<T: io::Read>(file: &mut T) -> Result<Cartridge, CartridgeError> {
        let mut buf = Vec::new();

        let file_size = try!(file.read_to_end(&mut buf));

        match Cartridge::get_rom_type(&buf) {
            Some(RomType::Ines) =>  Cartridge::load_ines(&buf),
            Some(RomType::Fds) =>  Cartridge::load_fds(&buf),
            Some(RomType::Unif) =>  Cartridge::load_unif(&buf),
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

        let mapper_number = (rom[6] >> 4) | (rom[7] & 0xF0);

        let mut data_start: usize = 16;

        if rom[6] & 0x04 != 0 {
            data_start = data_start + 512;
        }

        let mut mirroring = RomMirrioring::Horizontal;
        if rom[6] & 0x08 != 0 {
            mirroring = RomMirrioring::FourScreen;
        } else if rom[6] & 0x01 != 0 {
            mirroring = RomMirrioring::Vertical;
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
            prg_rom: rom[data_start..prg_rom_end].to_vec(),
            chr_rom: rom[prg_rom_end..chr_rom_end].to_vec(),
            mirroring: mirroring,
        };

        println!("PRGROM: {}, CHRROM: {}, Mapper: {}", prg_rom_bytes, chr_rom_bytes, mapper_number);
        Ok(cartridge)
    }

    fn load_fds(rom: &Vec<u8>) -> Result<Cartridge, CartridgeError> {
        println!("FDS");
        Err(CartridgeError::NotSupported)
    }

    fn load_unif(rom: &Vec<u8>) -> Result<Cartridge, CartridgeError> {
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

    pub fn register(&self, cpu: &mut Cpu, ppu: &mut Ppu) {
        cpu.register_read(DeviceKind::Mapper, AndAndMask(0x8000, 0x3fff));
        ppu.register_read(DeviceKind::Mapper, NotAndMask(0x1fff));
    }

    pub fn read(&self, bus: BusKind, state: &SystemState, address: u16) -> u8 {
        match bus {
            BusKind::Cpu => self.prg_rom[address as usize],
            BusKind::Ppu => self.chr_rom[address as usize],
        }
    }

    pub fn write(&self, bus: BusKind, state: &SystemState, address: u16, value: u8) {
        
    }
}
