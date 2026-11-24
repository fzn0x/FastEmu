//! Decoding bytecode into [`Insn`] arrays.

use super::insn::{Code, Handler, Insn, Switch};
use crate::classfile::{CodeAttr, Constant, ConstantPool};

struct Bytes<'a> {
    code: &'a [u8],
    pos: usize,
}

impl Bytes<'_> {
    fn u8(&mut self) -> Result<u8, String> {
        let b = *self
            .code
            .get(self.pos)
            .ok_or("an instruction runs past the end of the code")?;
        self.pos += 1;
        Ok(b)
    }

    fn i8(&mut self) -> Result<i8, String> {
        Ok(self.u8()? as i8)
    }

    fn u16(&mut self) -> Result<u16, String> {
        Ok(u16::from(self.u8()?) << 8 | u16::from(self.u8()?))
    }

    fn i16(&mut self) -> Result<i16, String> {
        Ok(self.u16()? as i16)
    }

    fn i32(&mut self) -> Result<i32, String> {
        Ok((u32::from(self.u16()?) << 16 | u32::from(self.u16()?)) as i32)
    }
}

/// Branch targets are bytecode offsets until [`decode`] maps them.
fn target(pc: usize, offset: i32) -> Result<u32, String> {
    let t = pc as i64 + i64::from(offset);
    u32::try_from(t).map_err(|_| format!("a branch at {pc} leaves the method"))
}

fn ldc(cp: &ConstantPool, index: u16, opcode: u8) -> Result<Insn, String> {
    Ok(match cp.get(index) {
        Some(Constant::Integer(v)) => Insn::Iconst(*v),
        Some(Constant::Float(v)) => Insn::Fconst(*v),
        Some(Constant::String(_)) => Insn::LdcString(index),
        Some(Constant::Class(_)) => Insn::LdcClass(index),
        Some(
            Constant::MethodType(_) | Constant::MethodHandle { .. } | Constant::Dynamic { .. },
        ) => Insn::Unsupported(opcode),
        _ => {
            return Err(format!(
                "ldc of constant pool entry {index}, which is not loadable"
            ));
        }
    })
}

fn ldc2(cp: &ConstantPool, index: u16) -> Result<Insn, String> {
    Ok(match cp.get(index) {
        Some(Constant::Long(v)) => Insn::Lconst(*v),
        Some(Constant::Double(v)) => Insn::Dconst(*v),
        _ => {
            return Err(format!(
                "ldc2_w of constant pool entry {index}, which is not a long or double"
            ));
        }
    })
}

/// Decodes one instruction at `b.pos`, with branch targets as offsets.
fn decode_one(
    b: &mut Bytes<'_>,
    cp: &ConstantPool,
    switches: &mut Vec<Switch>,
) -> Result<Insn, String> {
    let pc = b.pos;
    let op = b.u8()?;
    use Insn as I;
    Ok(match op {
        0x00 => I::Nop,
        0x01 => I::AconstNull,
        0x02..=0x08 => I::Iconst(i32::from(op) - 3),
        0x09 | 0x0a => I::Lconst(i64::from(op - 0x09)),
        0x0b..=0x0d => I::Fconst(f32::from(op - 0x0b)),
        0x0e | 0x0f => I::Dconst(f64::from(op - 0x0e)),
        0x10 => I::Iconst(i32::from(b.i8()?)),
        0x11 => I::Iconst(i32::from(b.i16()?)),
        0x12 => {
            let index = u16::from(b.u8()?);
            ldc(cp, index, op)?
        }
        0x13 => {
            let index = b.u16()?;
            ldc(cp, index, op)?
        }
        0x14 => {
            let index = b.u16()?;
            ldc2(cp, index)?
        }
        0x15 | 0x17 | 0x19 => I::Load(u16::from(b.u8()?)),
        0x16 | 0x18 => I::Load2(u16::from(b.u8()?)),
        0x1a..=0x1d => I::Load(u16::from(op - 0x1a)),
        0x1e..=0x21 => I::Load2(u16::from(op - 0x1e)),
        0x22..=0x25 => I::Load(u16::from(op - 0x22)),
        0x26..=0x29 => I::Load2(u16::from(op - 0x26)),
        0x2a..=0x2d => I::Load(u16::from(op - 0x2a)),
        0x2e => I::Iaload,
        0x2f => I::Laload,
        0x30 => I::Faload,
        0x31 => I::Daload,
        0x32 => I::Aaload,
        0x33 => I::Baload,
        0x34 => I::Caload,
        0x35 => I::Saload,
        0x36 | 0x38 | 0x3a => I::Store(u16::from(b.u8()?)),
        0x37 | 0x39 => I::Store2(u16::from(b.u8()?)),
        0x3b..=0x3e => I::Store(u16::from(op - 0x3b)),
        0x3f..=0x42 => I::Store2(u16::from(op - 0x3f)),
        0x43..=0x46 => I::Store(u16::from(op - 0x43)),
        0x47..=0x4a => I::Store2(u16::from(op - 0x47)),
        0x4b..=0x4e => I::Store(u16::from(op - 0x4b)),
        0x4f => I::Iastore,
        0x50 => I::Lastore,
        0x51 => I::Fastore,
        0x52 => I::Dastore,
        0x53 => I::Aastore,
        0x54 => I::Bastore,
        0x55 => I::Castore,
        0x56 => I::Sastore,
        0x57 => I::Pop,
        0x58 => I::Pop2,
        0x59 => I::Dup,
        0x5a => I::DupX1,
        0x5b => I::DupX2,
        0x5c => I::Dup2,
        0x5d => I::Dup2X1,
        0x5e => I::Dup2X2,
        0x5f => I::Swap,
        0x60 => I::Iadd,
        0x61 => I::Ladd,
        0x62 => I::Fadd,
        0x63 => I::Dadd,
        0x64 => I::Isub,
        0x65 => I::Lsub,
        0x66 => I::Fsub,
        0x67 => I::Dsub,
        0x68 => I::Imul,
        0x69 => I::Lmul,
        0x6a => I::Fmul,
        0x6b => I::Dmul,
        0x6c => I::Idiv,
        0x6d => I::Ldiv,
        0x6e => I::Fdiv,
        0x6f => I::Ddiv,
        0x70 => I::Irem,
        0x71 => I::Lrem,
        0x72 => I::Frem,
        0x73 => I::Drem,
        0x74 => I::Ineg,
        0x75 => I::Lneg,
        0x76 => I::Fneg,
        0x77 => I::Dneg,
        0x78 => I::Ishl,
        0x79 => I::Lshl,
        0x7a => I::Ishr,
        0x7b => I::Lshr,
        0x7c => I::Iushr,
        0x7d => I::Lushr,
        0x7e => I::Iand,
        0x7f => I::Land,
        0x80 => I::Ior,
        0x81 => I::Lor,
        0x82 => I::Ixor,
        0x83 => I::Lxor,
        0x84 => {
            let index = u16::from(b.u8()?);
            I::Iinc(index, i16::from(b.i8()?))
        }
        0x85 => I::I2l,
        0x86 => I::I2f,
        0x87 => I::I2d,
        0x88 => I::L2i,
        0x89 => I::L2f,
        0x8a => I::L2d,
        0x8b => I::F2i,
        0x8c => I::F2l,
        0x8d => I::F2d,
        0x8e => I::D2i,
        0x8f => I::D2l,
        0x90 => I::D2f,
        0x91 => I::I2b,
        0x92 => I::I2c,
        0x93 => I::I2s,
        0x94 => I::Lcmp,
        0x95 => I::Fcmpl,
        0x96 => I::Fcmpg,
        0x97 => I::Dcmpl,
        0x98 => I::Dcmpg,
        0x99..=0xa8 | 0xc6 | 0xc7 => {
            let t = target(pc, i32::from(b.i16()?))?;
            match op {
                0x99 => I::Ifeq(t),
                0x9a => I::Ifne(t),
                0x9b => I::Iflt(t),
                0x9c => I::Ifge(t),
                0x9d => I::Ifgt(t),
                0x9e => I::Ifle(t),
                0x9f => I::IfIcmpeq(t),
                0xa0 => I::IfIcmpne(t),
                0xa1 => I::IfIcmplt(t),
                0xa2 => I::IfIcmpge(t),
                0xa3 => I::IfIcmpgt(t),
                0xa4 => I::IfIcmple(t),
                0xa5 => I::IfAcmpeq(t),
                0xa6 => I::IfAcmpne(t),
                0xa7 => I::Goto(t),
                0xa8 => I::Jsr(t),
                0xc6 => I::IfNull(t),
                _ => I::IfNonNull(t),
            }
        }
        0xa9 => I::Ret(u16::from(b.u8()?)),
        0xaa | 0xab => {
            while !b.pos.is_multiple_of(4) {
                b.u8()?;
            }
            let default = target(pc, b.i32()?)?;
            let switch = if op == 0xaa {
                let low = b.i32()?;
                let high = b.i32()?;
                if high < low {
                    return Err(format!(
                        "tableswitch at {pc} has high {high} below low {low}"
                    ));
                }
                let n = (i64::from(high) - i64::from(low) + 1) as usize;
                if n > b.code.len() {
                    return Err(format!("tableswitch at {pc} is larger than the method"));
                }
                let mut targets = Vec::with_capacity(n);
                for _ in 0..n {
                    targets.push(target(pc, b.i32()?)?);
                }
                Switch::Table {
                    low,
                    targets: targets.into(),
                    default,
                }
            } else {
                let n = b.i32()?;
                if n < 0 || n as usize > b.code.len() {
                    return Err(format!("lookupswitch at {pc} has {n} pairs"));
                }
                let mut keys = Vec::with_capacity(n as usize);
                let mut targets = Vec::with_capacity(n as usize);
                for _ in 0..n {
                    keys.push(b.i32()?);
                    targets.push(target(pc, b.i32()?)?);
                }
                Switch::Lookup {
                    keys: keys.into(),
                    targets: targets.into(),
                    default,
                }
            };
            switches.push(switch);
            let index = (switches.len() - 1) as u32;
            if op == 0xaa {
                I::TableSwitch(index)
            } else {
                I::LookupSwitch(index)
            }
        }
        0xac | 0xae | 0xb0 => I::Return1,
        0xad | 0xaf => I::Return2,
        0xb1 => I::Return,
        0xb2 => I::GetStatic(b.u16()?),
        0xb3 => I::PutStatic(b.u16()?),
        0xb4 => I::GetField(b.u16()?),
        0xb5 => I::PutField(b.u16()?),
        0xb6 => I::InvokeVirtual(b.u16()?),
        0xb7 => I::InvokeSpecial(b.u16()?),
        0xb8 => I::InvokeStatic(b.u16()?),
        0xb9 => {
            let index = b.u16()?;
            b.u8()?;
            b.u8()?;
            I::InvokeInterface(index)
        }
        0xba => {
            b.u16()?;
            b.u16()?;
            I::Unsupported(op)
        }
        0xbb => I::New(b.u16()?),
        0xbc => {
            let atype = b.u8()?;
            if crate::class::ArrayType::from_atype(atype).is_none() {
                return Err(format!("newarray at {pc} has element type {atype}"));
            }
            I::NewArray(atype)
        }
        0xbd => I::ANewArray(b.u16()?),
        0xbe => I::ArrayLength,
        0xbf => I::Athrow,
        0xc0 => I::CheckCast(b.u16()?),
        0xc1 => I::InstanceOf(b.u16()?),
        0xc2 => I::MonitorEnter,
        0xc3 => I::MonitorExit,
        0xc4 => {
            let wide_op = b.u8()?;
            let index = b.u16()?;
            match wide_op {
                0x15 | 0x17 | 0x19 => I::Load(index),
                0x16 | 0x18 => I::Load2(index),
                0x36 | 0x38 | 0x3a => I::Store(index),
                0x37 | 0x39 => I::Store2(index),
                0xa9 => I::Ret(index),
                0x84 => I::Iinc(index, b.i16()?),
                _ => return Err(format!("wide at {pc} modifies opcode {wide_op:#x}")),
            }
        }
        0xc5 => {
            let index = b.u16()?;
            let dims = b.u8()?;
            if dims == 0 {
                return Err(format!("multianewarray at {pc} has no dimensions"));
            }
            I::MultiANewArray(index, dims)
        }
        0xc8 => I::Goto(target(pc, b.i32()?)?),
        0xc9 => I::Jsr(target(pc, b.i32()?)?),
        _ => return Err(format!("unknown opcode {op:#x} at {pc}")),
    })
}

/// Replaces the bytecode offset in a branch with an instruction index.
fn map_branch(insn: &mut Insn, map: &impl Fn(u32) -> Result<u32, String>) -> Result<(), String> {
    use Insn as I;
    match insn {
        I::Ifeq(t)
        | I::Ifne(t)
        | I::Iflt(t)
        | I::Ifge(t)
        | I::Ifgt(t)
        | I::Ifle(t)
        | I::IfIcmpeq(t)
        | I::IfIcmpne(t)
        | I::IfIcmplt(t)
        | I::IfIcmpge(t)
        | I::IfIcmpgt(t)
        | I::IfIcmple(t)
        | I::IfAcmpeq(t)
        | I::IfAcmpne(t)
        | I::Goto(t)
        | I::Jsr(t)
        | I::IfNull(t)
        | I::IfNonNull(t) => *t = map(*t)?,
        _ => {}
    }
    Ok(())
}

/// Checks that local `index` (and the next, for two-slot values) exists.
fn check_local(insn: &Insn, max_locals: u16) -> Result<(), String> {
    let (index, width) = match *insn {
        Insn::Load(i) | Insn::Store(i) | Insn::Ret(i) | Insn::Iinc(i, _) => (i, 1),
        Insn::Load2(i) | Insn::Store2(i) => (i, 2),
        _ => return Ok(()),
    };
    if u32::from(index) + width > u32::from(max_locals) {
        return Err(format!(
            "local {index} is outside the frame's {max_locals} locals"
        ));
    }
    Ok(())
}

/// Decodes a method's code. The error becomes a `VerifyError`.
pub(crate) fn decode(attr: &CodeAttr, cp: &ConstantPool, arg_slots: u16) -> Result<Code, String> {
    if attr.max_locals < arg_slots {
        return Err(format!(
            "max_locals {} is less than the {arg_slots} argument slots",
            attr.max_locals
        ));
    }
    let code = &attr.code;
    let mut index_of = vec![u32::MAX; code.len() + 1];
    let mut insns = Vec::with_capacity(code.len() / 2);
    let mut pcs = Vec::with_capacity(code.len() / 2);
    let mut switches = Vec::new();
    let mut b = Bytes { code, pos: 0 };
    while b.pos < code.len() {
        index_of[b.pos] = insns.len() as u32;
        pcs.push(b.pos as u32);
        let insn = decode_one(&mut b, cp, &mut switches)?;
        check_local(&insn, attr.max_locals)?;
        insns.push(insn);
    }
    index_of[code.len()] = insns.len() as u32;
    let map = |offset: u32| -> Result<u32, String> {
        match index_of.get(offset as usize) {
            Some(&i) if i != u32::MAX && (offset as usize) < code.len() => Ok(i),
            _ => Err(format!("a branch to {offset}, which is not an instruction")),
        }
    };
    for insn in &mut insns {
        map_branch(insn, &map)?;
    }
    for switch in &mut switches {
        match switch {
            Switch::Table {
                targets, default, ..
            }
            | Switch::Lookup {
                targets, default, ..
            } => {
                *default = map(*default)?;
                for t in targets.iter_mut() {
                    *t = map(*t)?;
                }
            }
        }
    }
    let bound = |offset: u16| -> Result<u32, String> {
        match index_of.get(offset as usize) {
            Some(&i) if i != u32::MAX => Ok(i),
            _ => Err(format!(
                "an exception handler bound at {offset}, which is not an instruction"
            )),
        }
    };
    let mut handlers = Vec::with_capacity(attr.handlers.len());
    for h in &attr.handlers {
        if h.catch_type != 0 && cp.class_name(h.catch_type).is_err() {
            return Err(format!(
                "an exception handler catches constant pool entry {}, which is not a class",
                h.catch_type
            ));
        }
        handlers.push(Handler {
            start: bound(h.start_pc)?,
            end: bound(h.end_pc)?,
            target: map(u32::from(h.handler_pc))?,
            catch_type: h.catch_type,
        });
    }
    let heights =
        super::flow::stack_heights(&insns, &pcs, &switches, &handlers, cp, attr.max_stack)?;
    // Only `ret` reads the heights, to check where a subroutine returns.
    let ret_heights = if insns.iter().any(|i| matches!(i, Insn::Ret(_))) {
        heights.into()
    } else {
        Box::default()
    };
    Ok(Code {
        insns: insns.into(),
        pcs: pcs.into(),
        handlers: handlers.into(),
        switches: switches.into(),
        ret_heights,
        max_locals: attr.max_locals,
        max_stack: attr.max_stack,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn attr(code: Vec<u8>, max_locals: u16) -> CodeAttr {
        CodeAttr {
            max_stack: 4,
            max_locals,
            code,
            handlers: Vec::new(),
            lines: Vec::new(),
        }
    }

    #[test]
    fn decodes_branches_to_instruction_indices() {
        // 0: iconst_0; 1: istore_0; 2: iinc 0 1; 5: iload_0; 6: bipush 10;
        // 8: if_icmplt -6 (to 2); 11: return
        let code = vec![
            0x03, 0x3b, 0x84, 0, 1, 0x1a, 0x10, 10, 0xa1, 0xff, 0xfa, 0xb1,
        ];
        let decoded = decode(&attr(code, 1), &ConstantPool::default(), 0).unwrap();
        assert_eq!(
            &*decoded.insns,
            &[
                Insn::Iconst(0),
                Insn::Store(0),
                Insn::Iinc(0, 1),
                Insn::Load(0),
                Insn::Iconst(10),
                Insn::IfIcmplt(2),
                Insn::Return,
            ]
        );
        assert_eq!(&*decoded.pcs, &[0, 1, 2, 5, 6, 8, 11]);
    }

    #[test]
    fn decodes_switches_with_padding() {
        // 0: iload_0; 1: tableswitch (pad 2) default 28, low 0, high 1,
        // targets 24, 26 (relative to 1); 25: iconst_0 ... ireturn
        let mut code = vec![0x1a, 0xaa, 0, 0];
        for v in [27i32, 0, 1, 23, 25] {
            code.extend(v.to_be_bytes());
        }
        code.extend([0x03, 0xac, 0x04, 0xac, 0x05, 0xac]);
        let decoded = decode(&attr(code, 1), &ConstantPool::default(), 1).unwrap();
        assert_eq!(decoded.insns[1], Insn::TableSwitch(0));
        let Switch::Table {
            targets, default, ..
        } = &decoded.switches[0]
        else {
            panic!("a table switch")
        };
        assert_eq!((&**targets, *default), (&[2, 4][..], 6));
    }

    #[test]
    fn wide_and_jsr_decode() {
        // wide iload 300; wide iinc 300 -1000; jsr +4; return; astore_1; ret 1
        let mut code = vec![0xc4, 0x15, 0x01, 0x2c, 0xc4, 0x84, 0x01, 0x2c];
        code.extend((-1000i16).to_be_bytes());
        code.extend([0xa8, 0, 4, 0xb1, 0x4c, 0xa9, 1]);
        let decoded = decode(&attr(code, 301), &ConstantPool::default(), 0).unwrap();
        assert_eq!(
            &*decoded.insns,
            &[
                Insn::Load(300),
                Insn::Iinc(300, -1000),
                Insn::Jsr(4),
                Insn::Return,
                Insn::Store(1),
                Insn::Ret(1),
            ]
        );
    }

    #[test]
    fn bad_code_is_refused() {
        let pool = ConstantPool::default();
        assert!(
            decode(&attr(vec![0xa7, 0, 1, 0xb1], 0), &pool, 0).is_err(),
            "mid-instruction"
        );
        assert!(
            decode(&attr(vec![0x1b, 0xb1], 1), &pool, 0).is_err(),
            "local 1 of 1"
        );
        assert!(
            decode(&attr(vec![0xfe], 0), &pool, 0).is_err(),
            "unknown opcode"
        );
        assert!(decode(&attr(vec![0x10], 0), &pool, 0).is_err(), "truncated");
        assert!(
            decode(&attr(vec![0xb1], 0), &pool, 1).is_err(),
            "too few locals"
        );
    }
}
