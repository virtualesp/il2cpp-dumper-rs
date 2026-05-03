use iced_x86::{Decoder, DecoderOptions, Formatter, Instruction, IntelFormatter, Mnemonic, OpKind, Register};
use super::{DisassembledInstruction, RegRegAccess, ConstantOp, LoadInfo};

pub fn disassemble_x86(
    bytes: &[u8],
    base_address: u64,
    max_instructions: usize,
    bitness: u32,
) -> Vec<DisassembledInstruction> {
    let mut result = Vec::with_capacity(max_instructions.min(512));
    let mut decoder = Decoder::with_ip(bitness, bytes, base_address, DecoderOptions::NONE);
    let mut formatter = IntelFormatter::new();

    formatter.options_mut().set_uppercase_mnemonics(true);
    formatter.options_mut().set_space_after_operand_separator(true);
    formatter.options_mut().set_hex_prefix("0x");
    formatter.options_mut().set_hex_suffix("");
    formatter.options_mut().set_branch_leading_zeros(false);

    let mut instruction = Instruction::default();
    let mut output = String::with_capacity(64);
    let mut count = 0;
    let mut seen_ret = false;

    while decoder.can_decode() && count < max_instructions {
        decoder.decode_out(&mut instruction);

        output.clear();
        formatter.format(&instruction, &mut output);

        let (mnemonic_str, operands) = split_mnemonic_operands(&output);

        let is_return = matches!(
            instruction.mnemonic(),
            Mnemonic::Ret | Mnemonic::Retf
        );

        let is_call = matches!(instruction.mnemonic(), Mnemonic::Call);

        let is_jcc = is_jcc_mnemonic(&instruction);

        let is_unconditional_branch = matches!(
            instruction.mnemonic(),
            Mnemonic::Jmp | Mnemonic::Ret | Mnemonic::Retf
        );

        let is_branch = is_unconditional_branch || is_call || is_jcc;

        let call_target = if is_call {
            extract_branch_target(&instruction)
        } else {
            None
        };

        let branch_target = if (is_jcc || is_unconditional_branch) && !is_return {
            extract_branch_target(&instruction)
        } else {
            None
        };

        let condition_code = if is_jcc {
            extract_x86_condition(&instruction)
        } else {
            None
        };

        let memory_offset = extract_memory_offset(&instruction);
        let reg_reg_access = extract_x86_reg_reg_access(&instruction);
        let constant_op = extract_x86_constant_op(&instruction);
        let load_info = extract_x86_load_info(&instruction);
        let indirect_call_reg = if is_call && call_target.is_none() {
            extract_x86_indirect_call_reg(&instruction)
        } else { None };

        let raw_start = (instruction.ip() - base_address) as usize;
        let raw_end = (raw_start + instruction.len()).min(bytes.len());
        let raw_bytes = if raw_start < bytes.len() {
            bytes[raw_start..raw_end].to_vec()
        } else {
            vec![0; instruction.len()]
        };

        result.push(DisassembledInstruction {
            address: instruction.ip(),
            size: instruction.len(),
            raw_bytes,
            mnemonic: mnemonic_str,
            operands,
            is_call,
            is_return,
            is_branch,
            is_unconditional_branch,
            call_target,
            branch_target,
            condition_code,
            memory_offset,
            reg_reg_access,
            constant_op,
            load_info,
            indirect_call_reg,
        });

        count += 1;

        if is_return {
            if seen_ret {
                break;
            }
            seen_ret = true;
        }
    }

    result
}

fn split_mnemonic_operands(text: &str) -> (String, String) {
    let trimmed = text.trim();
    if let Some(pos) = trimmed.find(' ') {
        let mnemonic = trimmed[..pos].to_string();
        let operands = trimmed[pos..].trim().to_string();
        (mnemonic, operands)
    } else {
        (trimmed.to_string(), String::new())
    }
}

fn is_jcc_mnemonic(instruction: &Instruction) -> bool {
    matches!(
        instruction.mnemonic(),
        Mnemonic::Ja | Mnemonic::Jae | Mnemonic::Jb | Mnemonic::Jbe
        | Mnemonic::Je | Mnemonic::Jne | Mnemonic::Jg | Mnemonic::Jge
        | Mnemonic::Jl | Mnemonic::Jle | Mnemonic::Jo | Mnemonic::Jno
        | Mnemonic::Jp | Mnemonic::Jnp | Mnemonic::Js | Mnemonic::Jns
        | Mnemonic::Jecxz | Mnemonic::Jrcxz
        | Mnemonic::Loop | Mnemonic::Loope | Mnemonic::Loopne
    )
}

fn extract_branch_target(instruction: &Instruction) -> Option<u64> {
    if instruction.op_count() >= 1 {
        match instruction.op0_kind() {
            OpKind::NearBranch16 => Some(instruction.near_branch16() as u64),
            OpKind::NearBranch32 => Some(instruction.near_branch32() as u64),
            OpKind::NearBranch64 => Some(instruction.near_branch64()),
            OpKind::FarBranch16 => Some(instruction.far_branch16() as u64),
            OpKind::FarBranch32 => Some(instruction.far_branch32() as u64),
            _ => None,
        }
    } else {
        None
    }
}

fn extract_x86_condition(instruction: &Instruction) -> Option<String> {
    let cond = match instruction.mnemonic() {
        Mnemonic::Je => "==",
        Mnemonic::Jne => "!=",
        Mnemonic::Jg => ">",
        Mnemonic::Jge => ">=",
        Mnemonic::Jl => "<",
        Mnemonic::Jle => "<=",
        Mnemonic::Ja => "> (unsigned)",
        Mnemonic::Jae => ">= (unsigned)",
        Mnemonic::Jb => "< (unsigned)",
        Mnemonic::Jbe => "<= (unsigned)",
        Mnemonic::Jo => "overflow",
        Mnemonic::Jno => "!overflow",
        Mnemonic::Js => "< 0 (sign)",
        Mnemonic::Jns => ">= 0 (sign)",
        Mnemonic::Jp => "parity",
        Mnemonic::Jnp => "!parity",
        Mnemonic::Jecxz | Mnemonic::Jrcxz => "counter == 0",
        Mnemonic::Loop => "counter != 0",
        Mnemonic::Loope => "counter != 0 && ==",
        Mnemonic::Loopne => "counter != 0 && !=",
        _ => return None,
    };
    Some(cond.to_string())
}

fn extract_memory_offset(instruction: &Instruction) -> Option<i64> {
    for i in 0..instruction.op_count() {
        let kind = match i {
            0 => instruction.op0_kind(),
            1 => instruction.op1_kind(),
            2 => instruction.op2_kind(),
            3 => instruction.op3_kind(),
            _ => continue,
        };

        if kind == OpKind::Memory {
            let disp = instruction.memory_displacement64() as i64;
            if disp != 0 {
                return Some(disp);
            }
        }
    }
    None
}

fn extract_x86_load_info(instruction: &Instruction) -> Option<LoadInfo> {
    if instruction.mnemonic() != Mnemonic::Mov {
        return None;
    }
    if instruction.op_count() < 2 { return None; }
    if instruction.op0_kind() != OpKind::Register { return None; }
    if instruction.op1_kind() != OpKind::Memory { return None; }
    if instruction.memory_index() != Register::None { return None; }

    let base = instruction.memory_base();
    if base == Register::None { return None; }

    let dest = normalize_x86_reg(instruction.op0_register());
    let base_norm = normalize_x86_reg(base);
    let offset = instruction.memory_displacement64() as i64;
    Some(LoadInfo { dest_reg: dest, base_reg: base_norm, offset })
}

fn extract_x86_indirect_call_reg(instruction: &Instruction) -> Option<u16> {
    if instruction.mnemonic() != Mnemonic::Call { return None; }
    if instruction.op0_kind() == OpKind::Register {
        return Some(normalize_x86_reg(instruction.op0_register()));
    }
    None
}

fn extract_x86_reg_reg_access(instruction: &Instruction) -> Option<RegRegAccess> {
    for i in 0..instruction.op_count() {
        let kind = match i {
            0 => instruction.op0_kind(),
            1 => instruction.op1_kind(),
            2 => instruction.op2_kind(),
            3 => instruction.op3_kind(),
            _ => continue,
        };

        if kind == OpKind::Memory {
            let index = instruction.memory_index();
            if index != Register::None {
                let base = instruction.memory_base();
                return Some(RegRegAccess {
                    base_reg: normalize_x86_reg(base),
                    index_reg: normalize_x86_reg(index),
                    shift: match instruction.memory_index_scale() {
                        1 => 0,
                        2 => 1,
                        4 => 2,
                        8 => 3,
                        _ => 0,
                    },
                });
            }
        }
    }
    None
}

fn normalize_x86_reg(reg: Register) -> u16 {
    reg.full_register() as u16
}

fn is_x86_gp_register(reg: Register) -> bool {
    let full = reg.full_register();
    matches!(full,
        Register::RAX | Register::RCX | Register::RDX | Register::RBX
        | Register::RSP | Register::RBP | Register::RSI | Register::RDI
        | Register::R8 | Register::R9 | Register::R10 | Register::R11
        | Register::R12 | Register::R13 | Register::R14 | Register::R15
        | Register::EAX | Register::ECX | Register::EDX | Register::EBX
        | Register::ESP | Register::EBP | Register::ESI | Register::EDI
    )
}

fn is_x86_immediate(kind: OpKind) -> bool {
    matches!(kind,
        OpKind::Immediate8 | OpKind::Immediate16 | OpKind::Immediate32
        | OpKind::Immediate64 | OpKind::Immediate8to16 | OpKind::Immediate8to32
        | OpKind::Immediate8to64 | OpKind::Immediate32to64
    )
}

fn get_x86_immediate(instruction: &Instruction, op_index: u32) -> u64 {
    match op_index {
        0 => instruction.immediate(0) as u64,
        1 => instruction.immediate(1) as u64,
        _ => 0,
    }
}

fn extract_x86_constant_op(instruction: &Instruction) -> Option<ConstantOp> {
    match instruction.mnemonic() {
        Mnemonic::Mov => {
            if instruction.op_count() < 2 { return None; }
            if instruction.op0_kind() != OpKind::Register { return None; }
            let dest = normalize_x86_reg(instruction.op0_register());
            if is_x86_immediate(instruction.op1_kind()) {
                let value = get_x86_immediate(instruction, 1);
                return Some(ConstantOp::MovImm { dest_reg: dest, value });
            }
            if instruction.op1_kind() == OpKind::Register {
                let src = normalize_x86_reg(instruction.op1_register());
                return Some(ConstantOp::MovReg { dest_reg: dest, src_reg: src });
            }
            Some(ConstantOp::Kill { dest_reg: dest })
        }
        Mnemonic::Xor => {
            if instruction.op_count() < 2 { return None; }
            if instruction.op0_kind() != OpKind::Register { return None; }
            let dest = normalize_x86_reg(instruction.op0_register());
            if instruction.op1_kind() == OpKind::Register
                && instruction.op0_register().full_register() == instruction.op1_register().full_register()
            {
                return Some(ConstantOp::MovImm { dest_reg: dest, value: 0 });
            }
            Some(ConstantOp::Kill { dest_reg: dest })
        }
        Mnemonic::Add | Mnemonic::Sub => {
            if instruction.op_count() < 2 { return None; }
            if instruction.op0_kind() != OpKind::Register { return None; }
            let dest = normalize_x86_reg(instruction.op0_register());
            if is_x86_immediate(instruction.op1_kind()) {
                let imm = get_x86_immediate(instruction, 1) as i64;
                let imm = if instruction.mnemonic() == Mnemonic::Sub { -imm } else { imm };
                return Some(ConstantOp::AddSubImm { dest_reg: dest, src_reg: dest, imm });
            }
            Some(ConstantOp::Kill { dest_reg: dest })
        }
        Mnemonic::Lea => {
            if instruction.op0_kind() == OpKind::Register {
                let dest = normalize_x86_reg(instruction.op0_register());
                return Some(ConstantOp::Kill { dest_reg: dest });
            }
            None
        }
        // Non-writing instructions
        Mnemonic::Cmp | Mnemonic::Test | Mnemonic::Push | Mnemonic::Nop
        | Mnemonic::Ret | Mnemonic::Retf
        | Mnemonic::Call | Mnemonic::Jmp
        | Mnemonic::Ja | Mnemonic::Jae | Mnemonic::Jb | Mnemonic::Jbe
        | Mnemonic::Je | Mnemonic::Jne | Mnemonic::Jg | Mnemonic::Jge
        | Mnemonic::Jl | Mnemonic::Jle | Mnemonic::Jo | Mnemonic::Jno
        | Mnemonic::Jp | Mnemonic::Jnp | Mnemonic::Js | Mnemonic::Jns
        | Mnemonic::Int | Mnemonic::Int1 | Mnemonic::Int3
        | Mnemonic::Hlt | Mnemonic::Ud0 | Mnemonic::Ud1 | Mnemonic::Ud2 => None,
        _ => {
            if instruction.op_count() >= 1 && instruction.op0_kind() == OpKind::Register {
                let reg = instruction.op0_register();
                if is_x86_gp_register(reg) {
                    return Some(ConstantOp::Kill { dest_reg: normalize_x86_reg(reg) });
                }
            }
            None
        }
    }
}
