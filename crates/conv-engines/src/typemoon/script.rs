//! The scripts (`allscr.mrg`) and their text (`script_text.mrg`).
//!
//! `allscr.mrg` is an MZP: entry 0 is a table of 32-byte script names
//! (`mrg::names`), entries 1 and 2 are MZPs of their own, and entry
//! `3 + i` is script `i`, MZX-compressed. A script is ASCII (UTF-8 in a
//! few names) with no line breaks: commands `_NAME(arg,arg,…)`, each
//! ended by `;`. Text is not in the scripts: an argument `$000107` is
//! line 107 of `script_text.mrg`.
//!
//! `script_text.mrg` is an MZP of 5 pairs: a table of big-endian u32
//! offsets (padded with `0xFFFFFFFF`), then the UTF-8 lines (CRLF-ended)
//! they point at. Pair 0 is
//! the game's language (Japanese, or English in the fan patch); the other
//! pairs are empty slots of the same length.

use super::FormatError;

#[derive(Debug, PartialEq)]
pub struct Command<'a> {
    /// As written: `_ZM01602`, `_STCH`.
    pub name: &'a str,
    pub args: Vec<&'a str>,
}

impl Command<'_> {
    /// The command without the 5 hex digits the `_Z?` commands carry
    /// (`_ZM01602` is `_ZM`): the name to rank and dispatch on.
    pub fn opcode(&self) -> &str {
        let b = self.name.as_bytes();
        match b {
            [b'_', b'Z', _, rest @ ..] if rest.len() == 5 && rest.iter().all(u8::is_ascii_hexdigit) => &self.name[..3],
            _ => self.name,
        }
    }

    /// The text lines its arguments name (`$000107`).
    pub fn text_refs(&self) -> impl Iterator<Item = usize> + '_ {
        self.args.iter().filter_map(|a| a.strip_prefix('$')?.parse().ok())
    }
}

/// The commands of a script, in order; a malformed one (the original has
/// one: `_STCP)21,0)`) as its raw text.
pub fn parse(script: &str) -> Vec<Result<Command<'_>, &str>> {
    script
        .split(';')
        .filter(|c| !c.is_empty())
        .map(|c| {
            let (name, rest) = c.split_once('(').ok_or(c)?;
            let args = rest.strip_suffix(')').ok_or(c)?;
            if !name.starts_with('_') || name.contains(')') {
                return Err(c);
            }
            Ok(Command { name, args: if args.is_empty() { Vec::new() } else { args.split(',').collect() } })
        })
        .collect()
}

/// The lines of a `script_text.mrg` pair, CRLF removed.
pub fn text<'a>(offsets: &[u8], strings: &'a [u8]) -> Result<Vec<&'a str>, FormatError> {
    // The table is padded with 0xFFFFFFFF.
    let starts: Vec<usize> = offsets
        .as_chunks::<4>()
        .0
        .iter()
        .map(|&o| u32::from_be_bytes(o))
        .take_while(|&o| o != u32::MAX)
        .map(|o| o as usize)
        .collect();
    (0..starts.len())
        .map(|i| {
            let end = starts.get(i + 1).copied().unwrap_or(strings.len());
            let line = strings.get(starts[i]..end).ok_or_else(|| FormatError::Bad(format!("text line {i} at {:#x}..{end:#x}, past the end", starts[i])))?;
            let line = std::str::from_utf8(line).map_err(|e| FormatError::Bad(format!("text line {i}: {e}")))?;
            Ok(line.strip_suffix("\r\n").unwrap_or(line))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commands() {
        let cmds = parse("_ZM01602($000107);_STCH(0,img2345,258-18,,CC);_RTM_();_STCP)21,0);");
        assert_eq!(cmds.len(), 4);
        let zm = cmds[0].as_ref().unwrap();
        assert_eq!((zm.opcode(), zm.text_refs().collect::<Vec<_>>()), ("_ZM", vec![107]));
        assert_eq!(cmds[1].as_ref().unwrap().args, ["0", "img2345", "258-18", "", "CC"]);
        assert_eq!(cmds[1].as_ref().unwrap().opcode(), "_STCH");
        assert_eq!(cmds[2].as_ref().unwrap().args, Vec::<&str>::new());
        assert_eq!(cmds[3], Err("_STCP)21,0)"));
        // Not a Z command with a hex id: kept whole.
        assert_eq!(Command { name: "_ZZe1001x", args: vec![] }.opcode(), "_ZZe1001x");
    }

    #[test]
    fn text_lines() {
        let strings = "一\r\ntwo\r\n".as_bytes();
        let offsets: Vec<u8> = [0u32, 5, u32::MAX].iter().flat_map(|o| o.to_be_bytes()).collect();
        assert_eq!(text(&offsets, strings).unwrap(), ["一", "two"]);
        let bad: Vec<u8> = [0u32, 99].iter().flat_map(|o| o.to_be_bytes()).collect();
        assert!(text(&bad, strings).is_err());
    }
}
