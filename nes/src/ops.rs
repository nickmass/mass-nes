static mut OPS: [Op; 0x100] = [Op {
    instruction: Instruction::Nop,
    addressing: Addressing::None,
}; 0x100];

#[derive(Debug, Clone, Copy)]
pub enum Instruction {
    Adc(ReadExec),
    And(ReadExec),
    Asl(ReadDummyExec),
    Asla,
    Bcc(Branch),
    Bcs(Branch),
    Beq(Branch),
    Bit(ReadExec),
    Bmi(Branch),
    Bne(Branch),
    Bpl(Branch),
    Brk(Break),
    Bvc(Branch),
    Bvs(Branch),
    Clc,
    Cld,
    Cli,
    Clv,
    Cmp(ReadExec),
    Cpx(ReadExec),
    Cpy(ReadExec),
    Dec(ReadDummyExec),
    Dex,
    Dey,
    Eor(ReadExec),
    Inc(ReadDummyExec),
    Inx,
    Iny,
    Jmp,
    Jsr(Jsr),
    Lda(ReadExec),
    Ldx(ReadExec),
    Ldy(ReadExec),
    Lsr(ReadDummyExec),
    Lsra,
    Nop,
    Ora(ReadExec),
    Pha,
    Php,
    Pla(DummyReadExec),
    Plp(DummyReadExec),
    Rol(ReadDummyExec),
    Rola,
    Ror(ReadDummyExec),
    Rora,
    Rti(Rti),
    Rts(Rts),
    Sbc(ReadExec),
    Sec,
    Sed,
    Sei,
    Sta,
    Stx,
    Sty,
    Tax,
    Tay,
    Tsx,
    Txa,
    Txs,
    Tya,

    //Illegals
    IllAhx,
    IllAlr(ReadExec),
    IllAnc(ReadExec),
    IllArr(ReadExec),
    IllAxs(ReadExec),
    IllDcp(ReadDummyExec),
    IllIsc(ReadDummyExec),
    IllKil,
    IllLas,
    IllLax(ReadExec),
    IllNop,
    IllNopAddr,
    IllRla(ReadDummyExec),
    IllRra(ReadDummyExec),
    IllSax,
    IllSbc(ReadExec),
    IllShx,
    IllShy,
    IllSlo(ReadDummyExec),
    IllSre(ReadDummyExec),
    IllTas,
    IllXaa(ReadExec),
}

impl Instruction {
    pub fn name(&self) -> &'static str {
        use Instruction::*;
        match self {
            Adc(..) => "ADC",
            And(..) => "AND",
            Asl(..) => "ASL",
            Asla => "ASL",
            Bit(..) => "BIT",
            Bcc(..) => "BCC",
            Bcs(..) => "BCS",
            Beq(..) => "BEQ",
            Bmi(..) => "BMI",
            Bne(..) => "BNE",
            Bpl(..) => "BPL",
            Brk(..) => "BRK",
            Bvc(..) => "BVC",
            Bvs(..) => "BVS",
            Clc => "CLC",
            Cld => "CLD",
            Cli => "CLI",
            Clv => "CLV",
            Cmp(..) => "CMP",
            Cpx(..) => "CPX",
            Cpy(..) => "CPY",
            Dec(..) => "DEC",
            Dex => "DEX",
            Dey => "DEY",
            Eor(..) => "EOR",
            Inc(..) => "INC",
            Inx => "INX",
            Iny => "INY",
            Jmp => "JMP",
            Jsr(..) => "JSR",
            Lda(..) => "LDA",
            Ldx(..) => "LDX",
            Ldy(..) => "LDY",
            Lsr(..) => "LSR",
            Lsra => "LSR",
            Nop => "NOP",
            Ora(..) => "ORA",
            Pha => "PHA",
            Php => "PHP",
            Pla(..) => "PLA",
            Plp(..) => "PLP",
            Rol(..) => "ROL",
            Rola => "ROL",
            Ror(..) => "ROR",
            Rora => "ROR",
            Rti(..) => "RTI",
            Rts(..) => "RTS",
            Sbc(..) => "SBC",
            Sec => "SEC",
            Sed => "SED",
            Sei => "SEI",
            Sta => "STA",
            Stx => "STX",
            Sty => "STY",
            Tax => "TAX",
            Tay => "TAY",
            Tsx => "TSX",
            Txa => "TXA",
            Txs => "TXS",
            Tya => "TYA",

            IllAhx => "*AHX",
            IllAnc(..) => "*ANC",
            IllAlr(..) => "*ALR",
            IllArr(..) => "*ARR",
            IllAxs(..) => "*AXS",
            IllDcp(..) => "*DCP",
            IllIsc(..) => "*ISC",
            IllKil => "*KIL",
            IllLas => "*LAS",
            IllLax(..) => "*LAX",
            IllNop => "*NOP",
            IllNopAddr => "*NOP",
            IllRla(..) => "*RLA",
            IllRra(..) => "*RRA",
            IllSax => "*SAX",
            IllSbc(..) => "*SBC",
            IllShx => "*SHX",
            IllShy => "*SHY",
            IllSlo(..) => "*SLO",
            IllSre(..) => "*SRE",
            IllTas => "*TAS",
            IllXaa(..) => "*XAA",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ReadExec {
    Read,
    Exec,
}

impl Default for ReadExec {
    fn default() -> Self {
        ReadExec::Read
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ReadDummyExec {
    Read,
    Dummy,
    Exec(u8),
}

impl Default for ReadDummyExec {
    fn default() -> Self {
        ReadDummyExec::Read
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Branch {
    Check,
    Branch,
}

impl Default for Branch {
    fn default() -> Self {
        Branch::Check
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Break {
    ReadDummy,
    WriteRegPcHigh,
    WriteRegPcLow,
    WriteRegP,
    ReadHighJump(u16),
    ReadLowJump(u16),
    UpdateRegPc(u16),
}

impl Default for Break {
    fn default() -> Self {
        Break::ReadDummy
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Jsr {
    ReadDummy,
    WriteRegPcHigh,
    WriteRegPcLow,
}

impl Default for Jsr {
    fn default() -> Self {
        Jsr::ReadDummy
    }
}

#[derive(Debug, Clone, Copy)]
pub enum DummyReadExec {
    Dummy,
    Read,
    Exec,
}

impl Default for DummyReadExec {
    fn default() -> Self {
        DummyReadExec::Dummy
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Rti {
    Dummy,
    ReadRegP,
    ReadRegPcLow,
    ReadRegPcHigh,
    Exec(u16),
}

impl Default for Rti {
    fn default() -> Self {
        Rti::Dummy
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Rts {
    Dummy,
    ReadRegPcLow,
    ReadRegPcHigh,
    Exec(u16),
}

impl Default for Rts {
    fn default() -> Self {
        Rts::Dummy
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ZeroPage {
    Read,
    Decode,
}

impl Default for ZeroPage {
    fn default() -> Self {
        ZeroPage::Read
    }
}

#[derive(Debug, Clone, Copy)]
pub enum AbsoluteOffset {
    ReadLow,
    ReadHigh,
    Decode(u16),
}

impl Default for AbsoluteOffset {
    fn default() -> Self {
        AbsoluteOffset::ReadLow
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Absolute {
    ReadLow,
    ReadHigh,
    Decode(u16),
}

impl Default for Absolute {
    fn default() -> Self {
        Absolute::ReadLow
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ZeroPageOffset {
    ReadImmediate,
    ApplyOffset,
}

impl Default for ZeroPageOffset {
    fn default() -> Self {
        ZeroPageOffset::ReadImmediate
    }
}

#[derive(Debug, Clone, Copy)]
pub enum IndirectAbsolute {
    ReadLow,
    ReadHigh,
    ReadIndirectLow(u16),
    ReadIndirectHigh(u16),
    Decode(u16),
}

impl Default for IndirectAbsolute {
    fn default() -> Self {
        IndirectAbsolute::ReadLow
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Relative {
    ReadRegPc,
    Decode,
}

impl Default for Relative {
    fn default() -> Self {
        Relative::ReadRegPc
    }
}

#[derive(Debug, Clone, Copy)]
pub enum IndirectX {
    ReadBase,
    ReadDummy,
    ReadIndirectLow(u16),
    ReadIndirectHigh(u16),
    Decode(u16),
}

impl Default for IndirectX {
    fn default() -> Self {
        IndirectX::ReadBase
    }
}

#[derive(Debug, Clone, Copy)]
pub enum IndirectY {
    ReadBase,
    ReadZeroPageLow,
    ReadZeroPageHigh(u16),
    Decode(u16),
}

impl Default for IndirectY {
    fn default() -> Self {
        IndirectY::ReadBase
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Reg {
    X,
    Y,
}

#[derive(Debug, Clone, Copy)]
pub enum DummyRead {
    Always,
    OnCarry,
}

#[derive(Debug, Clone, Copy)]
pub enum Addressing {
    None,
    ZeroPage(ZeroPage),
    Immediate,
    Accumulator,
    ZeroPageOffset(Reg, ZeroPageOffset),
    Absolute(Absolute),
    AbsoluteOffset(Reg, DummyRead, AbsoluteOffset),
    IndirectAbsolute(IndirectAbsolute),
    Relative(Relative),
    IndirectX(IndirectX),
    IndirectY(DummyRead, IndirectY),
}

impl Addressing {
    pub fn length(&self) -> usize {
        use Addressing::*;
        match self {
            None => 1,
            Accumulator => 1,
            Immediate => 2,
            ZeroPage(..) => 2,
            ZeroPageOffset(..) => 2,
            Absolute(..) => 3,
            AbsoluteOffset(..) => 3,
            IndirectAbsolute(..) => 3,
            Relative(..) => 2,
            IndirectX(..) => 2,
            IndirectY(..) => 2,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Op {
    pub instruction: Instruction,
    pub addressing: Addressing,
}

impl Op {
    pub fn load() -> &'static [Op; 0x100] {
        unsafe {
            use self::Addressing as A;
            use self::Instruction as I;

            let mut op_set = ::std::collections::HashSet::new();
            {
                let mut o = |o, i, a| {
                    OPS[o] = Op {
                        instruction: i,
                        addressing: a,
                    };

                    op_set.insert(o);
                };

                o(0xa8, I::Tay, A::None);
                o(0xaa, I::Tax, A::None);
                o(0xba, I::Tsx, A::None);
                o(0x98, I::Tya, A::None);
                o(0x8a, I::Txa, A::None);
                o(0x9a, I::Txs, A::None);

                o(0xa9, I::Lda(ReadExec::default()), A::Immediate);
                o(
                    0xa5,
                    I::Lda(ReadExec::default()),
                    A::ZeroPage(ZeroPage::default()),
                );
                o(
                    0xb5,
                    I::Lda(ReadExec::default()),
                    A::ZeroPageOffset(Reg::X, ZeroPageOffset::default()),
                );
                o(
                    0xad,
                    I::Lda(ReadExec::default()),
                    A::Absolute(Absolute::default()),
                );
                o(
                    0xbd,
                    I::Lda(ReadExec::default()),
                    A::AbsoluteOffset(Reg::X, DummyRead::OnCarry, AbsoluteOffset::default()),
                );
                o(
                    0xb9,
                    I::Lda(ReadExec::default()),
                    A::AbsoluteOffset(Reg::Y, DummyRead::OnCarry, AbsoluteOffset::default()),
                );
                o(
                    0xa1,
                    I::Lda(ReadExec::default()),
                    A::IndirectX(IndirectX::default()),
                );
                o(
                    0xb1,
                    I::Lda(ReadExec::default()),
                    A::IndirectY(DummyRead::OnCarry, IndirectY::default()),
                );

                o(0xa2, I::Ldx(ReadExec::default()), A::Immediate);
                o(
                    0xa6,
                    I::Ldx(ReadExec::default()),
                    A::ZeroPage(ZeroPage::default()),
                );
                o(
                    0xb6,
                    I::Ldx(ReadExec::default()),
                    A::ZeroPageOffset(Reg::Y, ZeroPageOffset::default()),
                );
                o(
                    0xae,
                    I::Ldx(ReadExec::default()),
                    A::Absolute(Absolute::default()),
                );
                o(
                    0xbe,
                    I::Ldx(ReadExec::default()),
                    A::AbsoluteOffset(Reg::Y, DummyRead::OnCarry, AbsoluteOffset::default()),
                );

                o(0xa0, I::Ldy(ReadExec::default()), A::Immediate);
                o(
                    0xa4,
                    I::Ldy(ReadExec::default()),
                    A::ZeroPage(ZeroPage::default()),
                );
                o(
                    0xb4,
                    I::Ldy(ReadExec::default()),
                    A::ZeroPageOffset(Reg::X, ZeroPageOffset::default()),
                );
                o(
                    0xac,
                    I::Ldy(ReadExec::default()),
                    A::Absolute(Absolute::default()),
                );
                o(
                    0xbc,
                    I::Ldy(ReadExec::default()),
                    A::AbsoluteOffset(Reg::X, DummyRead::OnCarry, AbsoluteOffset::default()),
                );

                o(0x85, I::Sta, A::ZeroPage(ZeroPage::default()));
                o(
                    0x95,
                    I::Sta,
                    A::ZeroPageOffset(Reg::X, ZeroPageOffset::default()),
                );
                o(0x8d, I::Sta, A::Absolute(Absolute::default()));
                o(
                    0x9d,
                    I::Sta,
                    A::AbsoluteOffset(Reg::X, DummyRead::Always, AbsoluteOffset::default()),
                );
                o(
                    0x99,
                    I::Sta,
                    A::AbsoluteOffset(Reg::Y, DummyRead::Always, AbsoluteOffset::default()),
                );
                o(0x81, I::Sta, A::IndirectX(IndirectX::default()));
                o(
                    0x91,
                    I::Sta,
                    A::IndirectY(DummyRead::Always, IndirectY::default()),
                );

                o(0x86, I::Stx, A::ZeroPage(ZeroPage::default()));
                o(
                    0x96,
                    I::Stx,
                    A::ZeroPageOffset(Reg::Y, ZeroPageOffset::default()),
                );
                o(0x8e, I::Stx, A::Absolute(Absolute::default()));

                o(0x84, I::Sty, A::ZeroPage(ZeroPage::default()));
                o(
                    0x94,
                    I::Sty,
                    A::ZeroPageOffset(Reg::X, ZeroPageOffset::default()),
                );
                o(0x8c, I::Sty, A::Absolute(Absolute::default()));

                o(0x48, I::Pha, A::None);
                o(0x08, I::Php, A::None);
                o(0x68, I::Pla(DummyReadExec::default()), A::None);
                o(0x28, I::Plp(DummyReadExec::default()), A::None);

                o(0x69, I::Adc(ReadExec::default()), A::Immediate);
                o(
                    0x65,
                    I::Adc(ReadExec::default()),
                    A::ZeroPage(ZeroPage::default()),
                );
                o(
                    0x75,
                    I::Adc(ReadExec::default()),
                    A::ZeroPageOffset(Reg::X, ZeroPageOffset::default()),
                );
                o(
                    0x6d,
                    I::Adc(ReadExec::default()),
                    A::Absolute(Absolute::default()),
                );
                o(
                    0x7d,
                    I::Adc(ReadExec::default()),
                    A::AbsoluteOffset(Reg::X, DummyRead::OnCarry, AbsoluteOffset::default()),
                );
                o(
                    0x79,
                    I::Adc(ReadExec::default()),
                    A::AbsoluteOffset(Reg::Y, DummyRead::OnCarry, AbsoluteOffset::default()),
                );
                o(
                    0x61,
                    I::Adc(ReadExec::default()),
                    A::IndirectX(IndirectX::default()),
                );
                o(
                    0x71,
                    I::Adc(ReadExec::default()),
                    A::IndirectY(DummyRead::OnCarry, IndirectY::default()),
                );

                o(0xe9, I::Sbc(ReadExec::default()), A::Immediate);
                o(
                    0xe5,
                    I::Sbc(ReadExec::default()),
                    A::ZeroPage(ZeroPage::default()),
                );
                o(
                    0xf5,
                    I::Sbc(ReadExec::default()),
                    A::ZeroPageOffset(Reg::X, ZeroPageOffset::default()),
                );
                o(
                    0xed,
                    I::Sbc(ReadExec::default()),
                    A::Absolute(Absolute::default()),
                );
                o(
                    0xfd,
                    I::Sbc(ReadExec::default()),
                    A::AbsoluteOffset(Reg::X, DummyRead::OnCarry, AbsoluteOffset::default()),
                );
                o(
                    0xf9,
                    I::Sbc(ReadExec::default()),
                    A::AbsoluteOffset(Reg::Y, DummyRead::OnCarry, AbsoluteOffset::default()),
                );
                o(
                    0xe1,
                    I::Sbc(ReadExec::default()),
                    A::IndirectX(IndirectX::default()),
                );
                o(
                    0xf1,
                    I::Sbc(ReadExec::default()),
                    A::IndirectY(DummyRead::OnCarry, IndirectY::default()),
                );

                o(0x29, I::And(ReadExec::default()), A::Immediate);
                o(
                    0x25,
                    I::And(ReadExec::default()),
                    A::ZeroPage(ZeroPage::default()),
                );
                o(
                    0x35,
                    I::And(ReadExec::default()),
                    A::ZeroPageOffset(Reg::X, ZeroPageOffset::default()),
                );
                o(
                    0x2d,
                    I::And(ReadExec::default()),
                    A::Absolute(Absolute::default()),
                );
                o(
                    0x3d,
                    I::And(ReadExec::default()),
                    A::AbsoluteOffset(Reg::X, DummyRead::OnCarry, AbsoluteOffset::default()),
                );
                o(
                    0x39,
                    I::And(ReadExec::default()),
                    A::AbsoluteOffset(Reg::Y, DummyRead::OnCarry, AbsoluteOffset::default()),
                );
                o(
                    0x21,
                    I::And(ReadExec::default()),
                    A::IndirectX(IndirectX::default()),
                );
                o(
                    0x31,
                    I::And(ReadExec::default()),
                    A::IndirectY(DummyRead::OnCarry, IndirectY::default()),
                );

                o(0x49, I::Eor(ReadExec::default()), A::Immediate);
                o(
                    0x45,
                    I::Eor(ReadExec::default()),
                    A::ZeroPage(ZeroPage::default()),
                );
                o(
                    0x55,
                    I::Eor(ReadExec::default()),
                    A::ZeroPageOffset(Reg::X, ZeroPageOffset::default()),
                );
                o(
                    0x4d,
                    I::Eor(ReadExec::default()),
                    A::Absolute(Absolute::default()),
                );
                o(
                    0x5d,
                    I::Eor(ReadExec::default()),
                    A::AbsoluteOffset(Reg::X, DummyRead::OnCarry, AbsoluteOffset::default()),
                );
                o(
                    0x59,
                    I::Eor(ReadExec::default()),
                    A::AbsoluteOffset(Reg::Y, DummyRead::OnCarry, AbsoluteOffset::default()),
                );
                o(
                    0x41,
                    I::Eor(ReadExec::default()),
                    A::IndirectX(IndirectX::default()),
                );
                o(
                    0x51,
                    I::Eor(ReadExec::default()),
                    A::IndirectY(DummyRead::OnCarry, IndirectY::default()),
                );

                o(0x09, I::Ora(ReadExec::default()), A::Immediate);
                o(
                    0x05,
                    I::Ora(ReadExec::default()),
                    A::ZeroPage(ZeroPage::default()),
                );
                o(
                    0x15,
                    I::Ora(ReadExec::default()),
                    A::ZeroPageOffset(Reg::X, ZeroPageOffset::default()),
                );
                o(
                    0x0d,
                    I::Ora(ReadExec::default()),
                    A::Absolute(Absolute::default()),
                );
                o(
                    0x1d,
                    I::Ora(ReadExec::default()),
                    A::AbsoluteOffset(Reg::X, DummyRead::OnCarry, AbsoluteOffset::default()),
                );
                o(
                    0x19,
                    I::Ora(ReadExec::default()),
                    A::AbsoluteOffset(Reg::Y, DummyRead::OnCarry, AbsoluteOffset::default()),
                );
                o(
                    0x01,
                    I::Ora(ReadExec::default()),
                    A::IndirectX(IndirectX::default()),
                );
                o(
                    0x11,
                    I::Ora(ReadExec::default()),
                    A::IndirectY(DummyRead::OnCarry, IndirectY::default()),
                );

                o(0xc9, I::Cmp(ReadExec::default()), A::Immediate);
                o(
                    0xc5,
                    I::Cmp(ReadExec::default()),
                    A::ZeroPage(ZeroPage::default()),
                );
                o(
                    0xd5,
                    I::Cmp(ReadExec::default()),
                    A::ZeroPageOffset(Reg::X, ZeroPageOffset::default()),
                );
                o(
                    0xcd,
                    I::Cmp(ReadExec::default()),
                    A::Absolute(Absolute::default()),
                );
                o(
                    0xdd,
                    I::Cmp(ReadExec::default()),
                    A::AbsoluteOffset(Reg::X, DummyRead::OnCarry, AbsoluteOffset::default()),
                );
                o(
                    0xd9,
                    I::Cmp(ReadExec::default()),
                    A::AbsoluteOffset(Reg::Y, DummyRead::OnCarry, AbsoluteOffset::default()),
                );
                o(
                    0xc1,
                    I::Cmp(ReadExec::default()),
                    A::IndirectX(IndirectX::default()),
                );
                o(
                    0xd1,
                    I::Cmp(ReadExec::default()),
                    A::IndirectY(DummyRead::OnCarry, IndirectY::default()),
                );

                o(0xe0, I::Cpx(ReadExec::default()), A::Immediate);
                o(
                    0xe4,
                    I::Cpx(ReadExec::default()),
                    A::ZeroPage(ZeroPage::default()),
                );
                o(
                    0xec,
                    I::Cpx(ReadExec::default()),
                    A::Absolute(Absolute::default()),
                );

                o(0xc0, I::Cpy(ReadExec::default()), A::Immediate);
                o(
                    0xc4,
                    I::Cpy(ReadExec::default()),
                    A::ZeroPage(ZeroPage::default()),
                );
                o(
                    0xcc,
                    I::Cpy(ReadExec::default()),
                    A::Absolute(Absolute::default()),
                );

                o(
                    0x24,
                    I::Bit(ReadExec::default()),
                    A::ZeroPage(ZeroPage::default()),
                );
                o(
                    0x2c,
                    I::Bit(ReadExec::default()),
                    A::Absolute(Absolute::default()),
                );

                o(
                    0xe6,
                    I::Inc(ReadDummyExec::default()),
                    A::ZeroPage(ZeroPage::default()),
                );
                o(
                    0xf6,
                    I::Inc(ReadDummyExec::default()),
                    A::ZeroPageOffset(Reg::X, ZeroPageOffset::default()),
                );
                o(
                    0xee,
                    I::Inc(ReadDummyExec::default()),
                    A::Absolute(Absolute::default()),
                );
                o(
                    0xfe,
                    I::Inc(ReadDummyExec::default()),
                    A::AbsoluteOffset(Reg::X, DummyRead::Always, AbsoluteOffset::default()),
                );

                o(0xe8, I::Inx, A::None);
                o(0xc8, I::Iny, A::None);

                o(
                    0xc6,
                    I::Dec(ReadDummyExec::default()),
                    A::ZeroPage(ZeroPage::default()),
                );
                o(
                    0xd6,
                    I::Dec(ReadDummyExec::default()),
                    A::ZeroPageOffset(Reg::X, ZeroPageOffset::default()),
                );
                o(
                    0xce,
                    I::Dec(ReadDummyExec::default()),
                    A::Absolute(Absolute::default()),
                );
                o(
                    0xde,
                    I::Dec(ReadDummyExec::default()),
                    A::AbsoluteOffset(Reg::X, DummyRead::Always, AbsoluteOffset::default()),
                );

                o(0xca, I::Dex, A::None);
                o(0x88, I::Dey, A::None);

                o(0x0a, I::Asla, A::Accumulator);
                o(
                    0x06,
                    I::Asl(ReadDummyExec::default()),
                    A::ZeroPage(ZeroPage::default()),
                );
                o(
                    0x16,
                    I::Asl(ReadDummyExec::default()),
                    A::ZeroPageOffset(Reg::X, ZeroPageOffset::default()),
                );
                o(
                    0x0e,
                    I::Asl(ReadDummyExec::default()),
                    A::Absolute(Absolute::default()),
                );
                o(
                    0x1e,
                    I::Asl(ReadDummyExec::default()),
                    A::AbsoluteOffset(Reg::X, DummyRead::Always, AbsoluteOffset::default()),
                );

                o(0x4a, I::Lsra, A::Accumulator);
                o(
                    0x46,
                    I::Lsr(ReadDummyExec::default()),
                    A::ZeroPage(ZeroPage::default()),
                );
                o(
                    0x56,
                    I::Lsr(ReadDummyExec::default()),
                    A::ZeroPageOffset(Reg::X, ZeroPageOffset::default()),
                );
                o(
                    0x4e,
                    I::Lsr(ReadDummyExec::default()),
                    A::Absolute(Absolute::default()),
                );
                o(
                    0x5e,
                    I::Lsr(ReadDummyExec::default()),
                    A::AbsoluteOffset(Reg::X, DummyRead::Always, AbsoluteOffset::default()),
                );

                o(0x2a, I::Rola, A::Accumulator);
                o(
                    0x26,
                    I::Rol(ReadDummyExec::default()),
                    A::ZeroPage(ZeroPage::default()),
                );
                o(
                    0x36,
                    I::Rol(ReadDummyExec::default()),
                    A::ZeroPageOffset(Reg::X, ZeroPageOffset::default()),
                );
                o(
                    0x2e,
                    I::Rol(ReadDummyExec::default()),
                    A::Absolute(Absolute::default()),
                );
                o(
                    0x3e,
                    I::Rol(ReadDummyExec::default()),
                    A::AbsoluteOffset(Reg::X, DummyRead::Always, AbsoluteOffset::default()),
                );

                o(0x6a, I::Rora, A::Accumulator);
                o(
                    0x66,
                    I::Ror(ReadDummyExec::default()),
                    A::ZeroPage(ZeroPage::default()),
                );
                o(
                    0x76,
                    I::Ror(ReadDummyExec::default()),
                    A::ZeroPageOffset(Reg::X, ZeroPageOffset::default()),
                );
                o(
                    0x6e,
                    I::Ror(ReadDummyExec::default()),
                    A::Absolute(Absolute::default()),
                );
                o(
                    0x7e,
                    I::Ror(ReadDummyExec::default()),
                    A::AbsoluteOffset(Reg::X, DummyRead::Always, AbsoluteOffset::default()),
                );

                o(0x4c, I::Jmp, A::Absolute(Absolute::default()));
                o(
                    0x6c,
                    I::Jmp,
                    A::IndirectAbsolute(IndirectAbsolute::default()),
                );
                o(
                    0x20,
                    I::Jsr(Jsr::default()),
                    A::Absolute(Absolute::default()),
                );
                o(0x40, I::Rti(Rti::default()), A::None);
                o(0x60, I::Rts(Rts::default()), A::None);

                o(
                    0x10,
                    I::Bpl(Branch::default()),
                    A::Relative(Relative::default()),
                );
                o(
                    0x30,
                    I::Bmi(Branch::default()),
                    A::Relative(Relative::default()),
                );
                o(
                    0x50,
                    I::Bvc(Branch::default()),
                    A::Relative(Relative::default()),
                );
                o(
                    0x70,
                    I::Bvs(Branch::default()),
                    A::Relative(Relative::default()),
                );
                o(
                    0x90,
                    I::Bcc(Branch::default()),
                    A::Relative(Relative::default()),
                );
                o(
                    0xb0,
                    I::Bcs(Branch::default()),
                    A::Relative(Relative::default()),
                );
                o(
                    0xd0,
                    I::Bne(Branch::default()),
                    A::Relative(Relative::default()),
                );
                o(
                    0xf0,
                    I::Beq(Branch::default()),
                    A::Relative(Relative::default()),
                );

                o(0x00, I::Brk(Break::default()), A::Immediate);

                o(0x18, I::Clc, A::None);
                o(0x58, I::Cli, A::None);
                o(0xd8, I::Cld, A::None);
                o(0xb8, I::Clv, A::None);
                o(0x38, I::Sec, A::None);
                o(0x78, I::Sei, A::None);
                o(0xf8, I::Sed, A::None);

                o(0xea, I::Nop, A::None);

                //Illegals
                o(0x87, I::IllSax, A::ZeroPage(ZeroPage::default()));
                o(
                    0x97,
                    I::IllSax,
                    A::ZeroPageOffset(Reg::Y, ZeroPageOffset::default()),
                );
                o(0x8f, I::IllSax, A::Absolute(Absolute::default()));
                o(0x83, I::IllSax, A::IndirectX(IndirectX::default()));

                o(
                    0xa7,
                    I::IllLax(ReadExec::default()),
                    A::ZeroPage(ZeroPage::default()),
                );
                o(
                    0xb7,
                    I::IllLax(ReadExec::default()),
                    A::ZeroPageOffset(Reg::Y, ZeroPageOffset::default()),
                );
                o(
                    0xaf,
                    I::IllLax(ReadExec::default()),
                    A::Absolute(Absolute::default()),
                );
                o(
                    0xbf,
                    I::IllLax(ReadExec::default()),
                    A::AbsoluteOffset(Reg::Y, DummyRead::OnCarry, AbsoluteOffset::default()),
                );
                o(
                    0xa3,
                    I::IllLax(ReadExec::default()),
                    A::IndirectX(IndirectX::default()),
                );
                o(
                    0xb3,
                    I::IllLax(ReadExec::default()),
                    A::IndirectY(DummyRead::OnCarry, IndirectY::default()),
                );

                o(
                    0x07,
                    I::IllSlo(ReadDummyExec::default()),
                    A::ZeroPage(ZeroPage::default()),
                );
                o(
                    0x17,
                    I::IllSlo(ReadDummyExec::default()),
                    A::ZeroPageOffset(Reg::X, ZeroPageOffset::default()),
                );
                o(
                    0x0f,
                    I::IllSlo(ReadDummyExec::default()),
                    A::Absolute(Absolute::default()),
                );
                o(
                    0x1f,
                    I::IllSlo(ReadDummyExec::default()),
                    A::AbsoluteOffset(Reg::X, DummyRead::Always, AbsoluteOffset::default()),
                );
                o(
                    0x1b,
                    I::IllSlo(ReadDummyExec::default()),
                    A::AbsoluteOffset(Reg::Y, DummyRead::Always, AbsoluteOffset::default()),
                );
                o(
                    0x03,
                    I::IllSlo(ReadDummyExec::default()),
                    A::IndirectX(IndirectX::default()),
                );
                o(
                    0x13,
                    I::IllSlo(ReadDummyExec::default()),
                    A::IndirectY(DummyRead::Always, IndirectY::default()),
                );

                o(
                    0x27,
                    I::IllRla(ReadDummyExec::default()),
                    A::ZeroPage(ZeroPage::default()),
                );
                o(
                    0x37,
                    I::IllRla(ReadDummyExec::default()),
                    A::ZeroPageOffset(Reg::X, ZeroPageOffset::default()),
                );
                o(
                    0x2f,
                    I::IllRla(ReadDummyExec::default()),
                    A::Absolute(Absolute::default()),
                );
                o(
                    0x3f,
                    I::IllRla(ReadDummyExec::default()),
                    A::AbsoluteOffset(Reg::X, DummyRead::Always, AbsoluteOffset::default()),
                );
                o(
                    0x3b,
                    I::IllRla(ReadDummyExec::default()),
                    A::AbsoluteOffset(Reg::Y, DummyRead::Always, AbsoluteOffset::default()),
                );
                o(
                    0x23,
                    I::IllRla(ReadDummyExec::default()),
                    A::IndirectX(IndirectX::default()),
                );
                o(
                    0x33,
                    I::IllRla(ReadDummyExec::default()),
                    A::IndirectY(DummyRead::Always, IndirectY::default()),
                );

                o(
                    0x47,
                    I::IllSre(ReadDummyExec::default()),
                    A::ZeroPage(ZeroPage::default()),
                );
                o(
                    0x57,
                    I::IllSre(ReadDummyExec::default()),
                    A::ZeroPageOffset(Reg::X, ZeroPageOffset::default()),
                );
                o(
                    0x4f,
                    I::IllSre(ReadDummyExec::default()),
                    A::Absolute(Absolute::default()),
                );
                o(
                    0x5f,
                    I::IllSre(ReadDummyExec::default()),
                    A::AbsoluteOffset(Reg::X, DummyRead::Always, AbsoluteOffset::default()),
                );
                o(
                    0x5b,
                    I::IllSre(ReadDummyExec::default()),
                    A::AbsoluteOffset(Reg::Y, DummyRead::Always, AbsoluteOffset::default()),
                );
                o(
                    0x43,
                    I::IllSre(ReadDummyExec::default()),
                    A::IndirectX(IndirectX::default()),
                );
                o(
                    0x53,
                    I::IllSre(ReadDummyExec::default()),
                    A::IndirectY(DummyRead::Always, IndirectY::default()),
                );

                o(
                    0x67,
                    I::IllRra(ReadDummyExec::default()),
                    A::ZeroPage(ZeroPage::default()),
                );
                o(
                    0x77,
                    I::IllRra(ReadDummyExec::default()),
                    A::ZeroPageOffset(Reg::X, ZeroPageOffset::default()),
                );
                o(
                    0x6f,
                    I::IllRra(ReadDummyExec::default()),
                    A::Absolute(Absolute::default()),
                );
                o(
                    0x7f,
                    I::IllRra(ReadDummyExec::default()),
                    A::AbsoluteOffset(Reg::X, DummyRead::Always, AbsoluteOffset::default()),
                );
                o(
                    0x7b,
                    I::IllRra(ReadDummyExec::default()),
                    A::AbsoluteOffset(Reg::Y, DummyRead::Always, AbsoluteOffset::default()),
                );
                o(
                    0x63,
                    I::IllRra(ReadDummyExec::default()),
                    A::IndirectX(IndirectX::default()),
                );
                o(
                    0x73,
                    I::IllRra(ReadDummyExec::default()),
                    A::IndirectY(DummyRead::Always, IndirectY::default()),
                );

                o(
                    0xc7,
                    I::IllDcp(ReadDummyExec::default()),
                    A::ZeroPage(ZeroPage::default()),
                );
                o(
                    0xd7,
                    I::IllDcp(ReadDummyExec::default()),
                    A::ZeroPageOffset(Reg::X, ZeroPageOffset::default()),
                );
                o(
                    0xcf,
                    I::IllDcp(ReadDummyExec::default()),
                    A::Absolute(Absolute::default()),
                );
                o(
                    0xdf,
                    I::IllDcp(ReadDummyExec::default()),
                    A::AbsoluteOffset(Reg::X, DummyRead::Always, AbsoluteOffset::default()),
                );
                o(
                    0xdb,
                    I::IllDcp(ReadDummyExec::default()),
                    A::AbsoluteOffset(Reg::Y, DummyRead::Always, AbsoluteOffset::default()),
                );
                o(
                    0xc3,
                    I::IllDcp(ReadDummyExec::default()),
                    A::IndirectX(IndirectX::default()),
                );
                o(
                    0xd3,
                    I::IllDcp(ReadDummyExec::default()),
                    A::IndirectY(DummyRead::Always, IndirectY::default()),
                );

                o(
                    0xe7,
                    I::IllIsc(ReadDummyExec::default()),
                    A::ZeroPage(ZeroPage::default()),
                );
                o(
                    0xf7,
                    I::IllIsc(ReadDummyExec::default()),
                    A::ZeroPageOffset(Reg::X, ZeroPageOffset::default()),
                );
                o(
                    0xef,
                    I::IllIsc(ReadDummyExec::default()),
                    A::Absolute(Absolute::default()),
                );
                o(
                    0xff,
                    I::IllIsc(ReadDummyExec::default()),
                    A::AbsoluteOffset(Reg::X, DummyRead::Always, AbsoluteOffset::default()),
                );
                o(
                    0xfb,
                    I::IllIsc(ReadDummyExec::default()),
                    A::AbsoluteOffset(Reg::Y, DummyRead::Always, AbsoluteOffset::default()),
                );
                o(
                    0xe3,
                    I::IllIsc(ReadDummyExec::default()),
                    A::IndirectX(IndirectX::default()),
                );
                o(
                    0xf3,
                    I::IllIsc(ReadDummyExec::default()),
                    A::IndirectY(DummyRead::Always, IndirectY::default()),
                );

                o(0x0b, I::IllAnc(ReadExec::default()), A::Immediate);
                o(0x2b, I::IllAnc(ReadExec::default()), A::Immediate);
                o(0x4b, I::IllAlr(ReadExec::default()), A::Immediate);
                o(0x6b, I::IllArr(ReadExec::default()), A::Immediate);
                o(0x8b, I::IllXaa(ReadExec::default()), A::Immediate);
                o(0xab, I::IllLax(ReadExec::default()), A::Immediate);
                o(0xcb, I::IllAxs(ReadExec::default()), A::Immediate);
                o(0xeb, I::IllSbc(ReadExec::default()), A::Immediate);
                o(
                    0x93,
                    I::IllAhx,
                    A::IndirectY(DummyRead::Always, IndirectY::default()),
                );
                o(
                    0x9f,
                    I::IllAhx,
                    A::AbsoluteOffset(Reg::Y, DummyRead::Always, AbsoluteOffset::default()),
                );
                o(
                    0x9c,
                    I::IllShy,
                    A::AbsoluteOffset(Reg::X, DummyRead::Always, AbsoluteOffset::default()),
                );
                o(
                    0x9e,
                    I::IllShx,
                    A::AbsoluteOffset(Reg::Y, DummyRead::Always, AbsoluteOffset::default()),
                );
                o(
                    0x9b,
                    I::IllTas,
                    A::AbsoluteOffset(Reg::Y, DummyRead::Always, AbsoluteOffset::default()),
                );
                o(
                    0xbb,
                    I::IllLas,
                    A::AbsoluteOffset(Reg::Y, DummyRead::OnCarry, AbsoluteOffset::default()),
                );

                o(0x1a, I::IllNop, A::None);
                o(0x3a, I::IllNop, A::None);
                o(0x5a, I::IllNop, A::None);
                o(0x7a, I::IllNop, A::None);
                o(0xda, I::IllNop, A::None);
                o(0xfa, I::IllNop, A::None);

                o(0x80, I::IllNopAddr, A::Immediate);
                o(0x82, I::IllNopAddr, A::Immediate);
                o(0x89, I::IllNopAddr, A::Immediate);
                o(0xc2, I::IllNopAddr, A::Immediate);
                o(0xe2, I::IllNopAddr, A::Immediate);

                o(0x04, I::IllNopAddr, A::ZeroPage(ZeroPage::default()));
                o(0x44, I::IllNopAddr, A::ZeroPage(ZeroPage::default()));
                o(0x64, I::IllNopAddr, A::ZeroPage(ZeroPage::default()));

                o(
                    0x14,
                    I::IllNopAddr,
                    A::ZeroPageOffset(Reg::X, ZeroPageOffset::default()),
                );
                o(
                    0x34,
                    I::IllNopAddr,
                    A::ZeroPageOffset(Reg::X, ZeroPageOffset::default()),
                );
                o(
                    0x54,
                    I::IllNopAddr,
                    A::ZeroPageOffset(Reg::X, ZeroPageOffset::default()),
                );
                o(
                    0x74,
                    I::IllNopAddr,
                    A::ZeroPageOffset(Reg::X, ZeroPageOffset::default()),
                );
                o(
                    0xd4,
                    I::IllNopAddr,
                    A::ZeroPageOffset(Reg::X, ZeroPageOffset::default()),
                );
                o(
                    0xf4,
                    I::IllNopAddr,
                    A::ZeroPageOffset(Reg::X, ZeroPageOffset::default()),
                );

                o(0x0c, I::IllNopAddr, A::Absolute(Absolute::default()));

                o(
                    0x1c,
                    I::IllNopAddr,
                    A::AbsoluteOffset(Reg::X, DummyRead::OnCarry, AbsoluteOffset::default()),
                );
                o(
                    0x3c,
                    I::IllNopAddr,
                    A::AbsoluteOffset(Reg::X, DummyRead::OnCarry, AbsoluteOffset::default()),
                );
                o(
                    0x5c,
                    I::IllNopAddr,
                    A::AbsoluteOffset(Reg::X, DummyRead::OnCarry, AbsoluteOffset::default()),
                );
                o(
                    0x7c,
                    I::IllNopAddr,
                    A::AbsoluteOffset(Reg::X, DummyRead::OnCarry, AbsoluteOffset::default()),
                );
                o(
                    0xdc,
                    I::IllNopAddr,
                    A::AbsoluteOffset(Reg::X, DummyRead::OnCarry, AbsoluteOffset::default()),
                );
                o(
                    0xfc,
                    I::IllNopAddr,
                    A::AbsoluteOffset(Reg::X, DummyRead::OnCarry, AbsoluteOffset::default()),
                );

                o(0x02, I::IllKil, A::None);
                o(0x12, I::IllKil, A::None);
                o(0x22, I::IllKil, A::None);
                o(0x32, I::IllKil, A::None);
                o(0x42, I::IllKil, A::None);
                o(0x52, I::IllKil, A::None);
                o(0x62, I::IllKil, A::None);
                o(0x72, I::IllKil, A::None);
                o(0x92, I::IllKil, A::None);
                o(0xb2, I::IllKil, A::None);
                o(0xd2, I::IllKil, A::None);
                o(0xf2, I::IllKil, A::None);
            }

            for i in 0..0x100 {
                if !op_set.contains(&i) {
                    panic!("Missing instruction: {}", i);
                }
            }

            &OPS
        }
    }
}
