//! NSS scripts: the C-like source language of Nitroplus's engine, stored as
//! Shift-JIS text in `nss.npa` and interpreted by the engine at run time.
//!
//! A file holds `#include "nss/x.nss"` lines and three kinds of blocks:
//!
//! ```text
//! chapter main { … }              entry point when the file is started
//! scene ma01_001.nss_MAIN { … }   called with call_scene (names may hold '.')
//! function FadeM("ナット名", 描画時間, $x) { … }
//! ```
//!
//! Statements: `$local`/`#global` assignment (`=`, `+=`, `-=`, `*=`, `/=`,
//! `++`, `--`), calls, `if`/`else`, `while`, `select { … case @label { … } }`,
//! `call_scene $var;` / `call_scene @->name;`, `call_chapter nss/x.nss;`,
//! `return`, `break`, labels (`begin:`) and text blocks.
//!
//! Parameters are bound by token: a parameter may be a `$var`, a bare word
//! (`描画時間`) or even a string literal (`"ナット名"`), and the body refers to
//! it with the same token. Bare words that are not parameters are names the
//! engine knows (`Center`, `null`, `true`, `Start`, `Black`, …).
//!
//! Expressions add `@` (relative value: `Move("x", 0, @$dx, @0, …)`), colours
//! (`#a44809`) and Bézier paths for `BezierMove`: a run of points, `(x,y)`
//! for points on the curve and `{x,y}` for control points.
//!
//! Text blocks (`<PRE @box0> … </PRE>`) hold `[text0010]` markers, text with
//! tags (`<voice name=… class=… src=…>`, `<RUBY text="…">…</RUBY>`, `<BR>`,
//! `<k>`, `<FONT …>`), `//` comment lines and inline code in braces
//! (`「……{WaitKey(1100);}ああ」`).
//!
//! Comments: `//`, `/* */` (strings inside are skipped), and lines that
//! start with dots (`..//ジャンプ指定`, `..ランダム明滅`).
//!
//! The shipped scripts contain mistakes the original engine runs anyway.
//! Accepted as the engine presumably reads them: `$a++:` (colon for
//! semicolon), a missing `;` at the end of a line, `Fade("x", 0 1000, …)`
//! (missing comma), `@` alone (= `@0`), unquoted wildcards
//! (`Delete(@OnBG*)`), `$$name`, a bare `#`/`$` (the scope argument of
//! `VariableValue`), `*/=====`. Anything else that doesn't parse becomes
//! [`StmtKind::Invalid`] (or is skipped between blocks) and is listed in
//! [`NssFile::errors`]: 19 places in 装甲悪鬼村正, all typos or dead code
//! (an extra `}`, a commented-out function header, `"a".1500.true`).
//!
//! Strings may name variables: `Fade("$現ナット名", …)` fades the object
//! whose name is in `$現ナット名`. That is left to the converter.

use std::fmt;

/// One parsed `.nss` file.
#[derive(Debug, Clone, PartialEq)]
pub struct NssFile {
    pub items: Vec<Item>,
    /// Mistakes in the source that the parser skipped over (see
    /// [`StmtKind::Invalid`]); the original engine runs these files too.
    pub errors: Vec<ParseError>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Item {
    Include(String),
    Chapter(Block),
    Scene(Block),
    Function(Function),
}

/// A `chapter` or `scene` block.
#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    pub name: String,
    pub line: u32,
    pub body: Vec<Stmt>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Function {
    pub name: String,
    pub line: u32,
    pub params: Vec<Param>,
    pub body: Vec<Stmt>,
}

/// A parameter is bound by the token that names it.
#[derive(Debug, Clone, PartialEq)]
pub enum Param {
    Var(Var),
    Word(String),
    Str(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Var {
    /// `$name`
    Local(String),
    /// `#name`, kept with the save data / across games
    Global(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Stmt {
    pub line: u32,
    pub kind: StmtKind,
}

#[derive(Debug, Clone, PartialEq)]
pub enum StmtKind {
    Expr(Expr),
    /// `$a = 1;`, `$a[0][$i] += 2;`, `$i++;`
    Assign { target: Var, index: Vec<Expr>, op: AssignOp, value: Option<Expr> },
    If { cond: Expr, then: Vec<Stmt>, otherwise: Option<Vec<Stmt>> },
    While { cond: Expr, body: Vec<Stmt> },
    Select(Vec<Stmt>),
    /// `case @label { … }`, only inside `select`. The label is kept as
    /// written: `@選択肢１`, `bk出口`, `MSGWND/MSG_yes`
    Case { label: String, body: Vec<Stmt> },
    CallScene(Target),
    CallChapter(Target),
    Return,
    Break,
    Label(String),
    Block(Vec<Stmt>),
    Text(Text),
    /// A `<voice …>` tag outside a text block: voice-only lines whose text
    /// was commented out (「テキスト表示せず。ボイスのみ」, mc01_026, md02_043a)
    Tag(Tag),
    /// A statement that doesn't parse, kept as source text: typos in the
    /// shipped scripts (`if(a||b)){…}`, `$x="a"+;`). The error is also in
    /// [`NssFile::errors`].
    Invalid(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssignOp {
    Set,
    Add,
    Sub,
    Mul,
    Div,
    /// `++`, no value
    Inc,
    /// `--`, no value
    Dec,
}

/// What `call_scene` / `call_chapter` names.
#[derive(Debug, Clone, PartialEq)]
pub enum Target {
    /// A variable holding the name (`$MainGameName`, usually `"@->" + file`)
    Var(Var),
    /// Written out: `@->ma01_001.nss_MAIN`, `nss/sys_load.nss`
    Name(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Int(i64),
    Float(f64),
    Str(String),
    /// `#rrggbb`
    Color(u32),
    /// A bare word: an engine constant or a token-bound parameter
    Word(String),
    Var(Var),
    /// `$a[i]`, `#Cg[$i][$j]`, `$Name[画像]`
    Index { base: Box<Expr>, index: Box<Expr> },
    Call { name: String, args: Vec<Expr> },
    Unary { op: UnaryOp, expr: Box<Expr> },
    Binary { op: BinaryOp, left: Box<Expr>, right: Box<Expr> },
    /// `(x, y)` (`control` false) or `{x, y}` (`control` true)
    Point { control: bool, x: Box<Expr>, y: Box<Expr> },
    /// Juxtaposed points, the curve argument of `BezierMove`
    Path(Vec<Expr>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Neg,
    Not,
    /// `@`: relative to the current value
    Relative,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    Or,
    And,
    Eq,
    Ne,
    Lt,
    Gt,
    Le,
    Ge,
    Add,
    Sub,
    Mul,
    Div,
    Rem,
}

impl fmt::Display for Var {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Var::Local(n) => write!(f, "${n}"),
            Var::Global(n) => write!(f, "#{n}"),
        }
    }
}

impl BinaryOp {
    fn symbol(self) -> &'static str {
        match self {
            BinaryOp::Or => "||",
            BinaryOp::And => "&&",
            BinaryOp::Eq => "==",
            BinaryOp::Ne => "!=",
            BinaryOp::Lt => "<",
            BinaryOp::Gt => ">",
            BinaryOp::Le => "<=",
            BinaryOp::Ge => ">=",
            BinaryOp::Add => "+",
            BinaryOp::Sub => "-",
            BinaryOp::Mul => "*",
            BinaryOp::Div => "/",
            BinaryOp::Rem => "%",
        }
    }
}

/// Source-like form (fully parenthesised binary operations).
impl fmt::Display for Expr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Expr::Int(v) => write!(f, "{v}"),
            Expr::Float(v) => write!(f, "{v}"),
            Expr::Str(s) => write!(f, "\"{s}\""),
            Expr::Color(c) => write!(f, "#{c:06x}"),
            Expr::Word(w) => f.write_str(w),
            Expr::Var(v) => write!(f, "{v}"),
            Expr::Index { base, index } => write!(f, "{base}[{index}]"),
            Expr::Call { name, args } => {
                write!(f, "{name}(")?;
                for (i, a) in args.iter().enumerate() {
                    if i > 0 {
                        f.write_str(", ")?;
                    }
                    write!(f, "{a}")?;
                }
                f.write_str(")")
            }
            Expr::Unary { op, expr } => {
                let op = match op {
                    UnaryOp::Neg => "-",
                    UnaryOp::Not => "!",
                    UnaryOp::Relative => "@",
                };
                write!(f, "{op}{expr}")
            }
            Expr::Binary { op, left, right } => write!(f, "({left} {} {right})", op.symbol()),
            Expr::Point { control: false, x, y } => write!(f, "({x},{y})"),
            Expr::Point { control: true, x, y } => write!(f, "{{{x},{y}}}"),
            Expr::Path(points) => points.iter().try_for_each(|p| write!(f, "{p}")),
        }
    }
}

/// A `<PRE>` text block.
#[derive(Debug, Clone, PartialEq)]
pub struct Text {
    /// `@box0`, or `None` for a bare `<PRE>`
    pub window: Option<String>,
    pub items: Vec<Inline>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Inline {
    /// `[text0010]` on a line of its own
    Marker(String),
    Str(String),
    Tag(Tag),
    Code(Vec<Stmt>),
    Comment(String),
    Newline,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Tag {
    /// As written (`voice`, `RUBY`, `BR`, `k`, …)
    pub name: String,
    pub close: bool,
    pub attrs: Vec<(String, String)>,
}

impl Tag {
    pub fn attr(&self, key: &str) -> Option<&str> {
        self.attrs.iter().find(|(k, _)| k.eq_ignore_ascii_case(key)).map(|(_, v)| v.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    pub line: u32,
    pub message: String,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "line {}: {}", self.line, self.message)
    }
}

impl std::error::Error for ParseError {}

type Result<T> = std::result::Result<T, ParseError>;

/// Decodes a script file's Shift-JIS (CP932) bytes.
pub fn decode(bytes: &[u8]) -> String {
    encoding_rs::SHIFT_JIS.decode(bytes).0.into_owned()
}

/// Parses a whole file (already decoded, see [`decode`]).
pub fn parse(src: &str) -> Result<NssFile> {
    let src = src.replace("\r\n", "\n");
    let tokens = lex(&src, 1)?;
    let mut p = Parser { src: &src, tokens: &tokens, pos: 0, errors: Vec::new() };
    let mut items = Vec::new();
    while !p.at_end() {
        let start = p.pos;
        match p.item() {
            Ok(item) => items.push(item),
            Err(e) => {
                // Stray `}` or `*/`, statements after a block that closed
                // early, a broken first line: skip to the next item.
                p.errors.push(e);
                p.pos = start + 1;
                while !p.at_end() && !p.at_item() {
                    p.pos += 1;
                }
            }
        }
    }
    Ok(NssFile { items, errors: p.errors })
}

// ---------------------------------------------------------------------------
// Lexer

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Word(String),
    Var(Var),
    Color(u32),
    Int(i64),
    Float(f64),
    Str(String),
    Punct(&'static str),
    /// Body of `<PRE …>` … `</PRE>`, with its first line number
    Pre { window: Option<String>, body: String, line: u32 },
    /// `<voice …>` outside a text block
    Tag(Tag),
}

#[derive(Debug, Clone)]
struct Token {
    tok: Tok,
    line: u32,
    start: usize,
    end: usize,
}

const PUNCTS: &[&str] = &[
    "++", "--", "+=", "-=", "*=", "/=", "==", "!=", "<=", ">=", "&&", "||", "(", ")", "{", "}", "[",
    "]", ";", ",", ":", "=", "+", "-", "*", "/", "%", "<", ">", "!", "@",
];

/// Characters that can't be part of a word, variable or number.
fn is_delim(c: char) -> bool {
    c.is_whitespace() || c == '\u{3000}' || "(){}[];,:=+-*/%<>!&|\"@$#'?".contains(c)
}

fn err<T>(line: u32, message: impl Into<String>) -> Result<T> {
    Err(ParseError { line, message: message.into() })
}

fn lex(src: &str, first_line: u32) -> Result<Vec<Token>> {
    let mut out = Vec::new();
    let mut line = first_line;
    let mut i = 0;
    let b = src.as_bytes();
    while i < src.len() {
        let c = src[i..].chars().next().unwrap();
        let rest = &src[i..];
        if c == '\n' {
            line += 1;
            i += 1;
        } else if c.is_whitespace() || c == '\u{3000}' {
            i += c.len_utf8();
        } else if rest.starts_with("//") {
            i += rest.find('\n').unwrap_or(rest.len());
        } else if (c == '.' && src[..i].trim_end_matches([' ', '\t']).ends_with('\n')) || rest.starts_with("$Revision:") {
            // `..//ジャンプ指定`, `..ランダム明滅`, `..\t//■準備`, and the
            // SVN keyword `$Revision: 28 $` some files start with
            i += rest.find('\n').unwrap_or(rest.len());
        } else if rest.starts_with("/*") {
            // Strings are skipped: commented-out code holds names like
            // "Config0*/*/*".
            let Some(n) = block_comment_len(rest) else { return err(line, "unterminated /* comment") };
            line += rest[..n].matches('\n').count() as u32;
            i += n;
            // `*/=====` closes some banner comments
            i += src[i..].len() - src[i..].trim_start_matches('=').len();
        } else if rest.starts_with("<PRE") && rest[4..].starts_with(|c: char| c == '>' || c.is_whitespace()) {
            let Some(gt) = rest.find('>') else { return err(line, "unterminated <PRE") };
            let window = rest[4..gt].trim();
            let window = (!window.is_empty()).then(|| window.to_string());
            // Text can hold whole `<PRE>“…”</PRE>` lines (quoted English).
            let Some(end) = matching_pre_end(rest) else { return err(line, "<PRE> without </PRE>") };
            let body = rest[gt + 1..end].to_string();
            out.push(Token { tok: Tok::Pre { window, body, line }, line, start: i, end: i + end + 6 });
            line += rest[..end].matches('\n').count() as u32;
            i += end + 6;
        } else if rest.starts_with("<voice") {
            let Some(gt) = rest.find('>') else { return err(line, "unterminated <voice") };
            out.push(Token { tok: Tok::Tag(parse_tag(&rest[1..gt], line)?), line, start: i, end: i + gt + 1 });
            i += gt + 1;
        } else if c == '"' {
            let Some(n) = rest[1..].find(['"', '\n']) else { return err(line, "unterminated string") };
            if rest.as_bytes()[1 + n] == b'\n' {
                return err(line, "newline in string");
            }
            out.push(Token { tok: Tok::Str(rest[1..1 + n].to_string()), line, start: i, end: i + n + 2 });
            i += n + 2;
        } else if c == '$' || c == '#' {
            // `$$name` (4 uses) keeps its second '$' in the name
            let skip = if rest[1..].starts_with('$') { 2 } else { 1 };
            let n = skip - 1 + rest[skip..].find(is_delim).unwrap_or(rest.len() - skip);
            let name = &rest[1..1 + n];
            let tok = if name.is_empty() {
                // `VariableValue(#, name)`: the scope on its own
                Tok::Word(c.to_string())
            } else if c == '#' && name.len() == 6 && name.bytes().all(|b| b.is_ascii_hexdigit()) {
                Tok::Color(u32::from_str_radix(name, 16).unwrap())
            } else if c == '$' {
                Tok::Var(Var::Local(name.to_string()))
            } else {
                Tok::Var(Var::Global(name.to_string()))
            };
            out.push(Token { tok, line, start: i, end: i + 1 + n });
            i += 1 + n;
        } else if let Some(p) = PUNCTS.iter().find(|p| rest.starts_with(**p)) {
            out.push(Token { tok: Tok::Punct(p), line, start: i, end: i + p.len() });
            i += p.len();
        } else {
            let mut n = rest.find(is_delim).unwrap_or(rest.len());
            // Unquoted wildcards: `Delete(@OnBG*);`, `FadeDelete(@StR*, …)`
            if rest[n..].starts_with('*') && rest[n + 1..].starts_with([',', ')']) {
                n += 1;
            }
            let word = &rest[..n];
            let tok = if word.bytes().all(|b| b.is_ascii_digit()) {
                match word.parse() {
                    Ok(v) => Tok::Int(v),
                    Err(_) => return err(line, format!("number out of range: {word}")),
                }
            } else if b[i].is_ascii_digit() && word.parse::<f64>().is_ok() {
                Tok::Float(word.parse().unwrap())
            } else {
                Tok::Word(word.to_string())
            };
            out.push(Token { tok, line, start: i, end: i + n });
            i += n;
        }
    }
    Ok(out)
}

/// Length of the `/* … */` comment at the start of `s`, skipping strings.
fn block_comment_len(s: &str) -> Option<usize> {
    let mut in_str = false;
    let mut i = 2;
    while i < s.len() {
        let rest = &s[i..];
        if rest.starts_with('"') {
            in_str = !in_str;
        } else if rest.starts_with('\n') {
            in_str = false;
        } else if !in_str && rest.starts_with("*/") {
            return Some(i + 2);
        }
        i += rest.chars().next().unwrap().len_utf8();
    }
    None
}

/// Byte index of the `</PRE>` closing the `<PRE` at the start of `s`.
fn matching_pre_end(s: &str) -> Option<usize> {
    let mut depth = 0;
    let mut i = 0;
    while let Some(n) = s[i..].find("PRE") {
        let at = i + n;
        if s[..at].ends_with("</") {
            depth -= 1;
            if depth == 0 {
                return Some(at - 2);
            }
        } else if s[..at].ends_with('<') {
            depth += 1;
        }
        i = at + 3;
    }
    None
}

// ---------------------------------------------------------------------------
// Parser

struct Parser<'a> {
    src: &'a str,
    tokens: &'a [Token],
    pos: usize,
    errors: Vec<ParseError>,
}

impl<'a> Parser<'a> {
    fn at_end(&self) -> bool {
        self.pos >= self.tokens.len()
    }

    /// At `chapter`, `scene`, `function` or `#include`.
    fn at_item(&self) -> bool {
        match self.peek() {
            Some(Tok::Word(w)) => matches!(w.as_str(), "chapter" | "scene" | "function"),
            Some(Tok::Var(Var::Global(g))) => g == "include",
            _ => false,
        }
    }

    fn peek(&self) -> Option<&'a Tok> {
        self.tokens.get(self.pos).map(|t| &t.tok)
    }

    fn peek_at(&self, k: usize) -> Option<&'a Tok> {
        self.tokens.get(self.pos + k).map(|t| &t.tok)
    }

    fn line(&self) -> u32 {
        self.tokens
            .get(self.pos)
            .or(self.tokens.last())
            .map_or(0, |t| t.line)
    }

    fn next(&mut self) -> Result<&'a Token> {
        let t = self.tokens.get(self.pos);
        self.pos += 1;
        t.map_or_else(|| err(self.line(), "unexpected end of file"), Ok)
    }

    fn is(&self, p: &str) -> bool {
        matches!(self.peek(), Some(Tok::Punct(q)) if *q == p)
    }

    fn is_word(&self, w: &str) -> bool {
        matches!(self.peek(), Some(Tok::Word(x)) if x == w)
    }

    fn eat(&mut self, p: &str) -> bool {
        let yes = self.is(p);
        if yes {
            self.pos += 1;
        }
        yes
    }

    fn expect(&mut self, p: &str) -> Result<()> {
        if self.eat(p) {
            Ok(())
        } else {
            err(self.line(), format!("expected '{p}', found {}", self.describe()))
        }
    }

    fn describe(&self) -> String {
        match self.tokens.get(self.pos) {
            Some(t) => format!("'{}'", &self.src[t.start..t.end.min(t.start + 40)]),
            None => "end of file".into(),
        }
    }

    fn word(&mut self) -> Result<String> {
        match self.peek() {
            Some(Tok::Word(w)) => {
                self.pos += 1;
                Ok(w.clone())
            }
            _ => err(self.line(), format!("expected a name, found {}", self.describe())),
        }
    }

    fn item(&mut self) -> Result<Item> {
        let line = self.line();
        match self.peek() {
            Some(Tok::Var(Var::Global(g))) if g == "include" => {
                self.pos += 1;
                let Some(Tok::Str(s)) = self.peek() else {
                    return err(line, "#include without a file name");
                };
                self.pos += 1;
                self.eat(";");
                Ok(Item::Include(s.clone()))
            }
            Some(Tok::Word(w)) if w == "chapter" || w == "scene" => {
                let chapter = w == "chapter";
                self.pos += 1;
                let name = self.word()?;
                let body = self.block()?;
                let b = Block { name, line, body };
                Ok(if chapter { Item::Chapter(b) } else { Item::Scene(b) })
            }
            Some(Tok::Word(w)) if w == "function" => {
                self.pos += 1;
                let name = self.word()?;
                self.expect("(")?;
                let mut params = Vec::new();
                while !self.eat(")") {
                    let t = self.next()?;
                    params.push(match &t.tok {
                        Tok::Var(v) => Param::Var(v.clone()),
                        Tok::Word(w) => Param::Word(w.clone()),
                        Tok::Str(s) => Param::Str(s.clone()),
                        _ => return err(t.line, format!("bad parameter in {name}")),
                    });
                    if !self.is(")") {
                        self.expect(",")?;
                    }
                }
                let body = self.block()?;
                Ok(Item::Function(Function { name, line, params, body }))
            }
            _ => err(line, format!("expected chapter, scene, function or #include, found {}", self.describe())),
        }
    }

    /// `{ stmt* }`
    fn block(&mut self) -> Result<Vec<Stmt>> {
        self.expect("{")?;
        let mut body = Vec::new();
        while !self.eat("}") {
            if self.at_end() {
                return err(self.line(), "missing '}'");
            }
            body.extend(self.stmt_or_invalid());
        }
        Ok(body)
    }

    /// A statement, or on a syntax error an [`StmtKind::Invalid`] covering
    /// the rest of the statement: up to a `;` or the end of the line,
    /// keeping braces balanced.
    fn stmt_or_invalid(&mut self) -> Option<Stmt> {
        let start = self.pos;
        match self.stmt() {
            Ok(s) => s,
            Err(e) => {
                self.pos = start;
                let line = self.tokens[start].line;
                let mut depth = 0;
                while !self.at_end() {
                    let t = &self.tokens[self.pos];
                    if depth == 0 && self.pos > start && t.line > line {
                        break;
                    }
                    match t.tok {
                        Tok::Punct("{") => depth += 1,
                        Tok::Punct("}") if depth == 0 => break,
                        Tok::Punct("}") => depth -= 1,
                        Tok::Punct(";") if depth == 0 => {
                            self.pos += 1;
                            break;
                        }
                        _ => {}
                    }
                    self.pos += 1;
                }
                if self.pos == start {
                    self.pos += 1;
                }
                let (a, b) = (self.tokens[start].start, self.tokens[self.pos - 1].end);
                self.errors.push(e);
                Some(Stmt { line, kind: StmtKind::Invalid(self.src[a..b].to_string()) })
            }
        }
    }

    /// A braced block, or a single statement (`if (x) Foo();`).
    fn body(&mut self) -> Result<Vec<Stmt>> {
        if self.is("{") {
            return self.block();
        }
        Ok(self.stmt()?.into_iter().collect())
    }

    /// `;`, or what the engine accepts instead: `:`, the end of the block,
    /// or a line break (`Window_Delete(0,true)` with no `;`).
    fn end_stmt(&mut self) -> Result<()> {
        if self.eat(";") || self.eat(":") || self.is("}") || self.at_end() {
            return Ok(());
        }
        if self.pos > 0 && self.tokens[self.pos].line > self.tokens[self.pos - 1].line {
            return Ok(());
        }
        err(self.line(), format!("expected ';', found {}", self.describe()))
    }

    /// One statement; `None` for an empty `;`.
    fn stmt(&mut self) -> Result<Option<Stmt>> {
        let line = self.line();
        let kind = match self.peek() {
            Some(Tok::Punct(";")) => {
                self.pos += 1;
                return Ok(None);
            }
            Some(Tok::Punct("{")) => StmtKind::Block(self.block()?),
            Some(Tok::Pre { window, body, line }) => {
                self.pos += 1;
                StmtKind::Text(Text { window: window.clone(), items: parse_text(body, *line, &mut self.errors)? })
            }
            Some(Tok::Tag(tag)) => {
                self.pos += 1;
                StmtKind::Tag(tag.clone())
            }
            Some(Tok::Word(w)) => match w.as_str() {
                "if" => {
                    self.pos += 1;
                    let cond = self.paren_cond()?;
                    let then = self.body()?;
                    let otherwise = if self.is_word("else") {
                        self.pos += 1;
                        Some(self.body()?)
                    } else {
                        None
                    };
                    StmtKind::If { cond, then, otherwise }
                }
                "while" => {
                    self.pos += 1;
                    let cond = self.paren_cond()?;
                    StmtKind::While { cond, body: self.body()? }
                }
                "select" => {
                    self.pos += 1;
                    StmtKind::Select(self.block()?)
                }
                "case" => {
                    // `case @選択肢１`, `case bk出口`, `case MSGWND/MSG_yes`
                    self.pos += 1;
                    let label = self.raw_until(&["{"])?;
                    StmtKind::Case { label, body: self.block()? }
                }
                "call_scene" | "call_chapter" => {
                    let scene = w == "call_scene";
                    self.pos += 1;
                    let target = self.target()?;
                    self.end_stmt()?;
                    if scene { StmtKind::CallScene(target) } else { StmtKind::CallChapter(target) }
                }
                "return" | "break" => {
                    let ret = w == "return";
                    self.pos += 1;
                    self.end_stmt()?;
                    if ret { StmtKind::Return } else { StmtKind::Break }
                }
                _ if matches!(self.peek_at(1), Some(Tok::Punct(":"))) => {
                    self.pos += 2;
                    StmtKind::Label(w.clone())
                }
                _ => {
                    let e = self.expr()?;
                    self.end_stmt()?;
                    StmtKind::Expr(e)
                }
            },
            Some(Tok::Var(v)) => {
                let save = self.pos;
                self.pos += 1;
                let mut index = Vec::new();
                while self.eat("[") {
                    index.push(self.expr()?);
                    self.expect("]")?;
                }
                let op = match self.peek() {
                    Some(Tok::Punct("=")) => Some(AssignOp::Set),
                    Some(Tok::Punct("+=")) => Some(AssignOp::Add),
                    Some(Tok::Punct("-=")) => Some(AssignOp::Sub),
                    Some(Tok::Punct("*=")) => Some(AssignOp::Mul),
                    Some(Tok::Punct("/=")) => Some(AssignOp::Div),
                    Some(Tok::Punct("++")) => Some(AssignOp::Inc),
                    Some(Tok::Punct("--")) => Some(AssignOp::Dec),
                    _ => None,
                };
                match op {
                    Some(op) => {
                        self.pos += 1;
                        let value = match op {
                            AssignOp::Inc | AssignOp::Dec => None,
                            _ => Some(self.expr()?),
                        };
                        self.end_stmt()?;
                        StmtKind::Assign { target: v.clone(), index, op, value }
                    }
                    None => {
                        self.pos = save;
                        let e = self.expr()?;
                        self.end_stmt()?;
                        StmtKind::Expr(e)
                    }
                }
            }
            _ => {
                let e = self.expr()?;
                self.end_stmt()?;
                StmtKind::Expr(e)
            }
        };
        Ok(Some(Stmt { line, kind }))
    }

    fn paren_cond(&mut self) -> Result<Expr> {
        self.expect("(")?;
        let e = self.expr()?;
        self.expect(")")?;
        Ok(e)
    }

    /// `$var` or a name written out up to the `;` (`@->x.nss_MAIN`, `nss/x.nss`).
    fn target(&mut self) -> Result<Target> {
        if let (Some(Tok::Var(v)), Some(Tok::Punct(";"))) = (self.peek(), self.peek_at(1)) {
            self.pos += 1;
            return Ok(Target::Var(v.clone()));
        }
        Ok(Target::Name(self.raw_until(&[";", "}"])?))
    }

    /// The source text of the tokens up to one of `stops` or the end of
    /// the line, without spaces.
    fn raw_until(&mut self, stops: &[&str]) -> Result<String> {
        let start = self.pos;
        let line = self.line();
        while !self.at_end() && !stops.iter().any(|p| self.is(p)) && self.tokens[self.pos].line == line {
            self.pos += 1;
        }
        if self.pos == start {
            return err(self.line(), format!("expected a name, found {}", self.describe()));
        }
        let (a, b) = (self.tokens[start].start, self.tokens[self.pos - 1].end);
        Ok(self.src[a..b].split_whitespace().collect())
    }

    fn expr(&mut self) -> Result<Expr> {
        self.binary(0)
    }

    fn binary(&mut self, level: usize) -> Result<Expr> {
        const LEVELS: &[&[(&str, BinaryOp)]] = &[
            &[("||", BinaryOp::Or)],
            &[("&&", BinaryOp::And)],
            &[("==", BinaryOp::Eq), ("!=", BinaryOp::Ne)],
            &[("<=", BinaryOp::Le), (">=", BinaryOp::Ge), ("<", BinaryOp::Lt), (">", BinaryOp::Gt)],
            &[("+", BinaryOp::Add), ("-", BinaryOp::Sub)],
            &[("*", BinaryOp::Mul), ("/", BinaryOp::Div), ("%", BinaryOp::Rem)],
        ];
        if level == LEVELS.len() {
            return self.unary();
        }
        let mut left = self.binary(level + 1)?;
        'outer: loop {
            for (p, op) in LEVELS[level] {
                if self.eat(p) {
                    let right = self.binary(level + 1)?;
                    left = Expr::Binary { op: *op, left: Box::new(left), right: Box::new(right) };
                    continue 'outer;
                }
            }
            return Ok(left);
        }
    }

    fn unary(&mut self) -> Result<Expr> {
        if self.eat("+") {
            return self.unary();
        }
        let op = if self.eat("-") {
            UnaryOp::Neg
        } else if self.eat("!") {
            UnaryOp::Not
        } else if self.eat("@") {
            if self.is(",") || self.is(")") {
                // `Move("x", 500, @, @100, …)`: unchanged
                return Ok(Expr::Unary { op: UnaryOp::Relative, expr: Box::new(Expr::Int(0)) });
            }
            UnaryOp::Relative
        } else {
            return self.postfix();
        };
        Ok(Expr::Unary { op, expr: Box::new(self.unary()?) })
    }

    /// A primary with its `[index]`es, or juxtaposed points forming a
    /// Bézier path.
    fn postfix(&mut self) -> Result<Expr> {
        let mut first = self.primary()?;
        while self.eat("[") {
            let index = self.expr()?;
            self.expect("]")?;
            first = Expr::Index { base: Box::new(first), index: Box::new(index) };
        }
        if !matches!(first, Expr::Point { .. }) || !(self.is("(") || self.is("{")) {
            return Ok(first);
        }
        let mut points = vec![first];
        while self.is("(") || self.is("{") {
            match self.primary()? {
                p @ Expr::Point { .. } => points.push(p),
                _ => return err(self.line(), "expected a point in a path"),
            }
        }
        Ok(Expr::Path(points))
    }

    fn primary(&mut self) -> Result<Expr> {
        let t = self.next()?;
        Ok(match &t.tok {
            Tok::Int(v) => Expr::Int(*v),
            Tok::Float(v) => Expr::Float(*v),
            Tok::Str(s) => Expr::Str(s.clone()),
            Tok::Color(c) => Expr::Color(*c),
            Tok::Var(v) => Expr::Var(v.clone()),
            Tok::Word(w) => {
                if self.eat("(") {
                    let mut args = Vec::new();
                    while !self.eat(")") {
                        args.push(self.expr()?);
                        // `Fade("x", 0 1000, …)`: a missing comma between
                        // numbers is taken as a separator
                        if !self.is(")") && !matches!(self.peek(), Some(Tok::Int(_))) {
                            self.expect(",")?;
                        }
                    }
                    Expr::Call { name: w.clone(), args }
                } else {
                    Expr::Word(w.clone())
                }
            }
            Tok::Punct(open @ ("(" | "{")) => {
                let control = *open == "{";
                let x = self.expr()?;
                if !control && self.eat(")") {
                    return Ok(x);
                }
                self.expect(",")?;
                let y = self.expr()?;
                self.expect(if control { "}" } else { ")" })?;
                Expr::Point { control, x: Box::new(x), y: Box::new(y) }
            }
            _ => {
                self.pos -= 1;
                return err(t.line, format!("expected an expression, found {}", self.describe()));
            }
        })
    }
}

// ---------------------------------------------------------------------------
// Text blocks

fn parse_text(body: &str, first_line: u32, errors: &mut Vec<ParseError>) -> Result<Vec<Inline>> {
    let mut items = Vec::new();
    let mut line = first_line;
    let mut run = String::new();
    let mut line_start = true;
    let mut i = 0;
    while i < body.len() {
        let rest = &body[i..];
        let c = rest.chars().next().unwrap();
        if line_start {
            let text = &rest[..rest.find('\n').unwrap_or(rest.len())];
            let trimmed = text.trim();
            if let Some(comment) = trimmed.strip_prefix("//") {
                items.push(Inline::Comment(comment.to_string()));
                i += text.len();
                line_start = false;
                continue;
            }
            if trimmed.len() > 2 && trimmed.starts_with('[') && trimmed.ends_with(']') && !trimmed[1..].contains('[') {
                items.push(Inline::Marker(trimmed[1..trimmed.len() - 1].to_string()));
                i += text.len();
                line_start = false;
                continue;
            }
        }
        line_start = false;
        if c == '\n' {
            flush(&mut run, &mut items);
            items.push(Inline::Newline);
            line += 1;
            line_start = true;
            i += 1;
        } else if c == '<' && rest[1..].starts_with(|c: char| c.is_ascii_alphabetic() || c == '/') {
            let Some(gt) = rest.find('>') else { return err(line, "unterminated tag") };
            flush(&mut run, &mut items);
            items.push(Inline::Tag(parse_tag(&rest[1..gt], line)?));
            i += gt + 1;
        } else if c == '{' {
            // Inline code, possibly over several lines
            let Some(close) = matching_brace(rest) else { return err(line, "unterminated { in text") };
            flush(&mut run, &mut items);
            let code = &rest[1..close];
            let tokens = lex(code, line)?;
            let mut p = Parser { src: code, tokens: &tokens, pos: 0, errors: Vec::new() };
            let mut stmts = Vec::new();
            while !p.at_end() {
                stmts.extend(p.stmt_or_invalid());
            }
            errors.append(&mut p.errors);
            items.push(Inline::Code(stmts));
            line += code.matches('\n').count() as u32;
            i += close + 1;
        } else {
            run.push(c);
            i += c.len_utf8();
        }
    }
    flush(&mut run, &mut items);
    // Drop the newline right after <PRE …> and the one before </PRE>.
    if items.first() == Some(&Inline::Newline) {
        items.remove(0);
    }
    if items.last() == Some(&Inline::Newline) {
        items.pop();
    }
    Ok(items)
}

fn flush(run: &mut String, items: &mut Vec<Inline>) {
    if !run.is_empty() {
        items.push(Inline::Str(std::mem::take(run)));
    }
}

/// Byte index of the `}` closing the `{` at the start of `s`, skipping
/// strings and `//` comments.
fn matching_brace(s: &str) -> Option<usize> {
    let mut depth = 0;
    let mut in_str = false;
    let mut in_comment = false;
    for (i, c) in s.char_indices() {
        if in_comment {
            in_comment = c != '\n';
            continue;
        }
        match c {
            '/' if !in_str && s[i..].starts_with("//") => in_comment = true,
            '"' => in_str = !in_str,
            '{' if !in_str => depth += 1,
            '}' if !in_str => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
    }
    None
}

/// `voice name="景明" src="…"`, `/RUBY`, `BR`.
fn parse_tag(s: &str, line: u32) -> Result<Tag> {
    let (close, s) = match s.strip_prefix('/') {
        Some(rest) => (true, rest),
        None => (false, s),
    };
    let name_end = s.find(char::is_whitespace).unwrap_or(s.len());
    let name = s[..name_end].to_string();
    let mut attrs = Vec::new();
    let mut rest = s[name_end..].trim_start();
    while !rest.is_empty() {
        let key_end = rest.find(|c: char| c == '=' || c.is_whitespace()).unwrap_or(rest.len());
        let key = rest[..key_end].to_string();
        rest = rest[key_end..].trim_start();
        let value = if let Some(r) = rest.strip_prefix('=') {
            let r = r.trim_start();
            if let Some(r) = r.strip_prefix('"') {
                let Some(q) = r.find('"') else { return err(line, format!("unterminated attribute in <{name}>")) };
                rest = r[q + 1..].trim_start();
                r[..q].to_string()
            } else {
                let e = r.find(char::is_whitespace).unwrap_or(r.len());
                rest = r[e..].trim_start();
                r[..e].to_string()
            }
        } else {
            String::new()
        };
        attrs.push((key, value));
    }
    Ok(Tag { name, close, attrs })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn body(src: &str) -> Vec<Stmt> {
        let f = parse(&format!("chapter main\n{{\n{src}\n}}\n")).unwrap();
        match f.items.into_iter().next().unwrap() {
            Item::Chapter(b) => b.body,
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn items_and_comments() {
        let f = parse(
            "#include \"nss/function.nss\"\n.//★版\n..//ジャンプ指定\n/* x\ny */\nscene ma01_001.nss_MAIN\n{\n}\nfunction FadeM(\"ナット名\",描画時間,$x)\n{\n\tFade(\"ナット名\",描画時間,$x,null,false);\n}\n",
        )
        .unwrap();
        assert_eq!(f.items[0], Item::Include("nss/function.nss".into()));
        assert!(matches!(&f.items[1], Item::Scene(b) if b.name == "ma01_001.nss_MAIN" && b.line == 6));
        let Item::Function(func) = &f.items[2] else { panic!() };
        assert_eq!(
            func.params,
            vec![Param::Str("ナット名".into()), Param::Word("描画時間".into()), Param::Var(Var::Local("x".into()))]
        );
        assert_eq!(func.body[0].line, 11);
    }

    #[test]
    fn statements() {
        let s = body(
            "$a = 1 + 2 * 3;\n#フラグ=true;\n$i++:\nif($a != 1 && !#b){ Wait(10); }else if($a){}else Foo();\nbegin:\nwhile(1){ break; }\ncall_scene $MainGameName;\ncall_scene @->ma01_002.nss_MAIN;\ncall_chapter nss/sys_load.nss;\ncall_scene @->x.nss\n$a=1;",
        );
        assert!(matches!(&s[0].kind, StmtKind::Assign { op: AssignOp::Set, value: Some(Expr::Binary { op: BinaryOp::Add, .. }), .. }));
        assert!(matches!(&s[1].kind, StmtKind::Assign { target: Var::Global(g), .. } if g == "フラグ"));
        assert!(matches!(&s[2].kind, StmtKind::Assign { op: AssignOp::Inc, value: None, .. }));
        let StmtKind::If { otherwise: Some(e), .. } = &s[3].kind else { panic!() };
        assert!(matches!(&e[0].kind, StmtKind::If { otherwise: Some(_), .. }));
        assert_eq!(s[4].kind, StmtKind::Label("begin".into()));
        assert_eq!(s[6].kind, StmtKind::CallScene(Target::Var(Var::Local("MainGameName".into()))));
        assert_eq!(s[7].kind, StmtKind::CallScene(Target::Name("@->ma01_002.nss_MAIN".into())));
        assert_eq!(s[8].kind, StmtKind::CallChapter(Target::Name("nss/sys_load.nss".into())));
        assert_eq!(s[9].kind, StmtKind::CallScene(Target::Name("@->x.nss".into())));
        assert!(matches!(s[10].kind, StmtKind::Assign { .. }));
    }

    #[test]
    fn select_relative_colour_and_path() {
        let s = body(
            "select{ case @選択肢１{ $GameName = \"ma02_003a.nss\"; } }\nMove(\"x\", 0, @$x2, @-100, null, true);\nCreateColor(\"c\", 1, 0, 0, 1024, 576, #a44809);\nBezierMove(\"n\", t, (@0,@0){@0,@$Y2}(@$X1,@$Y1), テンポ, 待ち);",
        );
        let StmtKind::Select(cases) = &s[0].kind else { panic!() };
        assert!(matches!(&cases[0].kind, StmtKind::Case { label, .. } if label == "@選択肢１"));
        let StmtKind::Expr(Expr::Call { args, .. }) = &s[1].kind else { panic!() };
        assert!(matches!(&args[2], Expr::Unary { op: UnaryOp::Relative, .. }));
        let StmtKind::Expr(Expr::Call { args, .. }) = &s[2].kind else { panic!() };
        assert_eq!(args[6], Expr::Color(0xa44809));
        let StmtKind::Expr(Expr::Call { args, .. }) = &s[3].kind else { panic!() };
        let Expr::Path(points) = &args[2] else { panic!("{:?}", args[2]) };
        assert_eq!(points.len(), 3);
        assert!(matches!(points[1], Expr::Point { control: true, .. }));
    }

    #[test]
    fn recovers_from_source_errors() {
        let f = parse(
            "}\nchapter main\n{\n\tif(!A()||!$b)){break;}\n\tFadeDelete(\"x\".1500.true);\n\tFade(\"x\", 0 1000, @, Foo*);\n\tWait(1)\n\t<voice name=\"景明\" src=\"voice/a\">\n}\n}\n",
        )
        .unwrap();
        assert_eq!(f.errors.len(), 4, "{:#?}", f.errors);
        let Item::Chapter(b) = &f.items[0] else { panic!() };
        assert_eq!(b.body[0].kind, StmtKind::Invalid("if(!A()||!$b)){break;}".into()));
        assert_eq!(b.body[1].kind, StmtKind::Invalid("FadeDelete(\"x\".1500.true);".into()));
        let StmtKind::Expr(Expr::Call { args, .. }) = &b.body[2].kind else { panic!() };
        assert_eq!(args.len(), 5);
        assert_eq!(args[2], Expr::Int(1000));
        assert_eq!(args[4], Expr::Word("Foo*".into()));
        assert!(matches!(&b.body[3].kind, StmtKind::Expr(Expr::Call { name, .. }) if name == "Wait"));
        assert!(matches!(&b.body[4].kind, StmtKind::Tag(t) if t.attr("src") == Some("voice/a")));
    }

    #[test]
    fn nested_pre_and_strings_in_comments() {
        let s = body("/* Move(\"Config0*/*/*\", 0); */\n<PRE @box0>\n<PRE>“Long ago”</PRE>\n</PRE>\n$a[1][$i] = $b[2];");
        let StmtKind::Text(t) = &s[0].kind else { panic!() };
        assert!(t.items.contains(&Inline::Str("“Long ago”".into())));
        let StmtKind::Assign { index, value: Some(Expr::Index { .. }), .. } = &s[1].kind else { panic!("{:?}", s[1]) };
        assert_eq!(index.len(), 2);
    }

    #[test]
    fn text_block() {
        let s = body(
            "<PRE @box0>\n[text0010]\n<voice name=\"景明\" class=\"景明\" src=\"voice/ma01/0050030a00\">\n「……{WaitKey(1100);}ああ」<BR>\n//◆ＳＥ\n<RUBY text=\"いんば\">印旛</RUBY>\n</PRE>\n\tSetTextEXC();",
        );
        let StmtKind::Text(t) = &s[0].kind else { panic!() };
        assert_eq!(t.window.as_deref(), Some("@box0"));
        assert_eq!(t.items[0], Inline::Marker("text0010".into()));
        let Inline::Tag(v) = &t.items[2] else { panic!() };
        assert_eq!((v.name.as_str(), v.attr("src")), ("voice", Some("voice/ma01/0050030a00")));
        assert_eq!(t.items[4], Inline::Str("「……".into()));
        assert!(matches!(&t.items[5], Inline::Code(c) if c.len() == 1 && c[0].line == 6));
        assert!(t.items.contains(&Inline::Comment("◆ＳＥ".into())));
        assert!(matches!(t.items.last(), Some(Inline::Tag(Tag { close: true, .. }))));
        assert_eq!(s[1].line, 10);
    }
}
