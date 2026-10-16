//! Compiled Squirrel 3 scripts of Muramasa's English release
//! (`script.npk`), read for their text and the telops' places.
//!
//! A file is `"SCRP"`, u32 header size (0x10), two u32; then a Squirrel 3
//! closure stream: u16 0xFAFA, `"RIQS"`, u32 sizeof(SQChar) (1; the stock
//! sizes of SQInteger and SQFloat are left out), the main function,
//! `"LIAT"`. SQInteger is 32-bit, strings UTF-8. A function is its parts,
//! each after a `"TRAP"` tag: source and name; eight counts; literals;
//! parameters; outer values; locals; line infos; default parameters;
//! instructions (8 bytes: i32 arg1, u8 op, arg0, arg2, arg3); functions;
//! then u32 stack size, u8 generator, u8 varparams.
//!
//! The opcodes are renumbered against stock 3.1; the ones read here
//! (`LOAD` 1, `LOADINT` 2, `DLOAD` 4, `CALL` 6, `PREPCALLK` 8, `GETK` 9,
//! `NEWSLOT` 11) are not. `research/nitroplus/nut.py` disassembles a whole file.

use crate::read::{Reader, Truncated};

const LOAD: u8 = 1;
const LOADINT: u8 = 2;
const DLOAD: u8 = 4;
const CALL: u8 = 6;
const PREPCALLK: u8 = 8;
const GETK: u8 = 9;
const NEWSLOT: u8 = 11;
// Read by `Function::calls` only.
const LINE: u8 = 0;
const LOADFLOAT: u8 = 3;
const TAILCALL: u8 = 5;
const PREPCALL: u8 = 7;
const MOVE: u8 = 10;
/// `ADD a b c op`: the arithmetic op, its operator's character in arg3.
const ARITH: u8 = 17;
/// `null` and the booleans (`nut.py`'s `DIV` and `BITW`).
const LOADNULL: u8 = 20;
const LOADBOOL: u8 = 22;
const NEG: u8 = 45;
/// `@x` (relative to the current value).
const RELATIVE: u8 = 61;

#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    Str(String),
    Int(i32),
    Other,
}

/// A call's argument as [`Function::calls`] reads it.
#[derive(Debug, Clone, PartialEq)]
pub enum Arg {
    Int(i32),
    Str(String),
    /// A constant read by name: `Center`, `Middle`
    Word(String),
    /// `@n`
    Relative(i32),
    Bool(bool),
    /// Anything else: `null`, variables, results
    Other,
}

#[derive(Debug, Clone, Copy)]
struct Instr {
    op: u8,
    arg1: i32,
    arg3: u8,
}

#[derive(Debug, Default)]
pub struct Function {
    pub literals: Vec<Literal>,
    code: Vec<Instr>,
    pub functions: Vec<Function>,
}

#[derive(Debug, thiserror::Error)]
pub enum SquirrelError {
    #[error("not a compiled script: expected {0:?} at {1:#x}")]
    Tag(&'static str, usize),
    #[error("unknown object type {0:#x} at {1:#x}")]
    Object(u32, usize),
    #[error(transparent)]
    Truncated(#[from] Truncated),
}

struct Cursor<'a> {
    r: Reader<'a>,
    pos: usize,
}

impl Cursor<'_> {
    fn u32(&mut self) -> Result<u32, Truncated> {
        let v = self.r.u32(self.pos)?;
        self.pos += 4;
        Ok(v)
    }

    fn tag(&mut self, tag: &'static str) -> Result<(), SquirrelError> {
        if self.r.slice(self.pos, tag.len())? != tag.as_bytes() {
            return Err(SquirrelError::Tag(tag, self.pos));
        }
        self.pos += tag.len();
        Ok(())
    }

    fn object(&mut self) -> Result<Literal, SquirrelError> {
        let at = self.pos;
        Ok(match self.u32()? {
            0x0800_0010 => {
                let n = self.u32()? as usize;
                let s = String::from_utf8_lossy(self.r.slice(self.pos, n)?).into_owned();
                self.pos += n;
                Literal::Str(s)
            }
            0x0500_0002 => Literal::Int(self.u32()? as i32),
            // float, bool
            0x0500_0004 | 0x0100_0008 => {
                self.u32()?;
                Literal::Other
            }
            0x0100_0001 => Literal::Other,
            t => return Err(SquirrelError::Object(t, at)),
        })
    }

    fn function(&mut self) -> Result<Function, SquirrelError> {
        self.tag("TRAP")?;
        self.object()?;
        self.object()?;
        self.tag("TRAP")?;
        let mut n = [0; 8];
        for c in &mut n {
            *c = self.u32()? as usize;
        }
        let [nlit, npar, nout, nloc, nline, ndef, nins, nfun] = n;
        let mut f = Function::default();
        self.tag("TRAP")?;
        for _ in 0..nlit {
            f.literals.push(self.object()?);
        }
        self.tag("TRAP")?;
        for _ in 0..npar {
            self.object()?;
        }
        self.tag("TRAP")?;
        for _ in 0..nout {
            self.u32()?;
            self.object()?;
            self.object()?;
        }
        self.tag("TRAP")?;
        for _ in 0..nloc {
            self.object()?;
            self.pos += 12;
        }
        self.tag("TRAP")?;
        self.pos += nline * 8;
        self.tag("TRAP")?;
        self.pos += ndef * 4;
        self.tag("TRAP")?;
        for _ in 0..nins {
            let arg1 = self.u32()? as i32;
            let b = self.r.slice(self.pos, 4)?;
            let (op, arg3) = (b[0], b[3]);
            self.pos += 4;
            f.code.push(Instr { op, arg1, arg3 });
        }
        self.tag("TRAP")?;
        for _ in 0..nfun {
            f.functions.push(self.function()?);
        }
        self.pos += 6;
        Ok(f)
    }
}

/// The main function of a compiled script.
pub fn load(bytes: &[u8]) -> Result<Function, SquirrelError> {
    let r = Reader(bytes);
    if r.slice(0, 4)? != b"SCRP" {
        return Err(SquirrelError::Tag("SCRP", 0));
    }
    let mut c = Cursor { pos: r.u32(4)? as usize, r };
    if c.r.slice(c.pos, 2)? != [0xfa, 0xfa] {
        return Err(SquirrelError::Tag("\u{fafa}", c.pos));
    }
    c.pos += 2;
    c.tag("RIQS")?;
    c.u32()?;
    let main = c.function()?;
    c.tag("LIAT")?;
    Ok(main)
}

impl Function {
    fn str(&self, i: i32) -> Option<&str> {
        match self.literals.get(usize::try_from(i).ok()?) {
            Some(Literal::Str(s)) => Some(s),
            _ => None,
        }
    }

    /// Every `name(int, "string", …)` call in this function and the ones
    /// inside it: the scenes' `Print(key, japanese)`.
    pub fn calls_int_str(&self, name: &str, out: &mut Vec<(i32, String)>) {
        for w in self.code.windows(3) {
            if w[0].op == PREPCALLK && self.str(w[0].arg1) == Some(name) && w[1].op == LOADINT && w[2].op == LOAD
                && let Some(s) = self.str(w[2].arg1)
            {
                out.push((w[1].arg1, s.to_string()));
            }
        }
        self.functions.iter().for_each(|f| f.calls_int_str(name, out));
    }

    /// Every string loaded right before a `name(…)` call is prepared, in
    /// this function and the ones inside it: ma01_026's
    /// `text = "…"; SetText("type_text01", text)`.
    pub fn strs_before_call(&self, name: &str, out: &mut Vec<String>) {
        for w in self.code.windows(2) {
            if w[0].op == LOAD && w[1].op == PREPCALLK && self.str(w[1].arg1) == Some(name)
                && let Some(s) = self.str(w[0].arg1)
            {
                out.push(s.to_string());
            }
        }
        self.functions.iter().for_each(|f| f.strs_before_call(name, out));
    }

    /// Every call to a function named by a literal (`PREPCALLK`), in this
    /// function and the ones inside it, with its arguments in the order
    /// they are loaded. A call whose arguments are built by code this
    /// doesn't follow (table literals, jumps) has none.
    pub fn calls(&self, out: &mut Vec<(String, Vec<Arg>)>) {
        // (name, arguments, followed)
        let mut open: Vec<(String, Vec<Arg>, bool)> = Vec::new();
        for ins in &self.code {
            let lit = |i: i32| self.str(i).map(str::to_string);
            if ins.op == PREPCALLK || ins.op == PREPCALL {
                let name = if ins.op == PREPCALLK { lit(ins.arg1) } else { None };
                open.push((name.unwrap_or_default(), Vec::new(), true));
                continue;
            }
            let Some((_, args, ok)) = open.last_mut() else { continue };
            match ins.op {
                LINE => {}
                LOAD => args.push(lit(ins.arg1).map_or(Arg::Other, Arg::Str)),
                LOADINT => args.push(Arg::Int(ins.arg1)),
                DLOAD => args.extend([ins.arg1, i32::from(ins.arg3)].map(|l| lit(l).map_or(Arg::Other, Arg::Str))),
                GETK => args.push(lit(ins.arg1).map_or(Arg::Other, Arg::Word)),
                LOADBOOL => args.push(Arg::Bool(ins.arg1 != 0)),
                LOADFLOAT | MOVE | LOADNULL => args.push(Arg::Other),
                NEG => match args.last_mut() {
                    Some(Arg::Int(n)) => *n = -*n,
                    _ => *ok = false,
                },
                RELATIVE => match args.last_mut() {
                    Some(a @ Arg::Int(_)) => {
                        let Arg::Int(n) = *a else { unreachable!() };
                        *a = Arg::Relative(n);
                    }
                    _ => *ok = false,
                },
                ARITH => {
                    let (b, a) = (args.pop(), args.pop());
                    args.push(match (a, b) {
                        (Some(Arg::Int(a)), Some(Arg::Int(b))) => match ins.arg3 {
                            b'+' => Arg::Int(a + b),
                            b'-' => Arg::Int(a - b),
                            b'*' => Arg::Int(a * b),
                            _ => Arg::Other,
                        },
                        _ => Arg::Other,
                    });
                }
                CALL | TAILCALL => {
                    let (name, args, ok) = open.pop().expect("an open call");
                    out.push((name, if ok { args } else { Vec::new() }));
                    if let Some((_, args, _)) = open.last_mut() {
                        args.push(Arg::Other);
                    }
                }
                _ => *ok = false,
            }
        }
        self.functions.iter().for_each(|f| f.calls(out));
    }

    /// The string literals loaded by `LOAD`/`DLOAD` in `code`, in order.
    fn loaded_strs(&self, code: &[Instr]) -> Vec<String> {
        let mut out = Vec::new();
        for ins in code {
            let lits: &[i32] = match ins.op {
                LOAD => &[ins.arg1],
                DLOAD => &[ins.arg1, i32::from(ins.arg3)],
                _ => &[],
            };
            out.extend(lits.iter().filter_map(|&l| self.str(l)).map(str::to_string));
        }
        out
    }

    /// Every `name(int, "string", "string", …)` call in this function and
    /// the ones inside it: the scenes' `SetChoice(id, japanese…)`.
    pub fn calls_int_strs(&self, name: &str, out: &mut Vec<(i32, Vec<String>)>) {
        for (i, w) in self.code.windows(2).enumerate() {
            if w[0].op == PREPCALLK && self.str(w[0].arg1) == Some(name) && w[1].op == LOADINT {
                let args = &self.code[i + 2..];
                let end = args.iter().position(|ins| ins.op == CALL).unwrap_or(args.len());
                out.push((w[1].arg1, self.loaded_strs(&args[..end])));
            }
        }
        self.functions.iter().for_each(|f| f.calls_int_strs(name, out));
    }

    /// Every `table["key"] <- "string"` in this function: the English
    /// text files' `TransChoice["10_1"] <- english`.
    pub fn table_str_rows(&self, table: &str) -> Vec<(String, String)> {
        let mut out = Vec::new();
        for (i, ins) in self.code.iter().enumerate() {
            if ins.op != GETK || self.str(ins.arg1) != Some(table) {
                continue;
            }
            let rest = &self.code[i + 1..];
            let Some(end) = rest.iter().take(3).position(|ins| ins.op == NEWSLOT) else { continue };
            if let [k, v] = self.loaded_strs(&rest[..end]).as_slice() {
                out.push((k.clone(), v.clone()));
            }
        }
        out
    }

    /// Every `table[int] <- "string"` in this function: the English text
    /// files' `TransText[key] <- english`.
    pub fn table_rows(&self, table: &str) -> Vec<(i32, String)> {
        let mut out = Vec::new();
        for w in self.code.windows(4) {
            if w[0].op == GETK && self.str(w[0].arg1) == Some(table) && w[1].op == LOADINT && w[2].op == LOAD && w[3].op == NEWSLOT
                && let Some(s) = self.str(w[2].arg1)
            {
                out.push((w[1].arg1, s.to_string()));
            }
        }
        out
    }
}
