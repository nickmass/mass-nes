use nes::system::{System, SystemState};
use nes::ops::*;
use std::collections::HashMap;

#[derive(Default)]
pub struct DebugState {
    trace_instrs: u32,
}

pub struct Debug {
    ops: HashMap<u8, Op>,
    op_names: HashMap<Instruction, &'static str>,
    op_lengths: HashMap<Addressing, u32>,
}

impl Debug {
    pub fn new() -> Debug {
        Debug {
            ops: Op::load(),
            op_names: Debug::load_op_names(),
            op_lengths: Debug::load_op_lengths(),
        }
    }

    pub fn log_for(&self, state: &mut SystemState, count: u32) {
        state.debug.trace_instrs = count;
    }

    pub fn trace(&self, system: &System, state: &mut SystemState, addr: u16) {
        if state.debug.trace_instrs != 0 {
            let log = self.trace_instruction(system, state, addr);
            println!("{}", log);
            state.debug.trace_instrs -= 1;
        }
    }

    pub fn trace_instruction(&self, system: &System, state: &mut SystemState, addr: u16) -> String {
        let instr = system.cpu.bus.peek(system, state, addr);
       
        let op = self.ops[&instr];
        let name = self.op_names[&op.instruction];
        let len = self.op_lengths[&op.addressing] as u16;

        let mut addr_inc = addr;
        let read = |state, addr| -> u8 {
            system.cpu.bus.peek(system, state, addr)
        };

        let mut read_pc = |state| -> u8 {
            addr_inc = addr_inc.wrapping_add(1);
            system.cpu.bus.peek(system, state, addr_inc)
        };

        let pc_string = format!("{:04X}", addr);

        let instr_bytes_string = {
            let mut buf = String::new();
            for x in 0..len {
                buf.push_str(&*format!(" {:02X}", read(state, x.wrapping_add(addr))));
            }

            buf
        };

        let addr_string = match op.addressing {
            Addressing::None => {
                format!("")
            },
            Addressing::Accumulator => {
                format!("A = {:02X}", state.cpu.reg_a)
            },
            Addressing::Immediate => {
                let r = read_pc(state);
                format!("#${:02X}", r)
            },
            Addressing::ZeroPage => {
                let a = read_pc(state);
                let v = read(state, a as u16);
                format!("${:02X} = {:02X}", a, v)
            },
            Addressing::ZeroPageX => {
                let a1 = read_pc(state) as u16;
                let a2 = a1.wrapping_add(state.cpu.reg_x as u16) & 0xff;
                let v = read(state, a2);
                format!("${:02X},X @ {:04X} = {:02X}", a1, a2, v)
            },
            Addressing::ZeroPageY => {
                let a1 = read_pc(state) as u16;
                let a2 = a1.wrapping_add(state.cpu.reg_y as u16) & 0xff;
                let v = read(state, a2);
                format!("${:02X},X @ {:04X} = {:02X}", a1, a2, v)
            },
            Addressing::Absolute => {
                let a_low = read_pc(state) as u16;
                let a_high = read_pc(state);
                let a = ((a_high as u16) << 8) | a_low;
                let v = read(state, a);
                format!("${:04X} = ${:02X}", a, v)
            },
            Addressing::AbsoluteX(_) => {
                let a_low = read_pc(state) as u16;
                let a_high = read_pc(state);
                let a1 = ((a_high as u16) << 8) | a_low;
                let a2 = a1.wrapping_add(state.cpu.reg_x as u16);
                let v = read(state, a2);
                format!("${:04X},X @ {:04X} = {:02X}", a1, a2, v)
            },
            Addressing::AbsoluteY(_) => {
                let a_low = read_pc(state) as u16;
                let a_high = read_pc(state);
                let a1 = ((a_high as u16) << 8) | a_low;
                let a2 = a1.wrapping_add(state.cpu.reg_y as u16);
                let v = read(state, a2);
                format!("${:04X},X @ {:04X} = {:02X}", a1, a2, v)
            },
            Addressing::IndirectAbsolute => { 
                let a1_low = read_pc(state) as u16;
                let a1_high = read_pc(state);
                let a1 = ((a1_high as u16) << 8) | a1_low;
                let a2_low = read(state, a1) as u16;
                let a2_high = read(state, (a1 & 0xff00) | (a1.wrapping_add(1) & 0xff));
                let a2 = ((a2_high as u16) << 8) | a2_low;
                format!("(${:04X}) @ {:04X}", a1, a2)
            },
            Addressing::Relative => {
                let rel = read_pc(state);
                let a = if rel < 0x080 {
                    addr.wrapping_add(rel as u16)
                } else {
                    addr.wrapping_add(rel as u16).wrapping_sub(256)
                };
                format!("${:04X}", a)
            },
            Addressing::IndirectX => {
                let a1 = read_pc(state) as u16;
                let a2 = a1.wrapping_add(state.cpu.reg_x as u16) & 0xff;
                let a3_low = read(state, a2) as u16;
                let a3_high = read(state, (a2 & 0xff00) | (a2.wrapping_add(1) & 0xff)) as u16;
                let a3 = (a3_high << 8) | a3_low;
                let v = read(state, a3);
                format!("(${:02X},X) @ {:02X} = {:04X} = {:02X}", a1, a2, a3, v)
            },
            Addressing::IndirectY(_) => {
                let a1 = read_pc(state) as u16;
                let a2_low = read(state, a1);
                let a2_high = read(state, (a1 & 0xff00) | (a1.wrapping_add(1) & 0xff));
                let a2 = ((a2_high as u16) << 8) | a2_low as u16;
                let a3 = a2.wrapping_add((state.cpu.reg_y & 0xff) as u16);
                let v = read(state, a3);
                format!("(${:02X}),Y = {:04X} @ {:04X} = {:02X}", a1, a2, a3, v)
            },
        };

        let reg_string = {
            format!("A:{:02X} X:{:02X} Y:{:02X} P:{:02X} SP:{:02X}",
                    state.cpu.reg_a,
                    state.cpu.reg_x,
                    state.cpu.reg_y,
                    state.cpu.reg_p(),
                    state.cpu.reg_sp)
        };

        format!("{}{: <10.10}{: >4.4} {: <30.30} {}",
                pc_string,
                instr_bytes_string,
                name,
                addr_string,
                reg_string)
    }

    fn load_op_lengths() -> HashMap<Addressing, u32> {
        let mut map = HashMap::new();
        map.insert(Addressing::None, 1);
        map.insert(Addressing::Accumulator, 1);
        map.insert(Addressing::Immediate, 2);
        map.insert(Addressing::ZeroPage, 2);
        map.insert(Addressing::ZeroPageX, 2);
        map.insert(Addressing::ZeroPageY, 2);
        map.insert(Addressing::Absolute, 3);
        map.insert(Addressing::AbsoluteX(DummyRead::OnCarry), 3);
        map.insert(Addressing::AbsoluteX(DummyRead::Always), 3);
        map.insert(Addressing::AbsoluteY(DummyRead::OnCarry), 3);
        map.insert(Addressing::AbsoluteY(DummyRead::Always), 3);
        map.insert(Addressing::IndirectAbsolute, 3);
        map.insert(Addressing::Relative, 2);
        map.insert(Addressing::IndirectX, 2);
        map.insert(Addressing::IndirectY(DummyRead::OnCarry), 2);
        map.insert(Addressing::IndirectY(DummyRead::Always), 2);
        map
    }

    fn load_op_names() -> HashMap<Instruction, &'static str> {
        let mut map = HashMap::new();    
        map.insert(Instruction::Adc, "ADC");    
        map.insert(Instruction::And, "AND");
        map.insert(Instruction::Asl, "ASL");
        map.insert(Instruction::Bit, "BIT");
        map.insert(Instruction::Bcc, "BCC");
        map.insert(Instruction::Bcs, "BCS");
        map.insert(Instruction::Beq, "BEQ");
        map.insert(Instruction::Bmi, "BMI");
        map.insert(Instruction::Bne, "BNE");
        map.insert(Instruction::Bpl, "BPL");
        map.insert(Instruction::Brk, "BRK");
        map.insert(Instruction::Bvc, "BVC");
        map.insert(Instruction::Bvs, "BVS");
        map.insert(Instruction::Clc, "CLC");
        map.insert(Instruction::Cld, "CLD");
        map.insert(Instruction::Cli, "CLI");
        map.insert(Instruction::Clv, "CLV");
        map.insert(Instruction::Cmp, "CMP");
        map.insert(Instruction::Cpx, "CPX");
        map.insert(Instruction::Cpy, "CPY");
        map.insert(Instruction::Dec, "DEC");
        map.insert(Instruction::Dex, "DEX");
        map.insert(Instruction::Dey, "DEY");
        map.insert(Instruction::Eor, "EOR");
        map.insert(Instruction::Inc, "INC");
        map.insert(Instruction::Inx, "INX");
        map.insert(Instruction::Iny, "INY");
        map.insert(Instruction::Jmp, "JMP");
        map.insert(Instruction::Jsr, "JSR");
        map.insert(Instruction::Lda, "LDA");
        map.insert(Instruction::Ldx, "LDX");
        map.insert(Instruction::Ldy, "LDY");
        map.insert(Instruction::Lsr, "LSR");
        map.insert(Instruction::Nop, "NOP");
        map.insert(Instruction::Ora, "ORA");
        map.insert(Instruction::Pha, "PHA");
        map.insert(Instruction::Php, "PHP");
        map.insert(Instruction::Pla, "PLA");
        map.insert(Instruction::Plp, "PLP");
        map.insert(Instruction::Rol, "ROL");
        map.insert(Instruction::Ror, "ROR");
        map.insert(Instruction::Rti, "RTI");
        map.insert(Instruction::Rts, "RTS");
        map.insert(Instruction::Sbc, "SBC");
        map.insert(Instruction::Sec, "SEC");
        map.insert(Instruction::Sed, "SED");
        map.insert(Instruction::Sei, "SEI");
        map.insert(Instruction::Sta, "STA");
        map.insert(Instruction::Stx, "STX");
        map.insert(Instruction::Sty, "STY");
        map.insert(Instruction::Tax, "TAX");
        map.insert(Instruction::Tay, "TAY");
        map.insert(Instruction::Tsx, "TSX");
        map.insert(Instruction::Txa, "TXA");
        map.insert(Instruction::Txs, "TXS");
        map.insert(Instruction::Tya, "TYA");

        map.insert(Instruction::IllAhx, "*AHX");
        map.insert(Instruction::IllAnc, "*ANC");
        map.insert(Instruction::IllAlr, "*ALR");
        map.insert(Instruction::IllArr, "*ARR");
        map.insert(Instruction::IllAxs, "*AXS");
        map.insert(Instruction::IllDcp, "*DCP");
        map.insert(Instruction::IllIsc, "*ISC");
        map.insert(Instruction::IllKil, "*KIL");
        map.insert(Instruction::IllLas, "*LAS");
        map.insert(Instruction::IllLax, "*LAX");
        map.insert(Instruction::IllNop, "*NOP");
        map.insert(Instruction::IllRla, "*RLA");
        map.insert(Instruction::IllRra, "*RRA");
        map.insert(Instruction::IllSax, "*SAX");
        map.insert(Instruction::IllSbc, "*SBC");
        map.insert(Instruction::IllShx, "*SHX");
        map.insert(Instruction::IllShy, "*SHY");
        map.insert(Instruction::IllSlo, "*SLO");
        map.insert(Instruction::IllSre, "*SRE");
        map.insert(Instruction::IllTas, "*TAS");
        map.insert(Instruction::IllXaa, "*XAA");

        map
    }
}
