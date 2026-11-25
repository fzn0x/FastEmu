//! Operand stack heights: a small part of what a bytecode verifier
//! checks, enough that the interpreter never reads or writes outside a
//! frame.
//!
//! FastEmu has no type verifier, but the interpreter indexes the operand
//! stack without bounds checks of its own, so every method is checked
//! once, when it is decoded: each reachable instruction has one stack
//! height whichever way it is reached, never pops more than the stack
//! holds, never grows it past `max_stack`, and never runs off the end of
//! the code. Constant pool operands of reachable instructions must name
//! entries of the right kind. A return must leave as many slots as the
//! method's descriptor says, because the caller counted on that many. Code
//! javac or ProGuard wrote always passes, as the verifiers of phones and
//! desktop virtual machines demand the same.
//!
//! A subroutine (`jsr` and `ret`) is assumed to leave the stack as it was
//! before its `jsr`, as javac's `finally` blocks do; `ret` checks that at
//! run time against the heights recorded here.

use super::insn::{Handler, Insn, Switch};
use crate::classfile::{Constant, ConstantPool};
use crate::descriptor::{MethodDescriptor, kind_of};

/// The height of an instruction not reached yet.
pub(crate) const UNREACHED: u32 = u32::MAX;

/// Stack slots a field reference's value takes.
fn field_slots(cp: &ConstantPool, index: u16) -> Result<usize, String> {
    match cp.get(index) {
        Some(Constant::FieldRef { .. }) => {
            let m = cp.member_ref(index).map_err(|e| e.0)?;
            Ok(kind_of(m.descriptor).slots())
        }
        _ => Err(format!(
            "constant pool entry {index} is not a field reference"
        )),
    }
}

/// Slots a method reference's arguments take (`this` included when
/// `receiver`) and slots its result takes.
fn method_slots(cp: &ConstantPool, index: u16, receiver: bool) -> Result<(usize, usize), String> {
    match cp.get(index) {
        Some(Constant::MethodRef { .. } | Constant::InterfaceMethodRef { .. }) => {
            let m = cp.member_ref(index).map_err(|e| e.0)?;
            let desc = MethodDescriptor::parse(m.descriptor)
                .ok_or_else(|| format!("constant pool entry {index} has a bad descriptor"))?;
            Ok((
                desc.param_slots() + usize::from(receiver),
                desc.ret_kind().slots(),
            ))
        }
        _ => Err(format!(
            "constant pool entry {index} is not a method reference"
        )),
    }
}

fn class_entry(cp: &ConstantPool, index: u16) -> Result<(), String> {
    cp.class_name(index)
        .map(|_| ())
        .map_err(|_| format!("constant pool entry {index} is not a class"))
}

/// What `insn` pops and then pushes, in slots.
fn effect(insn: Insn, cp: &ConstantPool) -> Result<(usize, usize), String> {
    use Insn as I;
    Ok(match insn {
        I::Nop | I::Iinc(..) | I::Goto(_) | I::Ret(_) | I::Return | I::Unsupported(_) => (0, 0),
        I::AconstNull | I::Iconst(_) | I::Fconst(_) | I::LdcString(_) | I::Load(_) => (0, 1),
        I::LdcClass(i) | I::New(i) => {
            class_entry(cp, i)?;
            (0, 1)
        }
        I::Lconst(_) | I::Dconst(_) | I::Load2(_) => (0, 2),
        I::Jsr(_) => (0, 1),
        I::Store(_)
        | I::Pop
        | I::Ifeq(_)
        | I::Ifne(_)
        | I::Iflt(_)
        | I::Ifge(_)
        | I::Ifgt(_)
        | I::Ifle(_)
        | I::IfNull(_)
        | I::IfNonNull(_)
        | I::TableSwitch(_)
        | I::LookupSwitch(_)
        | I::Return1
        | I::Athrow
        | I::MonitorEnter
        | I::MonitorExit => (1, 0),
        I::Store2(_)
        | I::Pop2
        | I::Return2
        | I::IfIcmpeq(_)
        | I::IfIcmpne(_)
        | I::IfIcmplt(_)
        | I::IfIcmpge(_)
        | I::IfIcmpgt(_)
        | I::IfIcmple(_)
        | I::IfAcmpeq(_)
        | I::IfAcmpne(_) => (2, 0),
        I::Iaload | I::Faload | I::Aaload | I::Baload | I::Caload | I::Saload => (2, 1),
        I::Laload | I::Daload => (2, 2),
        I::Iastore | I::Fastore | I::Aastore | I::Bastore | I::Castore | I::Sastore => (3, 0),
        I::Lastore | I::Dastore => (4, 0),
        I::Dup => (1, 2),
        I::DupX1 => (2, 3),
        I::DupX2 => (3, 4),
        I::Dup2 => (2, 4),
        I::Dup2X1 => (3, 5),
        I::Dup2X2 => (4, 6),
        I::Swap => (2, 2),
        I::Iadd
        | I::Isub
        | I::Imul
        | I::Idiv
        | I::Irem
        | I::Ishl
        | I::Ishr
        | I::Iushr
        | I::Iand
        | I::Ior
        | I::Ixor
        | I::Fadd
        | I::Fsub
        | I::Fmul
        | I::Fdiv
        | I::Frem
        | I::Fcmpl
        | I::Fcmpg => (2, 1),
        I::Ladd
        | I::Lsub
        | I::Lmul
        | I::Ldiv
        | I::Lrem
        | I::Land
        | I::Lor
        | I::Lxor
        | I::Dadd
        | I::Dsub
        | I::Dmul
        | I::Ddiv
        | I::Drem => (4, 2),
        I::Lshl | I::Lshr | I::Lushr => (3, 2),
        I::Lcmp | I::Dcmpl | I::Dcmpg => (4, 1),
        I::Ineg | I::Fneg | I::I2f | I::F2i | I::I2b | I::I2c | I::I2s => (1, 1),
        I::Lneg | I::Dneg | I::L2d | I::D2l => (2, 2),
        I::I2l | I::I2d | I::F2l | I::F2d => (1, 2),
        I::L2i | I::L2f | I::D2i | I::D2f => (2, 1),
        I::GetStatic(i) => (0, field_slots(cp, i)?),
        I::PutStatic(i) => (field_slots(cp, i)?, 0),
        I::GetField(i) => (1, field_slots(cp, i)?),
        I::PutField(i) => (1 + field_slots(cp, i)?, 0),
        I::InvokeStatic(i) => method_slots(cp, i, false)?,
        I::InvokeVirtual(i) | I::InvokeSpecial(i) | I::InvokeInterface(i) => {
            method_slots(cp, i, true)?
        }
        I::NewArray(_) | I::ArrayLength => (1, 1),
        I::ANewArray(i) | I::CheckCast(i) | I::InstanceOf(i) => {
            class_entry(cp, i)?;
            (1, 1)
        }
        I::MultiANewArray(i, dims) => {
            class_entry(cp, i)?;
            (usize::from(dims), 1)
        }
    })
}

/// Where control can go after `insn`, besides exception handlers. `None`
/// stands for the next instruction.
fn successors(insn: Insn, switches: &[Switch], out: &mut Vec<Option<u32>>) {
    use Insn as I;
    out.clear();
    match insn {
        I::Goto(t) => out.push(Some(t)),
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
        | I::IfNull(t)
        | I::IfNonNull(t) => out.extend([Some(t), None]),
        // The subroutine's own entry is handled by the caller, which
        // knows the return address is on the stack there.
        I::Jsr(_) => out.push(None),
        I::TableSwitch(s) | I::LookupSwitch(s) => match &switches[s as usize] {
            Switch::Table {
                targets, default, ..
            }
            | Switch::Lookup {
                targets, default, ..
            } => {
                out.push(Some(*default));
                out.extend(targets.iter().map(|&t| Some(t)));
            }
        },
        // `ret` returns to an instruction after a `jsr`, which is reached
        // from that `jsr`; executing an unsupported instruction throws.
        I::Ret(_) | I::Return | I::Return1 | I::Return2 | I::Athrow | I::Unsupported(_) => {}
        _ => out.push(None),
    }
}

/// The slots a return instruction hands to the caller.
fn return_slots(insn: Insn) -> Option<u8> {
    match insn {
        Insn::Return => Some(0),
        Insn::Return1 => Some(1),
        Insn::Return2 => Some(2),
        _ => None,
    }
}

/// Checks the operand stack of decoded code and returns the stack height
/// before each instruction ([`UNREACHED`] for unreachable ones). A
/// reachable return must give `ret_slots` slots, the width of the method's
/// result: 0 for `void`, 2 for `long` and `double`, 1 for the rest.
///
/// `pcs` maps instruction indices to bytecode offsets, for messages.
pub(crate) fn stack_heights(
    insns: &[Insn],
    pcs: &[u32],
    switches: &[Switch],
    handlers: &[Handler],
    cp: &ConstantPool,
    max_stack: u16,
    ret_slots: u8,
) -> Result<Vec<u32>, String> {
    let max = u32::from(max_stack);
    let mut heights = vec![UNREACHED; insns.len()];
    let mut work = Vec::new();
    let reach = |heights: &mut [u32], work: &mut Vec<u32>, to: u32, height: u32| {
        let slot = &mut heights[to as usize];
        if *slot == UNREACHED {
            *slot = height;
            work.push(to);
            Ok(())
        } else if *slot == height {
            Ok(())
        } else {
            Err(format!(
                "the operand stack holds {} or {height} values at {}",
                *slot, pcs[to as usize]
            ))
        }
    };
    if max == 0 && !handlers.is_empty() {
        return Err("an exception handler needs an operand stack slot".to_owned());
    }
    reach(&mut heights, &mut work, 0, 0)?;
    for h in handlers {
        reach(&mut heights, &mut work, h.target, 1)?;
    }
    let mut next = Vec::new();
    while let Some(at) = work.pop() {
        let insn = insns[at as usize];
        let pc = pcs[at as usize];
        let height = heights[at as usize];
        if return_slots(insn).is_some_and(|slots| slots != ret_slots) {
            return Err(format!(
                "a return of a different kind than the method declares at {pc}"
            ));
        }
        let (pops, pushes) = effect(insn, cp).map_err(|e| format!("at {pc}: {e}"))?;
        let Some(base) = height.checked_sub(pops as u32) else {
            return Err(format!("operand stack underflow at {pc}"));
        };
        let after = base + pushes as u32;
        if after > max {
            return Err(format!(
                "operand stack overflow at {pc}: {after} values, max_stack {max}"
            ));
        }
        if let Insn::Jsr(t) = insn {
            reach(&mut heights, &mut work, t, after)?;
        }
        successors(insn, switches, &mut next);
        for &to in &next {
            let (to, height) = match to {
                Some(t) => (t, after),
                // After a jsr, the subroutine has popped its return address.
                None if matches!(insn, Insn::Jsr(_)) => (at + 1, height),
                None => (at + 1, after),
            };
            if to as usize >= insns.len() {
                return Err(format!(
                    "execution falls off the end of the code after {pc}"
                ));
            }
            reach(&mut heights, &mut work, to, height)?;
        }
    }
    Ok(heights)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The heights of code of a `void` method.
    fn heights(insns: &[Insn], max_stack: u16) -> Result<Vec<u32>, String> {
        returning(insns, max_stack, 0)
    }

    /// The heights of code of a method whose result takes `ret_slots`.
    fn returning(insns: &[Insn], max_stack: u16, ret_slots: u8) -> Result<Vec<u32>, String> {
        let pcs: Vec<u32> = (0..insns.len() as u32).collect();
        stack_heights(
            insns,
            &pcs,
            &[],
            &[],
            &ConstantPool::default(),
            max_stack,
            ret_slots,
        )
    }

    #[test]
    fn heights_follow_branches_and_subroutines() {
        // A branch over a push and its pop, then a finally-style
        // subroutine: the jsr target sees the return address, the
        // instruction after the jsr does not.
        let insns = [
            Insn::Iconst(1),
            Insn::Ifeq(4),
            Insn::Lconst(0),
            Insn::Pop2,
            Insn::Jsr(6),
            Insn::Return,
            Insn::Store(0),
            Insn::Ret(0),
        ];
        let h = heights(&insns, 2).unwrap();
        assert_eq!(h, [0, 1, 0, 2, 0, 0, 1, 0]);
    }

    #[test]
    fn bad_stacks_are_refused() {
        let merge = [
            Insn::Iconst(1),
            Insn::Ifeq(3),
            Insn::Iconst(2),
            Insn::Return,
        ];
        assert!(heights(&merge, 2).unwrap_err().contains("holds"));
        assert!(
            heights(&[Insn::Pop, Insn::Return], 2)
                .unwrap_err()
                .contains("underflow")
        );
        let deep = [Insn::Lconst(0), Insn::Iconst(0), Insn::Return];
        assert!(heights(&deep, 2).unwrap_err().contains("overflow"));
        assert!(heights(&[Insn::Nop], 1).unwrap_err().contains("falls off"));
        assert!(heights(&[Insn::Jsr(1), Insn::Store(0), Insn::Ret(0)], 1).is_err());
    }

    #[test]
    fn a_return_must_match_the_declared_width() {
        let void = [Insn::Return];
        let int = [Insn::Iconst(0), Insn::Return1];
        let long = [Insn::Lconst(0), Insn::Return2];
        let kinds = [(0, &void[..]), (1, &int[..]), (2, &long[..])];
        for (declared, _) in kinds {
            for (width, insns) in kinds {
                let result = returning(insns, 2, declared);
                if width == declared {
                    assert!(result.is_ok(), "width {width}");
                } else {
                    let e = result.unwrap_err();
                    assert!(e.contains("different kind"), "{declared} {width}: {e}");
                }
            }
        }
    }

    #[test]
    fn a_return_nothing_reaches_is_not_checked() {
        // goto 3; iconst_0; ireturn; return: the ireturn is dead code of a
        // void method, which phones accept and so does this.
        let insns = [Insn::Goto(3), Insn::Iconst(0), Insn::Return1, Insn::Return];
        let h = heights(&insns, 1).unwrap();
        assert_eq!(h, [0, UNREACHED, UNREACHED, 0]);
    }
}
