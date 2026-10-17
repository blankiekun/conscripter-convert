//! The text scripts of 5pb./MAGES' TaskList engine (俺たちに翼はない,
//! PS Vita): Shift-JIS, `//` comments, nested `[Name] { … }` blocks
//! holding `:Key value, value …` properties.
//!
//! ```text
//! [TaskList]
//! {
//!     :TaskMax 100001
//!     [Task220]
//!     {
//!         :Tasktype "Message"
//!         :Message  "　自分が普通じゃないって…"
//!         :Name     12        // 鷹志
//!         :Voice    22562     // TAK1004
//!     }
//! }
//! ```
//!
//! A scene file has `[StartUp]` (scene title) and `[TaskList]`, whose
//! `[TaskN]` blocks run in file order. Values are integers, decimals,
//! quoted strings (`\n` is a line break) or bare words; commas separate
//! them; a string left open ends at the end of its line. A key may repeat inside a block and keys are matched without
//! regard to case (the scripts have `:Fadein` beside `:FadeIn`).

use std::cell::RefCell;

/// A property value.
#[derive(Debug, Clone, PartialEq)]
pub enum Val {
    Int(i64),
    Float(f64),
    Str(String),
    Word(String),
}

impl Val {
    pub fn int(&self) -> Option<i64> {
        match self {
            Val::Int(v) => Some(*v),
            Val::Float(v) => Some(*v as i64),
            _ => None,
        }
    }

    pub fn float(&self) -> Option<f64> {
        match self {
            Val::Int(v) => Some(*v as f64),
            Val::Float(v) => Some(*v),
            _ => None,
        }
    }

    pub fn str(&self) -> Option<&str> {
        match self {
            Val::Str(s) | Val::Word(s) => Some(s),
            _ => None,
        }
    }
}

/// A `[name] { … }` block.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Block {
    pub name: String,
    pub props: Vec<(String, Vec<Val>)>,
    pub children: Vec<Block>,
    /// 1-based line of the `[name]`.
    pub line: usize,
    /// Which values the converter read: bit i of entry p is value i of
    /// `props[p]` (`unread` lists the rest).
    pub read: RefCell<Vec<u64>>,
}

impl Block {
    fn find(&self, key: &str) -> Option<(usize, &[Val])> {
        self.props.iter().position(|(k, _)| k.eq_ignore_ascii_case(key)).map(|p| (p, self.props[p].1.as_slice()))
    }

    fn mark(&self, p: usize, bits: u64) {
        let mut read = self.read.borrow_mut();
        read.resize(self.props.len(), 0);
        read[p] |= bits;
    }

    /// The values of the first property `key` (case-insensitive), all
    /// counted as read.
    pub fn get(&self, key: &str) -> Option<&[Val]> {
        let (p, v) = self.find(key)?;
        self.mark(p, !0);
        Some(v)
    }

    /// The `i`th value of `key`.
    pub fn at(&self, key: &str, i: usize) -> Option<&Val> {
        let (p, v) = self.find(key)?;
        self.mark(p, 1 << i.min(63));
        v.get(i)
    }

    pub fn int(&self, key: &str) -> Option<i64> {
        self.at(key, 0)?.int()
    }

    /// The `i`th value of `key` as an integer.
    pub fn int_at(&self, key: &str, i: usize) -> Option<i64> {
        self.at(key, i)?.int()
    }

    pub fn float(&self, key: &str) -> Option<f64> {
        self.at(key, 0)?.float()
    }

    pub fn str(&self, key: &str) -> Option<&str> {
        self.at(key, 0)?.str()
    }

    /// The values nobody read: (key, None, its values) when the property
    /// was never looked at (a repeated key's later copies never are),
    /// else (key, Some(i), [value i]) for each value left.
    pub fn unread(&self) -> Vec<(&str, Option<usize>, &[Val])> {
        let read = self.read.borrow();
        let mut out = Vec::new();
        for (p, (k, vals)) in self.props.iter().enumerate() {
            match read.get(p).copied().unwrap_or(0) {
                0 => out.push((k.as_str(), None, vals.as_slice())),
                bits => out.extend((0..vals.len()).filter(|&i| bits & (1 << i.min(63)) == 0).map(|i| (k.as_str(), Some(i), &vals[i..=i]))),
            }
        }
        out
    }

    /// How many values it was given, its sub-blocks' too (a key without
    /// values counts one).
    pub fn values(&self) -> usize {
        self.props.iter().map(|(_, v)| v.len().max(1)).sum::<usize>() + self.children.iter().map(Block::values).sum::<usize>()
    }

    /// The first child named `name` (case-insensitive).
    pub fn child(&self, name: &str) -> Option<&Block> {
        self.children.iter().find(|c| c.name.eq_ignore_ascii_case(name))
    }

    /// The task's `:Tasktype`.
    pub fn task_type(&self) -> Option<&str> {
        self.str("Tasktype")
    }
}

#[derive(Debug, thiserror::Error)]
#[error("line {line}: {message}")]
pub struct ParseError {
    pub line: usize,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Open(String),
    LBrace,
    RBrace,
    Key(String),
    Val(Val),
}

fn tokenize(src: &str) -> Result<Vec<(Tok, usize)>, ParseError> {
    let chars: Vec<char> = src.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    let mut line = 1;
    let err = |line, m: &str| ParseError { line, message: m.to_string() };
    let word_end = |c: char| c.is_whitespace() || c == '\u{3000}' || ",{}[]:\"".contains(c);
    while i < chars.len() {
        let c = chars[i];
        match c {
            '\n' => {
                line += 1;
                i += 1;
            }
            c if c.is_whitespace() || c == '\u{3000}' || c == ',' || c == '\u{feff}' => i += 1,
            '/' if chars.get(i + 1) == Some(&'/') => {
                while i < chars.len() && chars[i] != '\n' {
                    i += 1;
                }
            }
            '/' if chars.get(i + 1) == Some(&'*') => {
                i += 2;
                while i < chars.len() && !(chars[i] == '*' && chars.get(i + 1) == Some(&'/')) {
                    if chars[i] == '\n' {
                        line += 1;
                    }
                    i += 1;
                }
                i += 2;
            }
            '{' => {
                out.push((Tok::LBrace, line));
                i += 1;
            }
            '}' => {
                out.push((Tok::RBrace, line));
                i += 1;
            }
            '[' => {
                let end = chars[i..].iter().position(|&c| c == ']' || c == '\n').map(|p| i + p);
                match end {
                    Some(e) if chars[e] == ']' => {
                        out.push((Tok::Open(chars[i + 1..e].iter().collect::<String>().trim().to_string()), line));
                        i = e + 1;
                    }
                    _ => return Err(err(line, "unclosed [")),
                }
            }
            ':' => {
                let start = i + 1;
                i = start;
                while i < chars.len() && !word_end(chars[i]) {
                    i += 1;
                }
                out.push((Tok::Key(chars[start..i].iter().collect()), line));
            }
            '"' => {
                let mut s = String::new();
                i += 1;
                loop {
                    match chars.get(i) {
                        // The engine ends an unclosed string at the line's
                        // end (three scripts rely on it).
                        None | Some('\n') => {
                            i -= 1;
                            if s.ends_with('\r') {
                                s.pop();
                            }
                            break;
                        }
                        Some('"') => break,
                        Some('\\') if chars.get(i + 1) == Some(&'n') => {
                            s.push('\n');
                            i += 1;
                        }
                        Some(&c) => s.push(c),
                    }
                    i += 1;
                }
                i += 1;
                out.push((Tok::Val(Val::Str(s)), line));
            }
            _ => {
                let start = i;
                while i < chars.len() && !word_end(chars[i]) && !(chars[i] == '/' && chars.get(i + 1) == Some(&'/')) {
                    i += 1;
                }
                let w: String = chars[start..i].iter().collect();
                let v = if let Ok(n) = w.parse::<i64>() {
                    Val::Int(n)
                } else if let Ok(f) = w.parse::<f64>() {
                    Val::Float(f)
                } else {
                    Val::Word(w)
                };
                out.push((Tok::Val(v), line));
            }
        }
    }
    Ok(out)
}

/// Parses a script into its top-level blocks.
pub fn parse(src: &str) -> Result<Vec<Block>, ParseError> {
    let toks = tokenize(src)?;
    let mut stack = vec![Block::default()];
    let mut pending: Option<(String, usize)> = None;
    let mut i = 0;
    while i < toks.len() {
        let (tok, line) = &toks[i];
        match tok {
            Tok::Open(name) => pending = Some((name.clone(), *line)),
            Tok::LBrace => {
                let (name, line) = pending.take().unwrap_or_default();
                stack.push(Block { name, line, ..Block::default() });
            }
            Tok::RBrace => {
                if stack.len() < 2 {
                    return Err(ParseError { line: *line, message: "unmatched }".into() });
                }
                let b = stack.pop().unwrap();
                stack.last_mut().unwrap().children.push(b);
            }
            Tok::Key(k) => {
                let mut vals = Vec::new();
                while let Some((Tok::Val(v), _)) = toks.get(i + 1) {
                    vals.push(v.clone());
                    i += 1;
                }
                stack.last_mut().unwrap().props.push((k.clone(), vals));
            }
            Tok::Val(_) => {}
        }
        i += 1;
    }
    if stack.len() != 1 {
        return Err(ParseError { line: toks.last().map_or(0, |t| t.1), message: "unclosed {".into() });
    }
    Ok(stack.pop().unwrap().children)
}

/// Decodes a Shift-JIS (cp932) script.
pub fn decode(bytes: &[u8]) -> String {
    encoding_rs::SHIFT_JIS.decode(bytes).0.into_owned()
}

/// A scene file: its title and tasks in order.
#[derive(Debug, Clone)]
pub struct Scene {
    pub title: Option<String>,
    pub tasks: Vec<Block>,
}

impl Scene {
    pub fn parse(src: &str) -> Result<Scene, ParseError> {
        let blocks = parse(src)?;
        let title = blocks.iter().find(|b| b.name == "StartUp").and_then(|b| b.str("SceneName")).map(str::to_string);
        let tasks = blocks.into_iter().find(|b| b.name == "TaskList").map(|b| b.children).unwrap_or_default();
        Ok(Scene { title, tasks })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SRC: &str = "[StartUp]\n{\n\t:SceneName \"序文\"\n}\n// comment [x]\n[TaskList]\n{\n\t:TaskMax\t100001\n\t[Task100]\n\t{\n\t\t:Tasktype\t\"Message\"\n\t\t:Message\t\"a\\nb\" // c\n\t\t:Name\t12\t// 鷹志\n\t\t:Text \"open\r\n\t}\n\t[Task110]\n\t{\u{3000}\n\t\t:Tasktype \"CharacterOpen\"\n\t\t[Chara0]\n\t\t{\n\t\t\t:Number 200,0,1,0,0\n\t\t\t:Scale 0.5\n\t\t}\n\t\t:Action \"=\" ,1\n\t}\n}\n";

    #[test]
    fn scene() {
        let s = Scene::parse(SRC).unwrap();
        assert_eq!(s.title.as_deref(), Some("序文"));
        assert_eq!(s.tasks.len(), 2);
        let m = &s.tasks[0];
        assert_eq!(m.task_type(), Some("Message"));
        assert_eq!(m.str("message"), Some("a\nb"));
        assert_eq!(m.int("Name"), Some(12));
        assert_eq!(m.str("Text"), Some("open"));
        let c = &s.tasks[1];
        let chara = c.child("Chara0").unwrap();
        assert_eq!(chara.int_at("Number", 2), Some(1));
        assert_eq!(chara.float("Scale"), Some(0.5));
        assert_eq!(c.get("Action").unwrap(), &[Val::Str("=".into()), Val::Int(1)]);
        // c's Tasktype was never asked for; of Chara0's Number only value 2.
        let keys = |b: &Block| b.unread().into_iter().map(|(k, i, _)| (k.to_string(), i)).collect::<Vec<_>>();
        assert_eq!(keys(c), vec![("Tasktype".into(), None)]);
        assert_eq!(keys(chara), [0, 1, 3, 4].map(|i| ("Number".to_string(), Some(i))));
        assert!(m.unread().is_empty());
    }
}
