//! Decoded instructions.
//!
//! Each method's bytecode is decoded once, on its first call, into an
//! array of [`Insn`]. Branch targets are instruction indices, constants
//! the pool can give at decode time are inlined, and the typed loads and
//! stores that only move slots are merged.

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
