pub const OPS: [Op; 0x100] = Op::load();

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

#[derive(Debug, Clone, Copy)]
pub enum ReadDummyExec {
    Read,
    Dummy,
    Exec(u8),
}

#[derive(Debug, Clone, Copy)]
pub enum Branch {
    Check,
    Branch,
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

#[derive(Debug, Clone, Copy)]
pub enum Jsr {
    ReadDummy,
    WriteRegPcHigh,
    WriteRegPcLow,
}

#[derive(Debug, Clone, Copy)]
pub enum DummyReadExec {
    Dummy,
    Read,
    Exec,
}

#[derive(Debug, Clone, Copy)]
pub enum Rti {
    Dummy,
    ReadRegP,
    ReadRegPcLow,
    ReadRegPcHigh,
    Exec(u16),
}

#[derive(Debug, Clone, Copy)]
pub enum Rts {
    Dummy,
    ReadRegPcLow,
    ReadRegPcHigh,
    Exec(u16),
}

#[derive(Debug, Clone, Copy)]
pub enum ZeroPage {
    Read,
    Decode,
}

#[derive(Debug, Clone, Copy)]
pub enum AbsoluteOffset {
    ReadLow,
    ReadHigh,
    Decode(u16),
}

#[derive(Debug, Clone, Copy)]
pub enum Absolute {
    ReadLow,
    ReadHigh,
    Decode(u16),
}

#[derive(Debug, Clone, Copy)]
pub enum ZeroPageOffset {
    ReadImmediate,
    ApplyOffset,
}

#[derive(Debug, Clone, Copy)]
pub enum IndirectAbsolute {
    ReadLow,
    ReadHigh,
    ReadIndirectLow(u16),
    ReadIndirectHigh(u16),
    Decode(u16),
}

#[derive(Debug, Clone, Copy)]
pub enum Relative {
    ReadRegPc,
    Decode,
}

#[derive(Debug, Clone, Copy)]
pub enum IndirectX {
    ReadBase,
    ReadDummy,
    ReadIndirectLow(u16),
    ReadIndirectHigh(u16),
    Decode(u16),
}

#[derive(Debug, Clone, Copy)]
pub enum IndirectY {
    ReadBase,
    ReadZeroPageLow,
    ReadZeroPageHigh(u16),
    Decode(u16),
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

macro_rules! set_op (
    ($ops:ident, $op:literal, $instruction:expr, $addressing:expr,) => {
        set_op!($ops, $op, $instruction, $addressing);
    };
    ($ops:ident, $op:literal, $instruction:expr, $addressing:expr) => {
        $ops[$op] = Op {
            instruction: $instruction,
            addressing: $addressing
        };
    };
);

impl Op {
    const fn load() -> [Op; 0x100] {
        use self::Addressing as A;
        use self::Instruction as I;

        let mut ops = [Op {
            instruction: I::Nop,
            addressing: A::None,
        }; 0x100];

        set_op!(ops, 0xa8, I::Tay, A::None);
        set_op!(ops, 0xaa, I::Tax, A::None);
        set_op!(ops, 0xba, I::Tsx, A::None);
        set_op!(ops, 0x98, I::Tya, A::None);
        set_op!(ops, 0x8a, I::Txa, A::None);
        set_op!(ops, 0x9a, I::Txs, A::None);

        set_op!(ops, 0xa9, I::Lda(ReadExec::Read), A::Immediate);
        set_op!(
            ops,
            0xa5,
            I::Lda(ReadExec::Read),
            A::ZeroPage(ZeroPage::Read),
        );
        set_op!(
            ops,
            0xb5,
            I::Lda(ReadExec::Read),
            A::ZeroPageOffset(Reg::X, ZeroPageOffset::ReadImmediate),
        );
        set_op!(
            ops,
            0xad,
            I::Lda(ReadExec::Read),
            A::Absolute(Absolute::ReadLow),
        );
        set_op!(
            ops,
            0xbd,
            I::Lda(ReadExec::Read),
            A::AbsoluteOffset(Reg::X, DummyRead::OnCarry, AbsoluteOffset::ReadLow),
        );
        set_op!(
            ops,
            0xb9,
            I::Lda(ReadExec::Read),
            A::AbsoluteOffset(Reg::Y, DummyRead::OnCarry, AbsoluteOffset::ReadLow),
        );
        set_op!(
            ops,
            0xa1,
            I::Lda(ReadExec::Read),
            A::IndirectX(IndirectX::ReadBase),
        );
        set_op!(
            ops,
            0xb1,
            I::Lda(ReadExec::Read),
            A::IndirectY(DummyRead::OnCarry, IndirectY::ReadBase),
        );

        set_op!(ops, 0xa2, I::Ldx(ReadExec::Read), A::Immediate);
        set_op!(
            ops,
            0xa6,
            I::Ldx(ReadExec::Read),
            A::ZeroPage(ZeroPage::Read),
        );
        set_op!(
            ops,
            0xb6,
            I::Ldx(ReadExec::Read),
            A::ZeroPageOffset(Reg::Y, ZeroPageOffset::ReadImmediate),
        );
        set_op!(
            ops,
            0xae,
            I::Ldx(ReadExec::Read),
            A::Absolute(Absolute::ReadLow),
        );
        set_op!(
            ops,
            0xbe,
            I::Ldx(ReadExec::Read),
            A::AbsoluteOffset(Reg::Y, DummyRead::OnCarry, AbsoluteOffset::ReadLow),
        );

        set_op!(ops, 0xa0, I::Ldy(ReadExec::Read), A::Immediate);
        set_op!(
            ops,
            0xa4,
            I::Ldy(ReadExec::Read),
            A::ZeroPage(ZeroPage::Read),
        );
        set_op!(
            ops,
            0xb4,
            I::Ldy(ReadExec::Read),
            A::ZeroPageOffset(Reg::X, ZeroPageOffset::ReadImmediate),
        );
        set_op!(
            ops,
            0xac,
            I::Ldy(ReadExec::Read),
            A::Absolute(Absolute::ReadLow),
        );
        set_op!(
            ops,
            0xbc,
            I::Ldy(ReadExec::Read),
            A::AbsoluteOffset(Reg::X, DummyRead::OnCarry, AbsoluteOffset::ReadLow),
        );

        set_op!(ops, 0x85, I::Sta, A::ZeroPage(ZeroPage::Read));
        set_op!(
            ops,
            0x95,
            I::Sta,
            A::ZeroPageOffset(Reg::X, ZeroPageOffset::ReadImmediate),
        );
        set_op!(ops, 0x8d, I::Sta, A::Absolute(Absolute::ReadLow));
        set_op!(
            ops,
            0x9d,
            I::Sta,
            A::AbsoluteOffset(Reg::X, DummyRead::Always, AbsoluteOffset::ReadLow),
        );
        set_op!(
            ops,
            0x99,
            I::Sta,
            A::AbsoluteOffset(Reg::Y, DummyRead::Always, AbsoluteOffset::ReadLow),
        );
        set_op!(ops, 0x81, I::Sta, A::IndirectX(IndirectX::ReadBase));
        set_op!(
            ops,
            0x91,
            I::Sta,
            A::IndirectY(DummyRead::Always, IndirectY::ReadBase),
        );

        set_op!(ops, 0x86, I::Stx, A::ZeroPage(ZeroPage::Read));
        set_op!(
            ops,
            0x96,
            I::Stx,
            A::ZeroPageOffset(Reg::Y, ZeroPageOffset::ReadImmediate),
        );
        set_op!(ops, 0x8e, I::Stx, A::Absolute(Absolute::ReadLow));

        set_op!(ops, 0x84, I::Sty, A::ZeroPage(ZeroPage::Read));
        set_op!(
            ops,
            0x94,
            I::Sty,
            A::ZeroPageOffset(Reg::X, ZeroPageOffset::ReadImmediate),
        );
        set_op!(ops, 0x8c, I::Sty, A::Absolute(Absolute::ReadLow));

        set_op!(ops, 0x48, I::Pha, A::None);
        set_op!(ops, 0x08, I::Php, A::None);
        set_op!(ops, 0x68, I::Pla(DummyReadExec::Dummy), A::None);
        set_op!(ops, 0x28, I::Plp(DummyReadExec::Dummy), A::None);

        set_op!(ops, 0x69, I::Adc(ReadExec::Read), A::Immediate);
        set_op!(
            ops,
            0x65,
            I::Adc(ReadExec::Read),
            A::ZeroPage(ZeroPage::Read),
        );
        set_op!(
            ops,
            0x75,
            I::Adc(ReadExec::Read),
            A::ZeroPageOffset(Reg::X, ZeroPageOffset::ReadImmediate),
        );
        set_op!(
            ops,
            0x6d,
            I::Adc(ReadExec::Read),
            A::Absolute(Absolute::ReadLow),
        );
        set_op!(
            ops,
            0x7d,
            I::Adc(ReadExec::Read),
            A::AbsoluteOffset(Reg::X, DummyRead::OnCarry, AbsoluteOffset::ReadLow),
        );
        set_op!(
            ops,
            0x79,
            I::Adc(ReadExec::Read),
            A::AbsoluteOffset(Reg::Y, DummyRead::OnCarry, AbsoluteOffset::ReadLow),
        );
        set_op!(
            ops,
            0x61,
            I::Adc(ReadExec::Read),
            A::IndirectX(IndirectX::ReadBase),
        );
        set_op!(
            ops,
            0x71,
            I::Adc(ReadExec::Read),
            A::IndirectY(DummyRead::OnCarry, IndirectY::ReadBase),
        );

        set_op!(ops, 0xe9, I::Sbc(ReadExec::Read), A::Immediate);
        set_op!(
            ops,
            0xe5,
            I::Sbc(ReadExec::Read),
            A::ZeroPage(ZeroPage::Read),
        );
        set_op!(
            ops,
            0xf5,
            I::Sbc(ReadExec::Read),
            A::ZeroPageOffset(Reg::X, ZeroPageOffset::ReadImmediate),
        );
        set_op!(
            ops,
            0xed,
            I::Sbc(ReadExec::Read),
            A::Absolute(Absolute::ReadLow),
        );
        set_op!(
            ops,
            0xfd,
            I::Sbc(ReadExec::Read),
            A::AbsoluteOffset(Reg::X, DummyRead::OnCarry, AbsoluteOffset::ReadLow),
        );
        set_op!(
            ops,
            0xf9,
            I::Sbc(ReadExec::Read),
            A::AbsoluteOffset(Reg::Y, DummyRead::OnCarry, AbsoluteOffset::ReadLow),
        );
        set_op!(
            ops,
            0xe1,
            I::Sbc(ReadExec::Read),
            A::IndirectX(IndirectX::ReadBase),
        );
        set_op!(
            ops,
            0xf1,
            I::Sbc(ReadExec::Read),
            A::IndirectY(DummyRead::OnCarry, IndirectY::ReadBase),
        );

        set_op!(ops, 0x29, I::And(ReadExec::Read), A::Immediate);
        set_op!(
            ops,
            0x25,
            I::And(ReadExec::Read),
            A::ZeroPage(ZeroPage::Read),
        );
        set_op!(
            ops,
            0x35,
            I::And(ReadExec::Read),
            A::ZeroPageOffset(Reg::X, ZeroPageOffset::ReadImmediate),
        );
        set_op!(
            ops,
            0x2d,
            I::And(ReadExec::Read),
            A::Absolute(Absolute::ReadLow),
        );
        set_op!(
            ops,
            0x3d,
            I::And(ReadExec::Read),
            A::AbsoluteOffset(Reg::X, DummyRead::OnCarry, AbsoluteOffset::ReadLow),
        );
        set_op!(
            ops,
            0x39,
            I::And(ReadExec::Read),
            A::AbsoluteOffset(Reg::Y, DummyRead::OnCarry, AbsoluteOffset::ReadLow),
        );
        set_op!(
            ops,
            0x21,
            I::And(ReadExec::Read),
            A::IndirectX(IndirectX::ReadBase),
        );
        set_op!(
            ops,
            0x31,
            I::And(ReadExec::Read),
            A::IndirectY(DummyRead::OnCarry, IndirectY::ReadBase),
        );

        set_op!(ops, 0x49, I::Eor(ReadExec::Read), A::Immediate);
        set_op!(
            ops,
            0x45,
            I::Eor(ReadExec::Read),
            A::ZeroPage(ZeroPage::Read),
        );
        set_op!(
            ops,
            0x55,
            I::Eor(ReadExec::Read),
            A::ZeroPageOffset(Reg::X, ZeroPageOffset::ReadImmediate),
        );
        set_op!(
            ops,
            0x4d,
            I::Eor(ReadExec::Read),
            A::Absolute(Absolute::ReadLow),
        );
        set_op!(
            ops,
            0x5d,
            I::Eor(ReadExec::Read),
            A::AbsoluteOffset(Reg::X, DummyRead::OnCarry, AbsoluteOffset::ReadLow),
        );
        set_op!(
            ops,
            0x59,
            I::Eor(ReadExec::Read),
            A::AbsoluteOffset(Reg::Y, DummyRead::OnCarry, AbsoluteOffset::ReadLow),
        );
        set_op!(
            ops,
            0x41,
            I::Eor(ReadExec::Read),
            A::IndirectX(IndirectX::ReadBase),
        );
        set_op!(
            ops,
            0x51,
            I::Eor(ReadExec::Read),
            A::IndirectY(DummyRead::OnCarry, IndirectY::ReadBase),
        );

        set_op!(ops, 0x09, I::Ora(ReadExec::Read), A::Immediate);
        set_op!(
            ops,
            0x05,
            I::Ora(ReadExec::Read),
            A::ZeroPage(ZeroPage::Read),
        );
        set_op!(
            ops,
            0x15,
            I::Ora(ReadExec::Read),
            A::ZeroPageOffset(Reg::X, ZeroPageOffset::ReadImmediate),
        );
        set_op!(
            ops,
            0x0d,
            I::Ora(ReadExec::Read),
            A::Absolute(Absolute::ReadLow),
        );
        set_op!(
            ops,
            0x1d,
            I::Ora(ReadExec::Read),
            A::AbsoluteOffset(Reg::X, DummyRead::OnCarry, AbsoluteOffset::ReadLow),
        );
        set_op!(
            ops,
            0x19,
            I::Ora(ReadExec::Read),
            A::AbsoluteOffset(Reg::Y, DummyRead::OnCarry, AbsoluteOffset::ReadLow),
        );
        set_op!(
            ops,
            0x01,
            I::Ora(ReadExec::Read),
            A::IndirectX(IndirectX::ReadBase),
        );
        set_op!(
            ops,
            0x11,
            I::Ora(ReadExec::Read),
            A::IndirectY(DummyRead::OnCarry, IndirectY::ReadBase),
        );

        set_op!(ops, 0xc9, I::Cmp(ReadExec::Read), A::Immediate);
        set_op!(
            ops,
            0xc5,
            I::Cmp(ReadExec::Read),
            A::ZeroPage(ZeroPage::Read),
        );
        set_op!(
            ops,
            0xd5,
            I::Cmp(ReadExec::Read),
            A::ZeroPageOffset(Reg::X, ZeroPageOffset::ReadImmediate),
        );
        set_op!(
            ops,
            0xcd,
            I::Cmp(ReadExec::Read),
            A::Absolute(Absolute::ReadLow),
        );
        set_op!(
            ops,
            0xdd,
            I::Cmp(ReadExec::Read),
            A::AbsoluteOffset(Reg::X, DummyRead::OnCarry, AbsoluteOffset::ReadLow),
        );
        set_op!(
            ops,
            0xd9,
            I::Cmp(ReadExec::Read),
            A::AbsoluteOffset(Reg::Y, DummyRead::OnCarry, AbsoluteOffset::ReadLow),
        );
        set_op!(
            ops,
            0xc1,
            I::Cmp(ReadExec::Read),
            A::IndirectX(IndirectX::ReadBase),
        );
        set_op!(
            ops,
            0xd1,
            I::Cmp(ReadExec::Read),
            A::IndirectY(DummyRead::OnCarry, IndirectY::ReadBase),
        );

        set_op!(ops, 0xe0, I::Cpx(ReadExec::Read), A::Immediate);
        set_op!(
            ops,
            0xe4,
            I::Cpx(ReadExec::Read),
            A::ZeroPage(ZeroPage::Read),
        );
        set_op!(
            ops,
            0xec,
            I::Cpx(ReadExec::Read),
            A::Absolute(Absolute::ReadLow),
        );

        set_op!(ops, 0xc0, I::Cpy(ReadExec::Read), A::Immediate);
        set_op!(
            ops,
            0xc4,
            I::Cpy(ReadExec::Read),
            A::ZeroPage(ZeroPage::Read),
        );
        set_op!(
            ops,
            0xcc,
            I::Cpy(ReadExec::Read),
            A::Absolute(Absolute::ReadLow),
        );

        set_op!(
            ops,
            0x24,
            I::Bit(ReadExec::Read),
            A::ZeroPage(ZeroPage::Read),
        );
        set_op!(
            ops,
            0x2c,
            I::Bit(ReadExec::Read),
            A::Absolute(Absolute::ReadLow),
        );

        set_op!(
            ops,
            0xe6,
            I::Inc(ReadDummyExec::Read),
            A::ZeroPage(ZeroPage::Read),
        );
        set_op!(
            ops,
            0xf6,
            I::Inc(ReadDummyExec::Read),
            A::ZeroPageOffset(Reg::X, ZeroPageOffset::ReadImmediate),
        );
        set_op!(
            ops,
            0xee,
            I::Inc(ReadDummyExec::Read),
            A::Absolute(Absolute::ReadLow),
        );
        set_op!(
            ops,
            0xfe,
            I::Inc(ReadDummyExec::Read),
            A::AbsoluteOffset(Reg::X, DummyRead::Always, AbsoluteOffset::ReadLow),
        );

        set_op!(ops, 0xe8, I::Inx, A::None);
        set_op!(ops, 0xc8, I::Iny, A::None);

        set_op!(
            ops,
            0xc6,
            I::Dec(ReadDummyExec::Read),
            A::ZeroPage(ZeroPage::Read),
        );
        set_op!(
            ops,
            0xd6,
            I::Dec(ReadDummyExec::Read),
            A::ZeroPageOffset(Reg::X, ZeroPageOffset::ReadImmediate),
        );
        set_op!(
            ops,
            0xce,
            I::Dec(ReadDummyExec::Read),
            A::Absolute(Absolute::ReadLow),
        );
        set_op!(
            ops,
            0xde,
            I::Dec(ReadDummyExec::Read),
            A::AbsoluteOffset(Reg::X, DummyRead::Always, AbsoluteOffset::ReadLow),
        );

        set_op!(ops, 0xca, I::Dex, A::None);
        set_op!(ops, 0x88, I::Dey, A::None);

        set_op!(ops, 0x0a, I::Asla, A::Accumulator);
        set_op!(
            ops,
            0x06,
            I::Asl(ReadDummyExec::Read),
            A::ZeroPage(ZeroPage::Read),
        );
        set_op!(
            ops,
            0x16,
            I::Asl(ReadDummyExec::Read),
            A::ZeroPageOffset(Reg::X, ZeroPageOffset::ReadImmediate),
        );
        set_op!(
            ops,
            0x0e,
            I::Asl(ReadDummyExec::Read),
            A::Absolute(Absolute::ReadLow),
        );
        set_op!(
            ops,
            0x1e,
            I::Asl(ReadDummyExec::Read),
            A::AbsoluteOffset(Reg::X, DummyRead::Always, AbsoluteOffset::ReadLow),
        );

        set_op!(ops, 0x4a, I::Lsra, A::Accumulator);
        set_op!(
            ops,
            0x46,
            I::Lsr(ReadDummyExec::Read),
            A::ZeroPage(ZeroPage::Read),
        );
        set_op!(
            ops,
            0x56,
            I::Lsr(ReadDummyExec::Read),
            A::ZeroPageOffset(Reg::X, ZeroPageOffset::ReadImmediate),
        );
        set_op!(
            ops,
            0x4e,
            I::Lsr(ReadDummyExec::Read),
            A::Absolute(Absolute::ReadLow),
        );
        set_op!(
            ops,
            0x5e,
            I::Lsr(ReadDummyExec::Read),
            A::AbsoluteOffset(Reg::X, DummyRead::Always, AbsoluteOffset::ReadLow),
        );

        set_op!(ops, 0x2a, I::Rola, A::Accumulator);
        set_op!(
            ops,
            0x26,
            I::Rol(ReadDummyExec::Read),
            A::ZeroPage(ZeroPage::Read),
        );
        set_op!(
            ops,
            0x36,
            I::Rol(ReadDummyExec::Read),
            A::ZeroPageOffset(Reg::X, ZeroPageOffset::ReadImmediate),
        );
        set_op!(
            ops,
            0x2e,
            I::Rol(ReadDummyExec::Read),
            A::Absolute(Absolute::ReadLow),
        );
        set_op!(
            ops,
            0x3e,
            I::Rol(ReadDummyExec::Read),
            A::AbsoluteOffset(Reg::X, DummyRead::Always, AbsoluteOffset::ReadLow),
        );

        set_op!(ops, 0x6a, I::Rora, A::Accumulator);
        set_op!(
            ops,
            0x66,
            I::Ror(ReadDummyExec::Read),
            A::ZeroPage(ZeroPage::Read),
        );
        set_op!(
            ops,
            0x76,
            I::Ror(ReadDummyExec::Read),
            A::ZeroPageOffset(Reg::X, ZeroPageOffset::ReadImmediate),
        );
        set_op!(
            ops,
            0x6e,
            I::Ror(ReadDummyExec::Read),
            A::Absolute(Absolute::ReadLow),
        );
        set_op!(
            ops,
            0x7e,
            I::Ror(ReadDummyExec::Read),
            A::AbsoluteOffset(Reg::X, DummyRead::Always, AbsoluteOffset::ReadLow),
        );

        set_op!(ops, 0x4c, I::Jmp, A::Absolute(Absolute::ReadLow));
        set_op!(
            ops,
            0x6c,
            I::Jmp,
            A::IndirectAbsolute(IndirectAbsolute::ReadLow),
        );
        set_op!(
            ops,
            0x20,
            I::Jsr(Jsr::ReadDummy),
            A::Absolute(Absolute::ReadLow),
        );
        set_op!(ops, 0x40, I::Rti(Rti::Dummy), A::None);
        set_op!(ops, 0x60, I::Rts(Rts::Dummy), A::None);

        set_op!(
            ops,
            0x10,
            I::Bpl(Branch::Check),
            A::Relative(Relative::ReadRegPc),
        );
        set_op!(
            ops,
            0x30,
            I::Bmi(Branch::Check),
            A::Relative(Relative::ReadRegPc),
        );
        set_op!(
            ops,
            0x50,
            I::Bvc(Branch::Check),
            A::Relative(Relative::ReadRegPc),
        );
        set_op!(
            ops,
            0x70,
            I::Bvs(Branch::Check),
            A::Relative(Relative::ReadRegPc),
        );
        set_op!(
            ops,
            0x90,
            I::Bcc(Branch::Check),
            A::Relative(Relative::ReadRegPc),
        );
        set_op!(
            ops,
            0xb0,
            I::Bcs(Branch::Check),
            A::Relative(Relative::ReadRegPc),
        );
        set_op!(
            ops,
            0xd0,
            I::Bne(Branch::Check),
            A::Relative(Relative::ReadRegPc),
        );
        set_op!(
            ops,
            0xf0,
            I::Beq(Branch::Check),
            A::Relative(Relative::ReadRegPc),
        );

        set_op!(ops, 0x00, I::Brk(Break::ReadDummy), A::Immediate);

        set_op!(ops, 0x18, I::Clc, A::None);
        set_op!(ops, 0x58, I::Cli, A::None);
        set_op!(ops, 0xd8, I::Cld, A::None);
        set_op!(ops, 0xb8, I::Clv, A::None);
        set_op!(ops, 0x38, I::Sec, A::None);
        set_op!(ops, 0x78, I::Sei, A::None);
        set_op!(ops, 0xf8, I::Sed, A::None);

        set_op!(ops, 0xea, I::Nop, A::None);

        //Illegals
        set_op!(ops, 0x87, I::IllSax, A::ZeroPage(ZeroPage::Read));
        set_op!(
            ops,
            0x97,
            I::IllSax,
            A::ZeroPageOffset(Reg::Y, ZeroPageOffset::ReadImmediate),
        );
        set_op!(ops, 0x8f, I::IllSax, A::Absolute(Absolute::ReadLow));
        set_op!(ops, 0x83, I::IllSax, A::IndirectX(IndirectX::ReadBase));

        set_op!(
            ops,
            0xa7,
            I::IllLax(ReadExec::Read),
            A::ZeroPage(ZeroPage::Read),
        );
        set_op!(
            ops,
            0xb7,
            I::IllLax(ReadExec::Read),
            A::ZeroPageOffset(Reg::Y, ZeroPageOffset::ReadImmediate),
        );
        set_op!(
            ops,
            0xaf,
            I::IllLax(ReadExec::Read),
            A::Absolute(Absolute::ReadLow),
        );
        set_op!(
            ops,
            0xbf,
            I::IllLax(ReadExec::Read),
            A::AbsoluteOffset(Reg::Y, DummyRead::OnCarry, AbsoluteOffset::ReadLow),
        );
        set_op!(
            ops,
            0xa3,
            I::IllLax(ReadExec::Read),
            A::IndirectX(IndirectX::ReadBase),
        );
        set_op!(
            ops,
            0xb3,
            I::IllLax(ReadExec::Read),
            A::IndirectY(DummyRead::OnCarry, IndirectY::ReadBase),
        );

        set_op!(
            ops,
            0x07,
            I::IllSlo(ReadDummyExec::Read),
            A::ZeroPage(ZeroPage::Read),
        );
        set_op!(
            ops,
            0x17,
            I::IllSlo(ReadDummyExec::Read),
            A::ZeroPageOffset(Reg::X, ZeroPageOffset::ReadImmediate),
        );
        set_op!(
            ops,
            0x0f,
            I::IllSlo(ReadDummyExec::Read),
            A::Absolute(Absolute::ReadLow),
        );
        set_op!(
            ops,
            0x1f,
            I::IllSlo(ReadDummyExec::Read),
            A::AbsoluteOffset(Reg::X, DummyRead::Always, AbsoluteOffset::ReadLow),
        );
        set_op!(
            ops,
            0x1b,
            I::IllSlo(ReadDummyExec::Read),
            A::AbsoluteOffset(Reg::Y, DummyRead::Always, AbsoluteOffset::ReadLow),
        );
        set_op!(
            ops,
            0x03,
            I::IllSlo(ReadDummyExec::Read),
            A::IndirectX(IndirectX::ReadBase),
        );
        set_op!(
            ops,
            0x13,
            I::IllSlo(ReadDummyExec::Read),
            A::IndirectY(DummyRead::Always, IndirectY::ReadBase),
        );

        set_op!(
            ops,
            0x27,
            I::IllRla(ReadDummyExec::Read),
            A::ZeroPage(ZeroPage::Read),
        );
        set_op!(
            ops,
            0x37,
            I::IllRla(ReadDummyExec::Read),
            A::ZeroPageOffset(Reg::X, ZeroPageOffset::ReadImmediate),
        );
        set_op!(
            ops,
            0x2f,
            I::IllRla(ReadDummyExec::Read),
            A::Absolute(Absolute::ReadLow),
        );
        set_op!(
            ops,
            0x3f,
            I::IllRla(ReadDummyExec::Read),
            A::AbsoluteOffset(Reg::X, DummyRead::Always, AbsoluteOffset::ReadLow),
        );
        set_op!(
            ops,
            0x3b,
            I::IllRla(ReadDummyExec::Read),
            A::AbsoluteOffset(Reg::Y, DummyRead::Always, AbsoluteOffset::ReadLow),
        );
        set_op!(
            ops,
            0x23,
            I::IllRla(ReadDummyExec::Read),
            A::IndirectX(IndirectX::ReadBase),
        );
        set_op!(
            ops,
            0x33,
            I::IllRla(ReadDummyExec::Read),
            A::IndirectY(DummyRead::Always, IndirectY::ReadBase),
        );

        set_op!(
            ops,
            0x47,
            I::IllSre(ReadDummyExec::Read),
            A::ZeroPage(ZeroPage::Read),
        );
        set_op!(
            ops,
            0x57,
            I::IllSre(ReadDummyExec::Read),
            A::ZeroPageOffset(Reg::X, ZeroPageOffset::ReadImmediate),
        );
        set_op!(
            ops,
            0x4f,
            I::IllSre(ReadDummyExec::Read),
            A::Absolute(Absolute::ReadLow),
        );
        set_op!(
            ops,
            0x5f,
            I::IllSre(ReadDummyExec::Read),
            A::AbsoluteOffset(Reg::X, DummyRead::Always, AbsoluteOffset::ReadLow),
        );
        set_op!(
            ops,
            0x5b,
            I::IllSre(ReadDummyExec::Read),
            A::AbsoluteOffset(Reg::Y, DummyRead::Always, AbsoluteOffset::ReadLow),
        );
        set_op!(
            ops,
            0x43,
            I::IllSre(ReadDummyExec::Read),
            A::IndirectX(IndirectX::ReadBase),
        );
        set_op!(
            ops,
            0x53,
            I::IllSre(ReadDummyExec::Read),
            A::IndirectY(DummyRead::Always, IndirectY::ReadBase),
        );

        set_op!(
            ops,
            0x67,
            I::IllRra(ReadDummyExec::Read),
            A::ZeroPage(ZeroPage::Read),
        );
        set_op!(
            ops,
            0x77,
            I::IllRra(ReadDummyExec::Read),
            A::ZeroPageOffset(Reg::X, ZeroPageOffset::ReadImmediate),
        );
        set_op!(
            ops,
            0x6f,
            I::IllRra(ReadDummyExec::Read),
            A::Absolute(Absolute::ReadLow),
        );
        set_op!(
            ops,
            0x7f,
            I::IllRra(ReadDummyExec::Read),
            A::AbsoluteOffset(Reg::X, DummyRead::Always, AbsoluteOffset::ReadLow),
        );
        set_op!(
            ops,
            0x7b,
            I::IllRra(ReadDummyExec::Read),
            A::AbsoluteOffset(Reg::Y, DummyRead::Always, AbsoluteOffset::ReadLow),
        );
        set_op!(
            ops,
            0x63,
            I::IllRra(ReadDummyExec::Read),
            A::IndirectX(IndirectX::ReadBase),
        );
        set_op!(
            ops,
            0x73,
            I::IllRra(ReadDummyExec::Read),
            A::IndirectY(DummyRead::Always, IndirectY::ReadBase),
        );

        set_op!(
            ops,
            0xc7,
            I::IllDcp(ReadDummyExec::Read),
            A::ZeroPage(ZeroPage::Read),
        );
        set_op!(
            ops,
            0xd7,
            I::IllDcp(ReadDummyExec::Read),
            A::ZeroPageOffset(Reg::X, ZeroPageOffset::ReadImmediate),
        );
        set_op!(
            ops,
            0xcf,
            I::IllDcp(ReadDummyExec::Read),
            A::Absolute(Absolute::ReadLow),
        );
        set_op!(
            ops,
            0xdf,
            I::IllDcp(ReadDummyExec::Read),
            A::AbsoluteOffset(Reg::X, DummyRead::Always, AbsoluteOffset::ReadLow),
        );
        set_op!(
            ops,
            0xdb,
            I::IllDcp(ReadDummyExec::Read),
            A::AbsoluteOffset(Reg::Y, DummyRead::Always, AbsoluteOffset::ReadLow),
        );
        set_op!(
            ops,
            0xc3,
            I::IllDcp(ReadDummyExec::Read),
            A::IndirectX(IndirectX::ReadBase),
        );
        set_op!(
            ops,
            0xd3,
            I::IllDcp(ReadDummyExec::Read),
            A::IndirectY(DummyRead::Always, IndirectY::ReadBase),
        );

        set_op!(
            ops,
            0xe7,
            I::IllIsc(ReadDummyExec::Read),
            A::ZeroPage(ZeroPage::Read),
        );
        set_op!(
            ops,
            0xf7,
            I::IllIsc(ReadDummyExec::Read),
            A::ZeroPageOffset(Reg::X, ZeroPageOffset::ReadImmediate),
        );
        set_op!(
            ops,
            0xef,
            I::IllIsc(ReadDummyExec::Read),
            A::Absolute(Absolute::ReadLow),
        );
        set_op!(
            ops,
            0xff,
            I::IllIsc(ReadDummyExec::Read),
            A::AbsoluteOffset(Reg::X, DummyRead::Always, AbsoluteOffset::ReadLow),
        );
        set_op!(
            ops,
            0xfb,
            I::IllIsc(ReadDummyExec::Read),
            A::AbsoluteOffset(Reg::Y, DummyRead::Always, AbsoluteOffset::ReadLow),
        );
        set_op!(
            ops,
            0xe3,
            I::IllIsc(ReadDummyExec::Read),
            A::IndirectX(IndirectX::ReadBase),
        );
        set_op!(
            ops,
            0xf3,
            I::IllIsc(ReadDummyExec::Read),
            A::IndirectY(DummyRead::Always, IndirectY::ReadBase),
        );

        set_op!(ops, 0x0b, I::IllAnc(ReadExec::Read), A::Immediate);
        set_op!(ops, 0x2b, I::IllAnc(ReadExec::Read), A::Immediate);
        set_op!(ops, 0x4b, I::IllAlr(ReadExec::Read), A::Immediate);
        set_op!(ops, 0x6b, I::IllArr(ReadExec::Read), A::Immediate);
        set_op!(ops, 0x8b, I::IllXaa(ReadExec::Read), A::Immediate);
        set_op!(ops, 0xab, I::IllLax(ReadExec::Read), A::Immediate);
        set_op!(ops, 0xcb, I::IllAxs(ReadExec::Read), A::Immediate);
        set_op!(ops, 0xeb, I::IllSbc(ReadExec::Read), A::Immediate);
        set_op!(
            ops,
            0x93,
            I::IllAhx,
            A::IndirectY(DummyRead::Always, IndirectY::ReadBase),
        );
        set_op!(
            ops,
            0x9f,
            I::IllAhx,
            A::AbsoluteOffset(Reg::Y, DummyRead::Always, AbsoluteOffset::ReadLow),
        );
        set_op!(
            ops,
            0x9c,
            I::IllShy,
            A::AbsoluteOffset(Reg::X, DummyRead::Always, AbsoluteOffset::ReadLow),
        );
        set_op!(
            ops,
            0x9e,
            I::IllShx,
            A::AbsoluteOffset(Reg::Y, DummyRead::Always, AbsoluteOffset::ReadLow),
        );
        set_op!(
            ops,
            0x9b,
            I::IllTas,
            A::AbsoluteOffset(Reg::Y, DummyRead::Always, AbsoluteOffset::ReadLow),
        );
        set_op!(
            ops,
            0xbb,
            I::IllLas,
            A::AbsoluteOffset(Reg::Y, DummyRead::OnCarry, AbsoluteOffset::ReadLow),
        );

        set_op!(ops, 0x1a, I::IllNop, A::None);
        set_op!(ops, 0x3a, I::IllNop, A::None);
        set_op!(ops, 0x5a, I::IllNop, A::None);
        set_op!(ops, 0x7a, I::IllNop, A::None);
        set_op!(ops, 0xda, I::IllNop, A::None);
        set_op!(ops, 0xfa, I::IllNop, A::None);

        set_op!(ops, 0x80, I::IllNopAddr, A::Immediate);
        set_op!(ops, 0x82, I::IllNopAddr, A::Immediate);
        set_op!(ops, 0x89, I::IllNopAddr, A::Immediate);
        set_op!(ops, 0xc2, I::IllNopAddr, A::Immediate);
        set_op!(ops, 0xe2, I::IllNopAddr, A::Immediate);

        set_op!(ops, 0x04, I::IllNopAddr, A::ZeroPage(ZeroPage::Read));
        set_op!(ops, 0x44, I::IllNopAddr, A::ZeroPage(ZeroPage::Read));
        set_op!(ops, 0x64, I::IllNopAddr, A::ZeroPage(ZeroPage::Read));

        set_op!(
            ops,
            0x14,
            I::IllNopAddr,
            A::ZeroPageOffset(Reg::X, ZeroPageOffset::ReadImmediate),
        );
        set_op!(
            ops,
            0x34,
            I::IllNopAddr,
            A::ZeroPageOffset(Reg::X, ZeroPageOffset::ReadImmediate),
        );
        set_op!(
            ops,
            0x54,
            I::IllNopAddr,
            A::ZeroPageOffset(Reg::X, ZeroPageOffset::ReadImmediate),
        );
        set_op!(
            ops,
            0x74,
            I::IllNopAddr,
            A::ZeroPageOffset(Reg::X, ZeroPageOffset::ReadImmediate),
        );
        set_op!(
            ops,
            0xd4,
            I::IllNopAddr,
            A::ZeroPageOffset(Reg::X, ZeroPageOffset::ReadImmediate),
        );
        set_op!(
            ops,
            0xf4,
            I::IllNopAddr,
            A::ZeroPageOffset(Reg::X, ZeroPageOffset::ReadImmediate),
        );

        set_op!(ops, 0x0c, I::IllNopAddr, A::Absolute(Absolute::ReadLow));

        set_op!(
            ops,
            0x1c,
            I::IllNopAddr,
            A::AbsoluteOffset(Reg::X, DummyRead::OnCarry, AbsoluteOffset::ReadLow),
        );
        set_op!(
            ops,
            0x3c,
            I::IllNopAddr,
            A::AbsoluteOffset(Reg::X, DummyRead::OnCarry, AbsoluteOffset::ReadLow),
        );
        set_op!(
            ops,
            0x5c,
            I::IllNopAddr,
            A::AbsoluteOffset(Reg::X, DummyRead::OnCarry, AbsoluteOffset::ReadLow),
        );
        set_op!(
            ops,
            0x7c,
            I::IllNopAddr,
            A::AbsoluteOffset(Reg::X, DummyRead::OnCarry, AbsoluteOffset::ReadLow),
        );
        set_op!(
            ops,
            0xdc,
            I::IllNopAddr,
            A::AbsoluteOffset(Reg::X, DummyRead::OnCarry, AbsoluteOffset::ReadLow),
        );
        set_op!(
            ops,
            0xfc,
            I::IllNopAddr,
            A::AbsoluteOffset(Reg::X, DummyRead::OnCarry, AbsoluteOffset::ReadLow),
        );

        set_op!(ops, 0x02, I::IllKil, A::None);
        set_op!(ops, 0x12, I::IllKil, A::None);
        set_op!(ops, 0x22, I::IllKil, A::None);
        set_op!(ops, 0x32, I::IllKil, A::None);
        set_op!(ops, 0x42, I::IllKil, A::None);
        set_op!(ops, 0x52, I::IllKil, A::None);
        set_op!(ops, 0x62, I::IllKil, A::None);
        set_op!(ops, 0x72, I::IllKil, A::None);
        set_op!(ops, 0x92, I::IllKil, A::None);
        set_op!(ops, 0xb2, I::IllKil, A::None);
        set_op!(ops, 0xd2, I::IllKil, A::None);
        set_op!(ops, 0xf2, I::IllKil, A::None);

        ops
    }
}
