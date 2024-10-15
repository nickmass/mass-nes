use std::fmt::Display;

pub trait Reader {
    fn read(&self, addr: u16) -> u8;
}

impl<T: Fn(u16) -> u8> Reader for T {
    fn read(&self, addr: u16) -> u8 {
        self(addr)
    }
}

pub struct InstructionIter<T> {
    reader: T,
    pc: u16,
}

impl<T: Reader> InstructionIter<T> {
    pub fn new(reader: T, pc: u16) -> Self {
        Self { reader, pc }
    }
}

impl<T: Reader> Iterator for InstructionIter<T> {
    type Item = Instruction;

    fn next(&mut self) -> Option<Self::Item> {
        Some(Instruction::read(&mut self.pc, &self.reader))
    }
}

pub struct Instruction {
    pub pc: u16,
    pub op_code: OpCode,
    pub addressing: Addressing,
}

impl Instruction {
    fn read<T: Reader>(pc: &mut u16, reader: &T) -> Self {
        let op_pc = *pc;
        let mut read = || {
            let v = reader.read(*pc);
            *pc = pc.wrapping_add(1);
            v
        };
        let op_code = OpCode::from_byte(read());
        let addressing = op_code.addressing.read_operands(read);

        Self {
            pc: op_pc,
            op_code,
            addressing,
        }
    }

    pub fn pc(&self) -> u16 {
        self.pc
    }
}

impl Display for Instruction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "0x{:04X}: {:02X} {} {} {}",
            self.pc,
            self.op_code.op_code,
            self.addressing.display_operands(),
            self.op_code.instruction.mnemonic(),
            self.addressing
        )
    }
}

#[derive(Debug, Copy, Clone)]
pub enum Addressing {
    Implied,
    Relative(u8),
    Immediate(u8),
    ZeroPage(u8),
    ZeroPageX(u8),
    ZeroPageY(u8),
    Absolute(u8, u8),
    AbsoluteX(u8, u8),
    AbsoluteY(u8, u8),
    Indirect(u8, u8),
    IndirectX(u8),
    IndirectY(u8),
}

impl Display for Addressing {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Addressing::Implied => write!(f, ""),
            Addressing::Relative(a) => write!(f, "${a:02X}"),
            Addressing::Immediate(a) => write!(f, "#${a:02X}"),
            Addressing::ZeroPage(a) => write!(f, "${a:02X}"),
            Addressing::ZeroPageX(a) => write!(f, "${a:02X}, X"),
            Addressing::ZeroPageY(a) => write!(f, "${a:02X}, Y"),
            Addressing::Absolute(a, b) => {
                let addr = (*b as u16) << 8 | (*a as u16);
                write!(f, "${addr:04X}")
            }
            Addressing::AbsoluteX(a, b) => {
                let addr = (*b as u16) << 8 | (*a as u16);
                write!(f, "${addr:04X}, X")
            }
            Addressing::AbsoluteY(a, b) => {
                let addr = (*b as u16) << 8 | (*a as u16);
                write!(f, "${addr:04X}, Y")
            }
            Addressing::Indirect(a, b) => {
                let addr = (*b as u16) << 8 | (*a as u16);
                write!(f, "(${addr:04X})")
            }
            Addressing::IndirectX(a) => write!(f, "(${a:02X}, X)"),
            Addressing::IndirectY(a) => write!(f, "(${a:02X}), Y"),
        }
    }
}

impl Addressing {
    pub fn display_operands(&self) -> DisplayOperands {
        match self {
            Addressing::Implied => DisplayOperands::None,
            Addressing::Relative(a)
            | Addressing::Immediate(a)
            | Addressing::ZeroPage(a)
            | Addressing::ZeroPageX(a)
            | Addressing::ZeroPageY(a)
            | Addressing::IndirectX(a)
            | Addressing::IndirectY(a) => DisplayOperands::One(*a),
            Addressing::Absolute(a, b)
            | Addressing::AbsoluteX(a, b)
            | Addressing::AbsoluteY(a, b)
            | Addressing::Indirect(a, b) => DisplayOperands::Two(*a, *b),
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub enum DisplayOperands {
    None,
    One(u8),
    Two(u8, u8),
}

impl Display for DisplayOperands {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DisplayOperands::None => write!(f, "      "),
            DisplayOperands::One(a) => write!(f, "{a:02X}    "),
            DisplayOperands::Two(a, b) => write!(f, "{a:02X} {b:02X} "),
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub enum AddressingKind {
    Implied,
    Relative,
    Immediate,
    ZeroPage,
    ZeroPageX,
    ZeroPageY,
    Absolute,
    AbsoluteX,
    AbsoluteY,
    Indirect,
    IndirectX,
    IndirectY,
}

impl AddressingKind {
    fn read_operands<F: FnMut() -> u8>(&self, mut read: F) -> Addressing {
        match self {
            AddressingKind::Implied => Addressing::Implied,
            AddressingKind::Relative => Addressing::Relative(read()),
            AddressingKind::Immediate => Addressing::Immediate(read()),
            AddressingKind::ZeroPage => Addressing::ZeroPage(read()),
            AddressingKind::ZeroPageX => Addressing::ZeroPageX(read()),
            AddressingKind::ZeroPageY => Addressing::ZeroPageY(read()),
            AddressingKind::Absolute => Addressing::Absolute(read(), read()),
            AddressingKind::AbsoluteX => Addressing::AbsoluteX(read(), read()),
            AddressingKind::AbsoluteY => Addressing::AbsoluteY(read(), read()),
            AddressingKind::Indirect => Addressing::Indirect(read(), read()),
            AddressingKind::IndirectX => Addressing::IndirectX(read()),
            AddressingKind::IndirectY => Addressing::IndirectY(read()),
        }
    }
}

macro_rules! declare_ops {
    ($($inst:ident($desc:literal) { $($byte:literal => $opcode:expr)*  })*) => {
        pub struct OpCode {
            pub op_code: u8,
            pub instruction: Inst,
            pub addressing: AddressingKind,
            pub cycles: usize,
            pub dummy_cycles: bool,
            pub illegal: bool,
        }

        impl Default for OpCode {
            fn default() -> Self {
                OpCode {
                    op_code: 0,
                    instruction: Inst::NOP,
                    addressing: AddressingKind::Immediate,
                    cycles: 0,
                    dummy_cycles: false,
                    illegal: false
                }
            }
        }

        impl OpCode {
            fn from_byte(byte: u8) -> Self {
                match byte {
                    $($($byte => $opcode,)*)*
                }
            }
        }

        #[derive(Debug, Copy, Clone)]
        pub enum Inst {
            $($inst,)*
        }

        impl Inst {
            pub fn mnemonic(&self) -> &'static str {
                match self {
                    $(Inst::$inst => stringify!($inst),)*
                }
            }

            pub fn description(&self) -> &'static str {
                match self {
                    $(Inst::$inst => $desc,)*
                }
            }
        }

    };
    (@arg, $op:expr, Dummy) => { $op.dummy_cycles = true; };
    (@arg, $op:expr, Illegal) => { $op.illegal = true; };
    ($($inst:ident($desc:literal) { $($byte:literal => $addr:ident, Cycles($cycles:literal) $(, $arg:tt)*;)*  })*) => {
        declare_ops! {
            $($inst($desc) { $($byte => {
                let mut op = OpCode::default();
                op.op_code = $byte;
                op.instruction = Inst::$inst;
                op.addressing = AddressingKind::$addr;
                op.cycles = $cycles;
                $(declare_ops!{@arg, op, $arg})*
                op
            })* })*
        }
    };
}

declare_ops! {
    ADC("Add memory to accumulator with carry") {
        0x69 => Immediate, Cycles(2);
        0x65 => ZeroPage, Cycles(3);
        0x75 => ZeroPageX, Cycles(4);
        0x6d => Absolute, Cycles(4);
        0x7d => AbsoluteX, Cycles(4), Dummy;
        0x79 => AbsoluteY, Cycles(4), Dummy;
        0x61 => IndirectX, Cycles(6);
        0x71 => IndirectY, Cycles(5), Dummy;
    }
    AHX("Store accumulator and index x") {
        0x9f => AbsoluteY, Cycles(5), Illegal;
        0x93 => IndirectY, Cycles(6), Illegal;
    }
    ALR("AND with accumulator and shift right") {
        0x4b => Immediate, Cycles(2), Illegal;
    }
    ANC("AND with accumulator with carry") {
        0x0b => Immediate, Cycles(2), Illegal;
        0x2b => Immediate, Cycles(2), Illegal;
    }
    AND("AND memory with accumulator") {
        0x29 => Immediate, Cycles(2);
        0x25 => ZeroPage, Cycles(3);
        0x35 => ZeroPageX, Cycles(4);
        0x2d => Absolute, Cycles(4);
        0x3d => AbsoluteX, Cycles(4), Dummy;
        0x39 => AbsoluteY, Cycles(4), Dummy;
        0x21 => IndirectX, Cycles(6);
        0x31 => IndirectY, Cycles(5);
    }
    ARR("AND with accumulator and rotate right") {
        0x6b => Immediate, Cycles(2), Illegal;
    }
    ASL("Shift left one bit") {
        0x0a => Implied, Cycles(2);
        0x06 => ZeroPage, Cycles(5);
        0x16 => ZeroPageX, Cycles(6);
        0x0e => Absolute, Cycles(6);
        0x1e => AbsoluteX, Cycles(7);
    }
    AXS("AND accumulator with index s and subtract") {
        0xcb => Immediate, Cycles(2), Illegal;
    }
    BCC("Branch on carry clear") {
        0x90 => Relative, Cycles(2), Dummy;
    }
    BCS("Branch on carry set") {
        0xb0 => Relative, Cycles(2), Dummy;
    }
    BEQ("Branch on result zero") {
        0xf0 => Relative, Cycles(2), Dummy;
    }
    BIT("Test bits in memory with accumulator") {
        0x24 => ZeroPage, Cycles(3);
        0x2c => Absolute, Cycles(4);
    }
    BMI("Branch on result minus") {
        0x30 => Relative, Cycles(2), Dummy;
    }
    BNE("Branch on result not zero") {
        0xd0 => Relative, Cycles(2), Dummy;
    }
    BPL("Branch on result plus") {
        0x10 => Relative, Cycles(2), Dummy;
    }
    BRK("Force break") {
        0x00 => Implied, Cycles(7);
    }
    BVC("Branch on overflow clear") {
        0x50 => Relative, Cycles(2), Dummy;
    }
    BVS("Branch on overflow set") {
        0x70 => Relative, Cycles(2), Dummy;
    }
    CLC("Clear carry flag") {
        0x18 => Implied, Cycles(2);
    }
    CLD("Clear decimal mode") {
        0xD8 => Implied, Cycles(2);
    }
    CLI("Clear interrupt disable bit") {
        0x58 => Implied, Cycles(2);
    }
    CLV("Clear overflow flag") {
        0xb8 => Implied, Cycles(2);
    }
    CMP("Compare memory and accumulator") {
        0xc9 => Immediate, Cycles(2);
        0xc5 => ZeroPage, Cycles(3);
        0xd5 => ZeroPageX, Cycles(4);
        0xcd => Absolute, Cycles(4);
        0xdd => AbsoluteX, Cycles(4), Dummy;
        0xd9 => AbsoluteY, Cycles(4), Dummy;
        0xc1 => IndirectX, Cycles(6);
        0xd1 => IndirectY, Cycles(5), Dummy;
    }
    CPX("Compare memory and index x") {
        0xe0 => Immediate, Cycles(2);
        0xe4 => ZeroPage, Cycles(3);
        0xec => Absolute, Cycles(4);
    }
    CPY("Compare memory and index y") {
        0xc0 => Immediate, Cycles(2);
        0xc4 => ZeroPage, Cycles(3);
        0xcc => Absolute, Cycles(4);
    }
    DCP("Decrement and compare with accumulator") {
        0xc7 => ZeroPage, Cycles(5), Illegal;
        0xd7 => ZeroPageX, Cycles(6), Illegal;
        0xcf => Absolute, Cycles(6), Illegal;
        0xdf => AbsoluteX, Cycles(7), Illegal;
        0xdb => AbsoluteY, Cycles(7), Illegal;
        0xc3 => IndirectX, Cycles(8), Illegal;
        0xd3 => IndirectY, Cycles(8), Illegal;
    }
    DEC("Decrement memory by one") {
        0xc6 => ZeroPage, Cycles(5);
        0xd6 => ZeroPageX, Cycles(6);
        0xce => Absolute, Cycles(6);
        0xde => AbsoluteX, Cycles(7);
    }
    DEX("Decrement index x by one") {
        0xca => Implied, Cycles(2);
    }
    DEY("Decrement index y by one") {
        0x88 => Implied, Cycles(2);
    }
    EOR("Exclusive-or memory with accumulator") {
        0x49 => Immediate, Cycles(2);
        0x45 => ZeroPage, Cycles(3);
        0x55 => ZeroPageX, Cycles(4);
        0x40 => Absolute, Cycles(4);
        0x5D => AbsoluteX, Cycles(4), Dummy;
        0x59 => AbsoluteY, Cycles(4), Dummy;
        0x41 => IndirectX, Cycles(6);
        0x51 => IndirectY, Cycles(5), Dummy;
    }
    INC("Increment memory by one") {
        0xe6 => ZeroPage, Cycles(5);
        0xf6 => ZeroPageX, Cycles(6);
        0xee => Absolute, Cycles(6);
        0xfe => AbsoluteX, Cycles(7);
    }
    INX("Increment index x by one") {
        0xe8 => Implied, Cycles(2);
    }
    INY("Increment index y by one") {
        0xc8 => Implied, Cycles(2);
    }
    ISC("Increment and subtract from accumulator") {
        0xe7 => ZeroPage, Cycles(5), Illegal;
        0xf7 => ZeroPageX, Cycles(6), Illegal;
        0xef => Absolute, Cycles(6), Illegal;
        0xff => AbsoluteX, Cycles(7), Illegal;
        0xfb => AbsoluteY, Cycles(7), Illegal;
        0xe3 => IndirectX, Cycles(8), Illegal;
        0xf3 => IndirectY, Cycles(8), Illegal;
    }
    JMP("Jump to new location") {
        0x4c => Absolute, Cycles(3);
        0x6c => Indirect, Cycles(5);
    }
    JSR("Jump to subroutine") {
        0x20 => Absolute, Cycles(6);
    }
    KIL("Halt the processor") {
        0x02 => Implied, Cycles(2), Illegal;
        0x12 => Implied, Cycles(2), Illegal;
        0x22 => Implied, Cycles(2), Illegal;
        0x32 => Implied, Cycles(2), Illegal;
        0x42 => Implied, Cycles(2), Illegal;
        0x52 => Implied, Cycles(2), Illegal;
        0x62 => Implied, Cycles(2), Illegal;
        0x72 => Implied, Cycles(2), Illegal;
        0x92 => Implied, Cycles(2), Illegal;
        0xb2 => Implied, Cycles(2), Illegal;
        0xd2 => Implied, Cycles(2), Illegal;
        0xf2 => Implied, Cycles(2), Illegal;
    }
    LAS("AND with stack pointer and transfer to accumulator and index x") {
        0xbb => AbsoluteY, Cycles(4), Dummy, Illegal;
    }
    LAX("Load accumulator and index x with memory") {
        0xab => Immediate, Cycles(2), Illegal;
        0xa7 => ZeroPage, Cycles(3), Illegal;
        0xb7 => ZeroPageY, Cycles(4), Illegal;
        0xaf => Absolute, Cycles(4), Illegal;
        0xbf => AbsoluteY, Cycles(4), Dummy, Illegal;
        0xa3 => IndirectX, Cycles(6), Illegal;
        0xb3 => IndirectY, Cycles(5), Dummy, Illegal;
    }
    LDA("Load accumulator with memory") {
        0xa9 => Immediate, Cycles(2);
        0xa5 => ZeroPage, Cycles(3);
        0xb5 => ZeroPageX, Cycles(4);
        0xad => Absolute, Cycles(4);
        0xbd => AbsoluteX, Cycles(4), Dummy;
        0xb9 => AbsoluteY, Cycles(4), Dummy;
        0xa1 => IndirectX, Cycles(6);
        0xb1 => IndirectY, Cycles(5), Dummy;
    }
    LDX("Load index x with memory") {
        0xa2 => Immediate, Cycles(2);
        0xa6 => ZeroPage, Cycles(3);
        0xb6 => ZeroPageY, Cycles(4);
        0xae => Absolute, Cycles(4);
        0xbe => AbsoluteY, Cycles(4), Dummy;
    }
    LDY("Load index y with memory") {
        0xa0 => Immediate, Cycles(2);
        0xa4 => ZeroPage, Cycles(3);
        0xb4 => ZeroPageX, Cycles(4);
        0xac => Absolute, Cycles(4);
        0xbc => AbsoluteX, Cycles(4), Dummy;
    }
    LSR("Shift right one bit") {
        0x4a => Implied, Cycles(2);
        0x46 => ZeroPage, Cycles(5);
        0x56 => ZeroPageX, Cycles(6);
        0x4e => Absolute, Cycles(6);
        0x5e => AbsoluteX, Cycles(7);
    }
    NOP("No operation") {
        0xea => Implied, Cycles(2);
        0x1a => Implied, Cycles(2), Illegal;
        0x3a => Implied, Cycles(2), Illegal;
        0x5a => Implied, Cycles(2), Illegal;
        0x7a => Implied, Cycles(2), Illegal;
        0xda => Implied, Cycles(2), Illegal;
        0xfa => Implied, Cycles(2), Illegal;
        0x80 => Immediate, Cycles(2), Illegal;
        0x82 => Immediate, Cycles(2), Illegal;
        0x89 => Immediate, Cycles(2), Illegal;
        0xc2 => Immediate, Cycles(2), Illegal;
        0xe2 => Immediate, Cycles(2), Illegal;
        0x04 => ZeroPage, Cycles(3), Illegal;
        0x44 => ZeroPage, Cycles(3), Illegal;
        0x64 => ZeroPage, Cycles(3), Illegal;
        0x14 => ZeroPageX, Cycles(4), Illegal;
        0x34 => ZeroPageX, Cycles(4), Illegal;
        0x54 => ZeroPageX, Cycles(4), Illegal;
        0x74 => ZeroPageX, Cycles(4), Illegal;
        0xd4 => ZeroPageX, Cycles(4), Illegal;
        0xf4 => ZeroPageX, Cycles(4), Illegal;
        0x0c => Absolute, Cycles(4), Illegal;
        0x1c => AbsoluteX, Cycles(4), Dummy, Illegal;
        0x3c => AbsoluteX, Cycles(4), Dummy, Illegal;
        0x5c => AbsoluteX, Cycles(4), Dummy, Illegal;
        0x7c => AbsoluteX, Cycles(4), Dummy, Illegal;
        0xdc => AbsoluteX, Cycles(4), Dummy, Illegal;
        0xfc => AbsoluteX, Cycles(4), Dummy, Illegal;
    }
    ORA("OR memory with accumulator") {
        0x09 => Immediate, Cycles(2);
        0x05 => ZeroPage, Cycles(3);
        0x15 => ZeroPageX, Cycles(4);
        0x0d => Absolute, Cycles(4);
        0x1d => AbsoluteX, Cycles(4), Dummy;
        0x19 => AbsoluteY, Cycles(4), Dummy;
        0x01 => IndirectX, Cycles(6);
        0x11 => IndirectY, Cycles(5);
    }
    PHA("Push accumulator on stack") {
        0x48 => Implied, Cycles(3);
    }
    PHP("Push processor status on stack") {
        0x08 => Implied, Cycles(3);
    }
    PLA("Pull accumulator from stack") {
        0x68 => Implied, Cycles(4);
    }
    PLP("Pull processor status from stack") {
        0x28 => Implied, Cycles(4);
    }
    RLA("Rotate left one bit and AND with accumulator") {
        0x27 => ZeroPage, Cycles(5), Illegal;
        0x37 => ZeroPageX, Cycles(6), Illegal;
        0x2f => Absolute, Cycles(6), Illegal;
        0x3f => AbsoluteX, Cycles(7), Illegal;
        0x3b => AbsoluteY, Cycles(7), Illegal;
        0x23 => IndirectX, Cycles(8), Illegal;
        0x33 => IndirectY, Cycles(8), Illegal;
    }
    ROL("Rotate left one bit") {
        0x2a => Implied, Cycles(2);
        0x26 => ZeroPage, Cycles(5);
        0x36 => ZeroPageX, Cycles(6);
        0x2e => Absolute, Cycles(6);
        0x3e => AbsoluteX, Cycles(7);
    }
    ROR("Rotate right one bit") {
        0x6a => Implied, Cycles(2);
        0x66 => ZeroPage, Cycles(5);
        0x76 => ZeroPageX, Cycles(6);
        0x6e => Absolute, Cycles(6);
        0x7e => AbsoluteX, Cycles(7);
    }
    RRA("Rotate right one bit and add with accumulator") {
        0x67 => ZeroPage, Cycles(5), Illegal;
        0x77 => ZeroPageX, Cycles(6), Illegal;
        0x6f => Absolute, Cycles(6), Illegal;
        0x7f => AbsoluteX, Cycles(7), Illegal;
        0x7b => AbsoluteY, Cycles(7), Illegal;
        0x63 => IndirectX, Cycles(8), Illegal;
        0x73 => IndirectY, Cycles(8), Illegal;
    }
    RTI("Return from interrupt") {
        0x4d => Implied, Cycles(6);
    }
    RTS("Return from subroutine") {
        0x60 => Implied, Cycles(6);
    }
    SAX("AND accumulator with index x") {
        0x87 => ZeroPage, Cycles(3), Illegal;
        0x97 => ZeroPageY, Cycles(4), Illegal;
        0x8f => Absolute, Cycles(4), Illegal;
        0x83 => IndirectX, Cycles(6), Illegal;
    }
    SBC("Subtract memory from accumulator with borrow") {
        0xeb => Immediate, Cycles(2), Illegal;
        0xe9 => Immediate, Cycles(2);
        0xe5 => ZeroPage, Cycles(3);
        0xf5 => ZeroPageX, Cycles(4);
        0xed => Absolute, Cycles(4);
        0xfd => AbsoluteX, Cycles(4), Dummy;
        0xf9 => AbsoluteY, Cycles(4), Dummy;
        0xe1 => IndirectX, Cycles(6);
        0xf1 => IndirectY, Cycles(5);
    }
    SEC("Set carry flag") {
        0x38 => Implied, Cycles(2);
    }
    SED("Set decimal mode") {
        0xf8 => Implied, Cycles(2);
    }
    SEI("Set interrupt disable status") {
        0x78 => Implied, Cycles(2);
    }
    SHX("Store index x and high byte of address") {
        0x9e => AbsoluteY, Cycles(5), Illegal;
    }
    SHY("Store index y and high byte of address") {
        0x9c => AbsoluteX, Cycles(5), Illegal;
    }
    SLO("Shift left one bit and OR with accumulator") {
        0x07 => ZeroPage, Cycles(5), Illegal;
        0x17 => ZeroPageX, Cycles(6), Illegal;
        0x0f => Absolute, Cycles(6), Illegal;
        0x1f => AbsoluteX, Cycles(7), Illegal;
        0x1b => AbsoluteY, Cycles(7), Illegal;
        0x03 => IndirectX, Cycles(8), Illegal;
        0x13 => IndirectY, Cycles(8), Illegal;
    }
    SRE("Shift right one bit and XOR with accumulator") {
        0x47 => ZeroPage, Cycles(5), Illegal;
        0x57 => ZeroPageX, Cycles(6), Illegal;
        0x4f => Absolute, Cycles(6), Illegal;
        0x5f => AbsoluteX, Cycles(7), Illegal;
        0x5b => AbsoluteY, Cycles(7), Illegal;
        0x43 => IndirectX, Cycles(8), Illegal;
        0x53 => IndirectY, Cycles(8), Illegal;
    }
    STA("Store accumulator in memory") {
        0x85 => ZeroPage, Cycles(3);
        0x95 => ZeroPageX, Cycles(4);
        0x8d => Absolute, Cycles(4);
        0x9d => AbsoluteX, Cycles(5);
        0x99 => AbsoluteY, Cycles(5);
        0x81 => IndirectX, Cycles(6);
        0x91 => IndirectY, Cycles(6);
    }
    STX("Store index x in memory") {
        0x86 => ZeroPage, Cycles(3);
        0x96 => ZeroPageY, Cycles(4);
        0x8e => Absolute, Cycles(4);
    }
    STY("Store index y in memory") {
        0x84 => ZeroPage, Cycles(3);
        0x94 => ZeroPageX, Cycles(4);
        0x8c => Absolute, Cycles(4);
    }
    TAS("Transfer accumulator AND index x to stack pointer and memory") {
        0x9b => AbsoluteY, Cycles(5), Illegal;
    }
    TAX("Transfer accumulator to index x") {
        0xaa => Implied, Cycles(2);
    }
    TAY("Transfer accumulator to index y") {
        0xa8 => Implied, Cycles(2);
    }
    TSX("Transfer stack pointer to index x") {
        0xba => Implied, Cycles(2);
    }
    TXA("Transfer index x to accumulator") {
        0x8a => Implied, Cycles(2);
    }
    TXS("Transfer index x to stack pointer") {
        0x9a => Implied, Cycles(2);
    }
    TYA("Transfer index y to accumulator") {
        0x98 => Implied, Cycles(2);
    }
    XAA("AND with index x and load in accumulator") {
        0x8b => Immediate, Cycles(2), Illegal;
    }
}
