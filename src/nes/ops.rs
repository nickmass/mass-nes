use std::collections::HashMap;

#[derive(Clone, Copy)]
pub enum Instruction {
    Adc,
    And,
    Asl,
    Bit,
    Bcc,
    Bcs,
    Beq,
    Bmi,
    Bne,
    Bpl,
    Brk,
    Bvc,
    Bvs,
    Clc,
    Cld,
    Cli,
    Clv,
    Cmp,
    Cpx,
    Cpy,
    Dec,
    Dex,
    Dey,
    Eor,
    Inc,
    Inx,
    Iny,
    Jmp,
    Jsr,
    Lda,
    Ldx,
    Ldy,
    Lsr,
    Nop,
    Ora,
    Pha,
    Php,
    Pla,
    Plp,
    Rol,
    Ror,
    Rti,
    Rts,
    Sbc,
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
    IllAnc,
    IllAlr,
    IllArr,
    IllAxs,
    IllDcp,
    IllIsc,
    IllKil,
    IllLas,
    IllLax,
    IllNop,
    IllRla,
    IllRra,
    IllSax,
    IllSbc,
    IllShx,
    IllShy,
    IllSlo,
    IllSre,
    IllTas,
    IllXaa,

    IllKill,
}

#[derive(Clone, Copy)]
pub enum Addressing {
    None,
    ZeroPage,
    Immediate,
    Accumulator,
    ZeroPageX,
    ZeroPageY,
    Absolute,
    AbsoluteX,
    AbsoluteY,
    IndirectAbsolute,
    Relative,
    IndirectX,
    IndirectY,
    AbsoluteXDummyAlways,
    AbsoluteYDummyAlways,
    IndirectYDummyAlways,
}

#[derive(Clone, Copy)]
pub struct Op {
    pub instruction: Instruction,
    pub addressing: Addressing
}

impl Default for Op {
    fn default() -> Op {
        Op { instruction: Instruction::Nop, addressing: Addressing::None }
    }
}

impl Op {
    pub fn load() -> HashMap<u8, Op> {
        use self::Instruction as I;
        use self::Addressing as A;
        let mut h = HashMap::new();
        {
            let mut o = |o,i,a| h.insert(o, Op { instruction: i, addressing: a });

            o(0xa8, I::Tay, A::None);
            o(0xaa, I::Tax, A::None);
            o(0xba, I::Tsx, A::None);
            o(0x98, I::Tya, A::None);
            o(0x8a, I::Txa, A::None);
            o(0x9a, I::Txs, A::None);

            o(0xa9, I::Lda, A::Immediate);
            o(0xa5, I::Lda, A::ZeroPage);
            o(0xb5, I::Lda, A::ZeroPageX);
            o(0xad, I::Lda, A::Absolute);
            o(0xbd, I::Lda, A::AbsoluteX);
            o(0xb9, I::Lda, A::AbsoluteY);
            o(0xa1, I::Lda, A::IndirectX);
            o(0xb1, I::Lda, A::IndirectY);
            
            o(0xa2, I::Ldx, A::Immediate);
            o(0xa6, I::Ldx, A::ZeroPage);
            o(0xb6, I::Ldx, A::ZeroPageY);
            o(0xae, I::Ldx, A::Absolute);
            o(0xbe, I::Ldx, A::AbsoluteY);

            o(0xa0, I::Ldy, A::Immediate);
            o(0xa4, I::Ldy, A::ZeroPage);
            o(0xb4, I::Ldy, A::ZeroPageX);
            o(0xac, I::Ldy, A::Absolute);
            o(0xbc, I::Ldy, A::AbsoluteX);
            
            o(0x85, I::Sta, A::ZeroPage);
            o(0x95, I::Sta, A::ZeroPageX);
            o(0x8d, I::Sta, A::Absolute);
            o(0x9d, I::Sta, A::AbsoluteXDummyAlways);
            o(0x99, I::Sta, A::AbsoluteYDummyAlways);
            o(0x81, I::Sta, A::IndirectX);
            o(0x91, I::Sta, A::IndirectYDummyAlways);
            
            o(0x86, I::Stx, A::ZeroPage);
            o(0x96, I::Stx, A::ZeroPageY);
            o(0x8e, I::Stx, A::Absolute);
            
            o(0x84, I::Sty, A::ZeroPage);
            o(0x94, I::Sty, A::ZeroPageX);
            o(0x8c, I::Sty, A::Absolute);
            
            o(0x48, I::Pha, A::None);
            o(0x08, I::Php, A::None);
            o(0x68, I::Pla, A::None);
            o(0x28, I::Plp, A::None);

            o(0x69, I::Adc, A::Immediate);
            o(0x65, I::Adc, A::ZeroPage);
            o(0x75, I::Adc, A::ZeroPageX);
            o(0x6d, I::Adc, A::Absolute);
            o(0x7d, I::Adc, A::AbsoluteX);
            o(0x79, I::Adc, A::AbsoluteY);
            o(0x61, I::Adc, A::IndirectX);
            o(0x71, I::Adc, A::IndirectY);
            
            o(0xe9, I::Sbc, A::Immediate);
            o(0xe5, I::Sbc, A::ZeroPage);
            o(0xf5, I::Sbc, A::ZeroPageX);
            o(0xed, I::Sbc, A::Absolute);
            o(0xfd, I::Sbc, A::AbsoluteX);
            o(0xf9, I::Sbc, A::AbsoluteY);
            o(0xe1, I::Sbc, A::IndirectX);
            o(0xf1, I::Sbc, A::IndirectY);
            
            o(0x29, I::And, A::Immediate);
            o(0x25, I::And, A::ZeroPage);
            o(0x35, I::And, A::ZeroPageX);
            o(0x2d, I::And, A::Absolute);
            o(0x3d, I::And, A::AbsoluteX);
            o(0x39, I::And, A::AbsoluteY);
            o(0x21, I::And, A::IndirectX);
            o(0x31, I::And, A::IndirectY);
            
            o(0x49, I::Eor, A::Immediate);
            o(0x45, I::Eor, A::ZeroPage);
            o(0x55, I::Eor, A::ZeroPageX);
            o(0x4d, I::Eor, A::Absolute);
            o(0x5d, I::Eor, A::AbsoluteX);
            o(0x59, I::Eor, A::AbsoluteY);
            o(0x41, I::Eor, A::IndirectX);
            o(0x51, I::Eor, A::IndirectY);
            
            o(0x09, I::Ora, A::Immediate);
            o(0x05, I::Ora, A::ZeroPage);
            o(0x15, I::Ora, A::ZeroPageX);
            o(0x0d, I::Ora, A::Absolute);
            o(0x1d, I::Ora, A::AbsoluteX);
            o(0x19, I::Ora, A::AbsoluteY);
            o(0x01, I::Ora, A::IndirectX);
            o(0x11, I::Ora, A::IndirectY);
            
            o(0xc9, I::Cmp, A::Immediate);
            o(0xc5, I::Cmp, A::ZeroPage);
            o(0xd5, I::Cmp, A::ZeroPageX);
            o(0xcd, I::Cmp, A::Absolute);
            o(0xdd, I::Cmp, A::AbsoluteX);
            o(0xd9, I::Cmp, A::AbsoluteY);
            o(0xc1, I::Cmp, A::IndirectX);
            o(0xd1, I::Cmp, A::IndirectY);
            
            o(0xe0, I::Cpx, A::Immediate);
            o(0xe4, I::Cpx, A::ZeroPage);
            o(0xec, I::Cpx, A::Absolute);
            
            o(0xc0, I::Cpy, A::Immediate);
            o(0xc4, I::Cpy, A::ZeroPage);
            o(0xcc, I::Cpy, A::Absolute);
            
            o(0x24, I::Bit, A::ZeroPage);
            o(0x2c, I::Bit, A::Absolute);

            o(0xe6, I::Inc, A::ZeroPage);
            o(0xf6, I::Inc, A::ZeroPageX);
            o(0xee, I::Inc, A::Absolute);
            o(0xfe, I::Inc, A::AbsoluteXDummyAlways);
            
            o(0xe8, I::Inx, A::None);
            o(0xc8, I::Iny, A::None);

            o(0xc6, I::Dec, A::ZeroPage);
            o(0xd6, I::Dec, A::ZeroPageX);
            o(0xce, I::Dec, A::Absolute);
            o(0xde, I::Dec, A::AbsoluteXDummyAlways);
            
            o(0xca, I::Dex, A::None);
            o(0x88, I::Dey, A::None);

            o(0x0a, I::Asl, A::Accumulator);
            o(0x06, I::Asl, A::ZeroPage);
            o(0x16, I::Asl, A::ZeroPageX);
            o(0x0e, I::Asl, A::Absolute);
            o(0x1e, I::Asl, A::AbsoluteXDummyAlways);
            
            o(0x4a, I::Lsr, A::Accumulator);
            o(0x46, I::Lsr, A::ZeroPage);
            o(0x56, I::Lsr, A::ZeroPageX);
            o(0x4e, I::Lsr, A::Absolute);
            o(0x5e, I::Lsr, A::AbsoluteXDummyAlways);
            
            o(0x2a, I::Rol, A::Accumulator);
            o(0x26, I::Rol, A::ZeroPage);
            o(0x36, I::Rol, A::ZeroPageX);
            o(0x2e, I::Rol, A::Absolute);
            o(0x3e, I::Rol, A::AbsoluteXDummyAlways);
            
            o(0x6a, I::Ror, A::Accumulator);
            o(0x66, I::Ror, A::ZeroPage);
            o(0x76, I::Ror, A::ZeroPageX);
            o(0x6e, I::Ror, A::Absolute);
            o(0x7e, I::Ror, A::AbsoluteXDummyAlways);
            
            o(0x4c, I::Jmp, A::Absolute);
            o(0x6c, I::Jmp, A::IndirectAbsolute);
            o(0x20, I::Jsr, A::Absolute);
            o(0x40, I::Rti, A::None);
            o(0x60, I::Rts, A::None);

            o(0x10, I::Bpl, A::Relative);
            o(0x30, I::Bmi, A::Relative);
            o(0x50, I::Bvc, A::Relative);
            o(0x70, I::Bvs, A::Relative);
            o(0x90, I::Bcc, A::Relative);
            o(0xb0, I::Bcs, A::Relative);
            o(0xd0, I::Bne, A::Relative);
            o(0xf0, I::Beq, A::Relative);
            
            o(0x00, I::Brk, A::Immediate);

            o(0x18, I::Clc, A::None);
            o(0x58, I::Cli, A::None);
            o(0xd8, I::Cld, A::None);
            o(0xb8, I::Clv, A::None);
            o(0x38, I::Sec, A::None);
            o(0x78, I::Sei, A::None);
            o(0xf8, I::Sed, A::None);
            
            o(0xea, I::Nop, A::None);


            //Illegals
            o(0x87, I::IllSax, A::ZeroPage);
            o(0x97, I::IllSax, A::ZeroPageY);
            o(0x8f, I::IllSax, A::Absolute);
            o(0x83, I::IllSax, A::IndirectX);

            o(0xa7, I::IllLax, A::ZeroPage);
            o(0xb7, I::IllLax, A::ZeroPageY);
            o(0xaf, I::IllLax, A::Absolute);
            o(0xbf, I::IllLax, A::AbsoluteY);
            o(0xa3, I::IllLax, A::IndirectX);
            o(0xb3, I::IllLax, A::IndirectY);

            o(0x07, I::IllSlo, A::ZeroPage);
            o(0x17, I::IllSlo, A::ZeroPageX);
            o(0x0f, I::IllSlo, A::Absolute);
            o(0x1f, I::IllSlo, A::AbsoluteXDummyAlways);
            o(0x1b, I::IllSlo, A::AbsoluteYDummyAlways);
            o(0x03, I::IllSlo, A::IndirectX);
            o(0x13, I::IllSlo, A::IndirectYDummyAlways);

            o(0x27, I::IllRla, A::ZeroPage);
            o(0x37, I::IllRla, A::ZeroPageX);
            o(0x2f, I::IllRla, A::Absolute);
            o(0x3f, I::IllRla, A::AbsoluteXDummyAlways);
            o(0x3b, I::IllRla, A::AbsoluteYDummyAlways);
            o(0x23, I::IllRla, A::IndirectX);
            o(0x33, I::IllRla, A::IndirectYDummyAlways);
            
            o(0x47, I::IllSre, A::ZeroPage);
            o(0x57, I::IllSre, A::ZeroPageX);
            o(0x4f, I::IllSre, A::Absolute);
            o(0x5f, I::IllSre, A::AbsoluteXDummyAlways);
            o(0x5b, I::IllSre, A::AbsoluteYDummyAlways);
            o(0x43, I::IllSre, A::IndirectX);
            o(0x53, I::IllSre, A::IndirectYDummyAlways);
            
            o(0x67, I::IllRra, A::ZeroPage);
            o(0x77, I::IllRra, A::ZeroPageX);
            o(0x6f, I::IllRra, A::Absolute);
            o(0x7f, I::IllRra, A::AbsoluteXDummyAlways);
            o(0x7b, I::IllRra, A::AbsoluteYDummyAlways);
            o(0x63, I::IllRra, A::IndirectX);
            o(0x73, I::IllRra, A::IndirectYDummyAlways);
            
            o(0xc7, I::IllDcp, A::ZeroPage);
            o(0xd7, I::IllDcp, A::ZeroPageX);
            o(0xcf, I::IllDcp, A::Absolute);
            o(0xdf, I::IllDcp, A::AbsoluteXDummyAlways);
            o(0xdb, I::IllDcp, A::AbsoluteYDummyAlways);
            o(0xc3, I::IllDcp, A::IndirectX);
            o(0xd3, I::IllDcp, A::IndirectYDummyAlways);
            
            o(0xe7, I::IllIsc, A::ZeroPage);
            o(0xf7, I::IllIsc, A::ZeroPageX);
            o(0xef, I::IllIsc, A::Absolute);
            o(0xff, I::IllIsc, A::AbsoluteXDummyAlways);
            o(0xfb, I::IllIsc, A::AbsoluteYDummyAlways);
            o(0xe3, I::IllIsc, A::IndirectX);
            o(0xf3, I::IllIsc, A::IndirectYDummyAlways);
            
            o(0x0b, I::IllAnc, A::Immediate);
            o(0x2b, I::IllAnc, A::Immediate);
            o(0x4b, I::IllAlr, A::Immediate);
            o(0x6b, I::IllArr, A::Immediate);
            o(0x8b, I::IllXaa, A::Immediate);
            o(0xab, I::IllLax, A::Immediate);
            o(0xcb, I::IllAxs, A::Immediate);
            o(0xeb, I::IllSbc, A::Immediate);
            o(0x93, I::IllAhx, A::IndirectYDummyAlways);
            o(0x9f, I::IllAhx, A::AbsoluteYDummyAlways);
            o(0x9c, I::IllShy, A::AbsoluteXDummyAlways);
            o(0x9e, I::IllShx, A::AbsoluteYDummyAlways);
            o(0x9b, I::IllTas, A::AbsoluteYDummyAlways);
            o(0xbb, I::IllLas, A::AbsoluteY);
            
            o(0x1a, I::IllNop, A::None);
            o(0x3a, I::IllNop, A::None);
            o(0x5a, I::IllNop, A::None);
            o(0x7a, I::IllNop, A::None);
            o(0xda, I::IllNop, A::None);
            o(0xfa, I::IllNop, A::None);

            o(0x80, I::IllNop, A::Immediate);
            o(0x82, I::IllNop, A::Immediate);
            o(0x89, I::IllNop, A::Immediate);
            o(0xc2, I::IllNop, A::Immediate);
            o(0xe2, I::IllNop, A::Immediate);

            o(0x04, I::IllNop, A::ZeroPage);
            o(0x44, I::IllNop, A::ZeroPage);
            o(0x64, I::IllNop, A::ZeroPage);

            o(0x14, I::IllNop, A::ZeroPageX);
            o(0x34, I::IllNop, A::ZeroPageX);
            o(0x54, I::IllNop, A::ZeroPageX);
            o(0x74, I::IllNop, A::ZeroPageX);
            o(0xd4, I::IllNop, A::ZeroPageX);
            o(0xf4, I::IllNop, A::ZeroPageX);

            o(0x0c, I::IllNop, A::Absolute);

            o(0x1c, I::IllNop, A::AbsoluteX);
            o(0x3c, I::IllNop, A::AbsoluteX);
            o(0x5c, I::IllNop, A::AbsoluteX);
            o(0x7c, I::IllNop, A::AbsoluteX);
            o(0xdc, I::IllNop, A::AbsoluteX);
            o(0xfc, I::IllNop, A::AbsoluteX);

            o(0x02, I::IllKill, A::None);
            o(0x12, I::IllKill, A::None);
            o(0x22, I::IllKill, A::None);
            o(0x32, I::IllKill, A::None);
            o(0x42, I::IllKill, A::None);
            o(0x52, I::IllKill, A::None);
            o(0x62, I::IllKill, A::None);
            o(0x72, I::IllKill, A::None);
            o(0x92, I::IllKill, A::None);
            o(0xb2, I::IllKill, A::None);
            o(0xd2, I::IllKill, A::None);
            o(0xf2, I::IllKill, A::None);
        }

        for i in 0..255 { 
            assert!(h.contains_key(&i), format!("Missing Op: {:x}", i));
        }

        h
    }
}
