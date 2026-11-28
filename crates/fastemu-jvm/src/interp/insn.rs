//! Decoded instructions.
//!
//! Each method's bytecode is decoded once, on its first call, into an
//! array of [`Insn`]. Branch targets are instruction indices, constants
//! the pool can give at decode time are inlined, and the typed loads and
//! stores that only move slots are merged.

/// An `int` operation that cannot throw, in a fused instruction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum IntOp {
    Add,
    Sub,
    Mul,
    And,
    Or,
    Xor,
    Shl,
    Shr,
    Ushr,
}

impl IntOp {
    /// The operation of `insn`, if it is one of these.
    pub(crate) fn of(insn: Insn) -> Option<IntOp> {
        Some(match insn {
            Insn::Iadd => IntOp::Add,
            Insn::Isub => IntOp::Sub,
            Insn::Imul => IntOp::Mul,
            Insn::Iand => IntOp::And,
            Insn::Ior => IntOp::Or,
            Insn::Ixor => IntOp::Xor,
            Insn::Ishl => IntOp::Shl,
            Insn::Ishr => IntOp::Shr,
            Insn::Iushr => IntOp::Ushr,
            _ => return None,
        })
    }

    /// `a op b`, as the instruction computes it.
    #[inline]
    pub(crate) fn apply(self, a: i32, b: i32) -> i32 {
        match self {
            IntOp::Add => a.wrapping_add(b),
            IntOp::Sub => a.wrapping_sub(b),
            IntOp::Mul => a.wrapping_mul(b),
            IntOp::And => a & b,
            IntOp::Or => a | b,
            IntOp::Xor => a ^ b,
            IntOp::Shl => a.wrapping_shl(b as u32),
            IntOp::Shr => a.wrapping_shr(b as u32),
            IntOp::Ushr => (a as u32).wrapping_shr(b as u32) as i32,
        }
    }
}

/// A `long` operation that cannot throw, in a fused instruction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LongOp {
    Add,
    Sub,
    Mul,
    And,
    Or,
    Xor,
}

impl LongOp {
    pub(crate) fn of(insn: Insn) -> Option<LongOp> {
        Some(match insn {
            Insn::Ladd => LongOp::Add,
            Insn::Lsub => LongOp::Sub,
            Insn::Lmul => LongOp::Mul,
            Insn::Land => LongOp::And,
            Insn::Lor => LongOp::Or,
            Insn::Lxor => LongOp::Xor,
            _ => return None,
        })
    }

    #[inline]
    pub(crate) fn apply(self, a: i64, b: i64) -> i64 {
        match self {
            LongOp::Add => a.wrapping_add(b),
            LongOp::Sub => a.wrapping_sub(b),
            LongOp::Mul => a.wrapping_mul(b),
            LongOp::And => a & b,
            LongOp::Or => a | b,
            LongOp::Xor => a ^ b,
        }
    }
}

/// A `float` or `double` operation, in a fused instruction. None of them
/// throws.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FloatOp {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
}

impl FloatOp {
    /// The operation of `insn`, a `float` one when `double` is false.
    pub(crate) fn of(insn: Insn, double: bool) -> Option<FloatOp> {
        Some(match (insn, double) {
            (Insn::Fadd, false) | (Insn::Dadd, true) => FloatOp::Add,
            (Insn::Fsub, false) | (Insn::Dsub, true) => FloatOp::Sub,
            (Insn::Fmul, false) | (Insn::Dmul, true) => FloatOp::Mul,
            (Insn::Fdiv, false) | (Insn::Ddiv, true) => FloatOp::Div,
            (Insn::Frem, false) | (Insn::Drem, true) => FloatOp::Rem,
            _ => return None,
        })
    }

    #[inline]
    pub(crate) fn apply<
        T: std::ops::Add<Output = T>
            + std::ops::Sub<Output = T>
            + std::ops::Mul<Output = T>
            + std::ops::Div<Output = T>
            + std::ops::Rem<Output = T>,
    >(
        self,
        a: T,
        b: T,
    ) -> T {
        match self {
            FloatOp::Add => a + b,
            FloatOp::Sub => a - b,
            FloatOp::Mul => a * b,
            FloatOp::Div => a / b,
            FloatOp::Rem => a % b,
        }
    }
}

/// The comparison instruction of a fused compare and branch.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Cmp {
    Long,
    /// `fcmpl` and `fcmpg`: NaN compares as -1 and as 1.
    FloatL,
    FloatG,
    DoubleL,
    DoubleG,
}

impl Cmp {
    pub(crate) fn of(insn: Insn) -> Option<Cmp> {
        Some(match insn {
            Insn::Lcmp => Cmp::Long,
            Insn::Fcmpl => Cmp::FloatL,
            Insn::Fcmpg => Cmp::FloatG,
            Insn::Dcmpl => Cmp::DoubleL,
            Insn::Dcmpg => Cmp::DoubleG,
            _ => return None,
        })
    }
}

/// The comparison of an `if_icmp<cond>`, in a fused instruction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Cond {
    Eq,
    Ne,
    Lt,
    Ge,
    Gt,
    Le,
}

impl Cond {
    /// The comparison and the target of `insn`, if it is an `if_icmp`.
    pub(crate) fn of(insn: Insn) -> Option<(Cond, u32)> {
        Some(match insn {
            Insn::IfIcmpeq(t) => (Cond::Eq, t),
            Insn::IfIcmpne(t) => (Cond::Ne, t),
            Insn::IfIcmplt(t) => (Cond::Lt, t),
            Insn::IfIcmpge(t) => (Cond::Ge, t),
            Insn::IfIcmpgt(t) => (Cond::Gt, t),
            Insn::IfIcmple(t) => (Cond::Le, t),
            _ => return None,
        })
    }

    /// The comparison and the target of `insn`, if it is an `if<cond>`,
    /// which compares with zero.
    pub(crate) fn of_zero(insn: Insn) -> Option<(Cond, u32)> {
        Some(match insn {
            Insn::Ifeq(t) => (Cond::Eq, t),
            Insn::Ifne(t) => (Cond::Ne, t),
            Insn::Iflt(t) => (Cond::Lt, t),
            Insn::Ifge(t) => (Cond::Ge, t),
            Insn::Ifgt(t) => (Cond::Gt, t),
            Insn::Ifle(t) => (Cond::Le, t),
            _ => return None,
        })
    }

    #[inline]
    pub(crate) fn holds(self, a: i32, b: i32) -> bool {
        match self {
            Cond::Eq => a == b,
            Cond::Ne => a != b,
            Cond::Lt => a < b,
            Cond::Ge => a >= b,
            Cond::Gt => a > b,
            Cond::Le => a <= b,
        }
    }
}

/// One decoded instruction. Operands named `u16` are constant pool
/// indices unless the name says otherwise; `u32` operands are instruction
/// indices.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Insn {
    Nop,
    AconstNull,
    Iconst(i32),
    Lconst(i64),
    Fconst(f32),
    Dconst(f64),
    /// `ldc` of a `CONSTANT_String`.
    LdcString(u16),
    /// `ldc` of a `CONSTANT_Class`.
    LdcClass(u16),
    /// `aload` of local `n`: a whole slot, a reference.
    Load(u16),
    /// `iload`, `fload`, `lload` and `dload`: each copies only the value of
    /// its type, so that a value just written is read back at the size it
    /// was written, and the processor forwards it from the store.
    ILoad(u16),
    FLoad(u16),
    LLoad(u16),
    DLoad(u16),
    /// `astore` of local `n`: a whole slot, a reference or a `jsr`
    /// return address.
    Store(u16),
    IStore(u16),
    FStore(u16),
    LStore(u16),
    DStore(u16),
    Iaload,
    Laload,
    Faload,
    Daload,
    Aaload,
    Baload,
    Caload,
    Saload,
    Iastore,
    Lastore,
    Fastore,
    Dastore,
    Aastore,
    Bastore,
    Castore,
    Sastore,
    Pop,
    Pop2,
    Dup,
    DupX1,
    DupX2,
    Dup2,
    Dup2X1,
    Dup2X2,
    Swap,
    Iadd,
    Ladd,
    Fadd,
    Dadd,
    Isub,
    Lsub,
    Fsub,
    Dsub,
    Imul,
    Lmul,
    Fmul,
    Dmul,
    Idiv,
    Ldiv,
    Fdiv,
    Ddiv,
    Irem,
    Lrem,
    Frem,
    Drem,
    Ineg,
    Lneg,
    Fneg,
    Dneg,
    Ishl,
    Lshl,
    Ishr,
    Lshr,
    Iushr,
    Lushr,
    Iand,
    Land,
    Ior,
    Lor,
    Ixor,
    Lxor,
    /// `iinc local, delta`.
    Iinc(u16, i16),
    I2l,
    I2f,
    I2d,
    L2i,
    L2f,
    L2d,
    F2i,
    F2l,
    F2d,
    D2i,
    D2l,
    D2f,
    I2b,
    I2c,
    I2s,
    Lcmp,
    Fcmpl,
    Fcmpg,
    Dcmpl,
    Dcmpg,
    Ifeq(u32),
    Ifne(u32),
    Iflt(u32),
    Ifge(u32),
    Ifgt(u32),
    Ifle(u32),
    IfIcmpeq(u32),
    IfIcmpne(u32),
    IfIcmplt(u32),
    IfIcmpge(u32),
    IfIcmpgt(u32),
    IfIcmple(u32),
    IfAcmpeq(u32),
    IfAcmpne(u32),
    Goto(u32),
    Jsr(u32),
    /// `ret` through local `n`.
    Ret(u16),
    /// An index into the method's switch tables.
    TableSwitch(u32),
    LookupSwitch(u32),
    /// `ireturn`, `freturn`, `areturn`.
    Return1,
    /// `lreturn`, `dreturn`.
    Return2,
    Return,
    GetStatic(u16),
    PutStatic(u16),
    GetField(u16),
    PutField(u16),
    InvokeVirtual(u16),
    InvokeSpecial(u16),
    InvokeStatic(u16),
    InvokeInterface(u16),
    New(u16),
    /// `newarray` with its element type code.
    NewArray(u8),
    ANewArray(u16),
    ArrayLength,
    Athrow,
    CheckCast(u16),
    InstanceOf(u16),
    MonitorEnter,
    MonitorExit,
    // Fused instructions. Each stands in the place of the first instruction
    // of a common sequence and does the whole sequence, and the others stay
    // behind it, so every index keeps its meaning. None of them can throw.
    // A sequence is fused only when nothing branches into its middle (see
    // `decode::fuse`).
    /// `iload a; iconst k; op`.
    ILoadConstOp(u16, i32, IntOp),
    /// `iload a; iload b; op`.
    ILoadLoadOp(u16, u16, IntOp),
    /// `op; istore c`.
    OpIStore(IntOp, u16),
    /// `iload a; iload b; if_icmp<cond> t`.
    IfLocals(u16, u16, Cond, u32),
    /// `iload a; iconst k; if_icmp<cond> t`.
    IfLocalConst(u16, i32, Cond, u32),
    /// `iinc a k; goto t`.
    IincGoto(u16, i16, u32),
    /// `lload a; ldc2_w k; op`.
    LLoadConstOp(u16, i64, LongOp),
    /// `op; lstore c`.
    OpLStore(LongOp, u16),
    /// `fload a; ldc k; op`.
    FLoadConstOp(u16, f32, FloatOp),
    /// `op; fstore c`.
    OpFStore(FloatOp, u16),
    /// `dload a; ldc2_w k; op`.
    DLoadConstOp(u16, f64, FloatOp),
    /// `op; dstore c`.
    OpDStore(FloatOp, u16),
    /// `lcmp`, `fcmpl`, `fcmpg`, `dcmpl` or `dcmpg`, then `if<cond> t`.
    CmpIf(Cmp, Cond, u32),
    /// `multianewarray class, dimensions`.
    MultiANewArray(u16, u8),
    IfNull(u32),
    IfNonNull(u32),
    /// An instruction FastEmu does not run (`invokedynamic`, `ldc` of a
    /// method handle); executing it throws `IncompatibleClassChangeError`.
    Unsupported(u8),
}

/// A `tableswitch` or `lookupswitch`, with instruction-index targets.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Switch {
    Table {
        low: i32,
        targets: Box<[u32]>,
        default: u32,
    },
    /// Keys sorted, as the class file has them.
    Lookup {
        keys: Box<[i32]>,
        targets: Box<[u32]>,
        default: u32,
    },
}

impl Switch {
    #[inline]
    pub(crate) fn target(&self, key: i32) -> u32 {
        match self {
            Switch::Table {
                low,
                targets,
                default,
            } => {
                let i = i64::from(key) - i64::from(*low);
                if i >= 0 && (i as usize) < targets.len() {
                    targets[i as usize]
                } else {
                    *default
                }
            }
            Switch::Lookup {
                keys,
                targets,
                default,
            } => match keys.binary_search(&key) {
                Ok(i) => targets[i],
                // Some obfuscators emit unsorted keys; fall back to a scan.
                Err(_) => keys
                    .iter()
                    .position(|&k| k == key)
                    .map_or(*default, |i| targets[i]),
            },
        }
    }
}

/// An exception handler with instruction-index bounds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Handler {
    pub(crate) start: u32,
    pub(crate) end: u32,
    pub(crate) target: u32,
    /// A class constant pool index, 0 for catch-all.
    pub(crate) catch_type: u16,
}

/// A method's decoded code.
#[derive(Debug)]
pub(crate) struct Code {
    pub(crate) insns: Box<[Insn]>,
    /// The bytecode offset of each instruction, for line numbers.
    pub(crate) pcs: Box<[u32]>,
    pub(crate) handlers: Box<[Handler]>,
    pub(crate) switches: Box<[Switch]>,
    /// The operand stack height before each instruction, kept only for
    /// methods with `ret`, which checks it (see [`super::flow`]).
    pub(crate) ret_heights: Box<[u32]>,
    pub(crate) max_locals: u16,
    pub(crate) max_stack: u16,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn instructions_stay_small() {
        assert_eq!(std::mem::size_of::<Insn>(), 16);
    }

    #[test]
    fn switches_pick_targets() {
        let table = Switch::Table {
            low: -1,
            targets: vec![10, 11, 12].into(),
            default: 99,
        };
        assert_eq!(table.target(-1), 10);
        assert_eq!(table.target(1), 12);
        assert_eq!(table.target(2), 99);
        assert_eq!(table.target(i32::MIN), 99);
        let lookup = Switch::Lookup {
            keys: vec![-5, 3, 100].into(),
            targets: vec![1, 2, 3].into(),
            default: 0,
        };
        assert_eq!(lookup.target(3), 2);
        assert_eq!(lookup.target(4), 0);
    }
}
