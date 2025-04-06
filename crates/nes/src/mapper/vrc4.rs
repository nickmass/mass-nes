use std::rc::Rc;

#[cfg(feature = "save-states")]
use nes_traits::SaveState;
#[cfg(feature = "save-states")]
use serde::{Deserialize, Serialize};

use crate::bus::{AddressBus, AndAndMask, AndEqualsAndMask, BusKind, DeviceKind};
use crate::cartridge::INes;
use crate::debug::Debug;
use crate::mapper::Mapper;
use crate::memory::{FixedMemoryBlock, Memory};
use crate::ppu::PpuFetchKind;

use super::vrc_irq::VrcIrq;
use super::{Mirroring, Nametable, SimpleMirroring};

#[cfg_attr(feature = "save-states", derive(Serialize, Deserialize))]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Vrc4Variant {
    Vrc2a,
    Vrc2b,
    Vrc2c,
    Vrc4a,
    Vrc4b,
    Vrc4c,
    Vrc4d,
    Vrc4e,
    Vrc4f,
}

impl Vrc4Variant {
    fn register_decode(&self, addr: u16) -> u16 {
        let (a0, a1) = match self {
            Vrc4Variant::Vrc2a => (1, 0),
            Vrc4Variant::Vrc2b => (0, 1),
            Vrc4Variant::Vrc2c => (1, 0),
            Vrc4Variant::Vrc4a => (1, 2),
            Vrc4Variant::Vrc4b => (1, 0),
            Vrc4Variant::Vrc4c => (6, 7),
            Vrc4Variant::Vrc4d => (3, 2),
            Vrc4Variant::Vrc4e => (2, 3),
            Vrc4Variant::Vrc4f => (0, 1),
        };

        let a0 = addr >> a0 & 1;
        let a1 = addr >> a1 & 1;

        (a0 | (a1 << 1)) | addr & 0xf000
    }

    fn is_vrc2(&self) -> bool {
        matches!(
            self,
            Vrc4Variant::Vrc2a | Vrc4Variant::Vrc2b | Vrc4Variant::Vrc2c
        )
    }

    fn decode_mirroring(&self, value: u8) -> Mirroring {
        if self.is_vrc2() {
            if value & 1 == 0 {
                Mirroring::Vertical
            } else {
                Mirroring::Horizontal
            }
        } else {
            match value & 0x3 {
                0 => Mirroring::Vertical,
                1 => Mirroring::Horizontal,
                2 => Mirroring::Single(Nametable::InternalB),
                3 => Mirroring::Single(Nametable::InternalA),
                _ => unreachable!(),
            }
        }
    }

    fn is_mirroring_reg(&self, decode_addr: u16) -> bool {
        if self.is_vrc2() {
            decode_addr == 0x9000
                || decode_addr == 0x9001
                || decode_addr == 0x9002
                || decode_addr == 0x9003
        } else {
            decode_addr == 0x9000
        }
    }

    fn is_swap_reg(&self, decode_addr: u16) -> bool {
        if self.is_vrc2() {
            false
        } else {
            decode_addr == 0x9002
        }
    }

    fn has_microwire(&self) -> bool {
        self.is_vrc2()
    }

    fn has_irq(&self) -> bool {
        !self.is_vrc2()
    }

    fn decode_chr_bank(&self, lo: u8, hi: u8) -> usize {
        let lo = lo & 0xf;
        let hi = if self.is_vrc2() { hi & 0xf } else { hi & 0x1f };

        let bank = lo as usize | ((hi as usize) << 4);

        if *self == Vrc4Variant::Vrc2a {
            bank >> 1
        } else {
            bank
        }
    }
}

#[cfg_attr(feature = "save-states", derive(SaveState))]
pub struct Vrc4 {
    #[cfg_attr(feature = "save-states", save(skip))]
    cartridge: INes,
    variant: Vrc4Variant,
    mirroring: SimpleMirroring,
    #[cfg_attr(feature = "save-states", save(nested))]
    irq: VrcIrq,
    prg_ram: Option<FixedMemoryBlock<8>>,
    prg_regs: [u8; 4],
    chr_lo_regs: [u8; 8],
    chr_hi_regs: [u8; 8],
    ram_protect: bool,
    swap_mode: bool,
    microwire_latch: u8,
}

impl Vrc4 {
    pub fn new(mut cartridge: INes, variant: Vrc4Variant, debug: Rc<Debug>) -> Self {
        let last_bank = ((cartridge.prg_rom.len() / 0x2000) - 1) as u8;
        let fixed_bank = ((cartridge.prg_rom.len() / 0x2000) - 2) as u8;
        let prg_ram = if cartridge.prg_ram_bytes > 0 {
            let mut ram = FixedMemoryBlock::new();
            if let Some(wram) = cartridge.wram.take() {
                ram.restore_wram(wram);
            }
            Some(ram)
        } else {
            None
        };

        let mirroring = SimpleMirroring::new(cartridge.mirroring);

        Self {
            variant,
            cartridge,
            mirroring,
            irq: VrcIrq::new(debug),
            prg_ram,
            prg_regs: [0, 0, fixed_bank, last_bank],
            chr_lo_regs: [0; 8],
            chr_hi_regs: [0; 8],
            ram_protect: true,
            swap_mode: false,
            microwire_latch: 0,
        }
    }

    fn read_cpu(&self, addr: u16) -> u8 {
        if addr & 0x8000 != 0 {
            let bank_idx = match (self.swap_mode, addr & 0xe000) {
                (_, 0xe000) => 3,
                (_, 0xa000) => 1,
                (true, 0x8000) => 2,
                (true, 0xc000) => 0,
                (false, 0x8000) => 0,
                (false, 0xc000) => 2,
                _ => unreachable!(),
            };
            let bank = self.prg_regs[bank_idx] as usize;
            return self.cartridge.prg_rom.read_mapped(bank, 8 * 1024, addr);
        } else if let Some(ram) = self.prg_ram.as_ref() {
            if !self.ram_protect {
                return ram.read(addr);
            }
        } else if self.variant.has_microwire() {
            if addr >= 0x6000 && addr < 0x7000 {
                return self.microwire_latch & 0x01;
            }
        }

        0
    }

    fn write_cpu(&mut self, addr: u16, value: u8) {
        if addr >= 0x8000 {
            let addr = self.variant.register_decode(addr);
            let mut chr_lo = None;
            let mut chr_hi = None;
            match addr {
                0x8000 | 0x8001 | 0x8002 | 0x8003 => self.prg_regs[0] = value,
                0xa000 | 0xa001 | 0xa002 | 0xa003 => self.prg_regs[1] = value,
                0xb000 => chr_lo = Some(0),
                0xb002 => chr_lo = Some(1),
                0xc000 => chr_lo = Some(2),
                0xc002 => chr_lo = Some(3),
                0xd000 => chr_lo = Some(4),
                0xd002 => chr_lo = Some(5),
                0xe000 => chr_lo = Some(6),
                0xe002 => chr_lo = Some(7),
                0xb001 => chr_hi = Some(0),
                0xb003 => chr_hi = Some(1),
                0xc001 => chr_hi = Some(2),
                0xc003 => chr_hi = Some(3),
                0xd001 => chr_hi = Some(4),
                0xd003 => chr_hi = Some(5),
                0xe001 => chr_hi = Some(6),
                0xe003 => chr_hi = Some(7),
                0xf000 if self.variant.has_irq() => self.irq.latch_lo(value),
                0xf001 if self.variant.has_irq() => self.irq.latch_hi(value),
                0xf002 if self.variant.has_irq() => self.irq.control(value),
                0xf003 if self.variant.has_irq() => self.irq.acknowledge(),
                addr if self.variant.is_swap_reg(addr) => {
                    self.ram_protect = value & 0x01 == 0;
                    self.swap_mode = value & 0x02 != 0;
                }
                addr if self.variant.is_mirroring_reg(addr) => {
                    let mirroring = self.variant.decode_mirroring(value);
                    self.mirroring.set(mirroring);
                }
                _ => {}
            }

            if let Some(lo) = chr_lo {
                self.chr_lo_regs[lo] = value;
            }
            if let Some(hi) = chr_hi {
                self.chr_hi_regs[hi] = value;
            }
        } else if let Some(ram) = self.prg_ram.as_mut() {
            if !self.ram_protect {
                ram.write(addr, value);
            }
        } else if self.variant.has_microwire() && addr >= 0x6000 && addr < 0x7000 {
            self.microwire_latch = value;
        }
    }

    fn read_ppu(&self, addr: u16) -> u8 {
        let bank_idx = addr as usize >> 10;
        let bank = self
            .variant
            .decode_chr_bank(self.chr_lo_regs[bank_idx], self.chr_hi_regs[bank_idx]);
        self.cartridge.chr_rom.read_mapped(bank, 1024, addr)
    }
}

impl Mapper for Vrc4 {
    fn register(&self, cpu: &mut AddressBus) {
        cpu.register_read(DeviceKind::Mapper, AndEqualsAndMask(0xe000, 0x6000, 0x7fff));
        cpu.register_write(DeviceKind::Mapper, AndEqualsAndMask(0xe000, 0x6000, 0x7fff));
        cpu.register_read(DeviceKind::Mapper, AndAndMask(0x8000, 0xffff));
        cpu.register_write(DeviceKind::Mapper, AndAndMask(0x8000, 0xffff));
    }

    fn peek(&self, bus: BusKind, addr: u16) -> u8 {
        match bus {
            BusKind::Cpu => self.read_cpu(addr),
            BusKind::Ppu => self.read_ppu(addr),
        }
    }

    fn write(&mut self, bus: BusKind, addr: u16, value: u8) {
        match bus {
            BusKind::Cpu => self.write_cpu(addr, value),
            BusKind::Ppu => (),
        }
    }

    fn peek_ppu_fetch(&self, address: u16, _kind: PpuFetchKind) -> Nametable {
        self.mirroring.ppu_fetch(address)
    }

    fn tick(&mut self) {
        if self.variant.has_irq() {
            self.irq.tick();
        }
    }

    fn get_irq(&mut self) -> bool {
        if self.variant.has_irq() {
            self.irq.irq()
        } else {
            false
        }
    }

    fn save_wram(&self) -> Option<super::SaveWram> {
        if self.cartridge.battery {
            self.prg_ram.as_ref().and_then(|r| r.save_wram())
        } else {
            None
        }
    }
}
