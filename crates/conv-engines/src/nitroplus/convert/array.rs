//! The built-ins that take a variable rather than its value: `Array`,
//! `AssocArray` and `Count` (装甲悪鬼村正's ending table
//! `$EndName[$ENDNumber][0]`), `VariableValue` (a variable named by a
//! string, `#ClearRoll%02d`), and `String` (printf).
//!
//! An element is the variable `name[i]`; `Array(v, a, b)` sets `v[0]`,
//! `v[1]` and leaves `v`'s own value alone (`$EndName[0]` is "英雄編" and
//! `$EndName[0][0]` "Roll01"). `AssocArray(v, k0, k1)` lets `v["k1"]`
//! mean `v[1]`.

use super::*;

/// Where an array's length and its key names are kept among the variables.
fn count_key(key: &str) -> String {
    format!("{key}#count")
}
fn assoc_key(key: &str, name: &str) -> String {
    format!("{key}#assoc:{name}")
}

impl<'a> Compiler<'a, '_> {
    /// `base[index]`, a key name given to `AssocArray` meaning its place.
    pub(super) fn element_key(&self, base: &str, index: &Value) -> String {
        if let Value::Str(s) = index
            && let Some(i) = self.vars.get(&assoc_key(base, s))
        {
            return format!("{base}[{}]", i.int());
        }
        format!("{base}[{}]", index.string())
    }

    /// One of these built-ins, or `None` for any other call.
    pub(super) fn variable_call(&mut self, name: &str, args: &'a [Expr]) -> Option<Value> {
        let first = args.first()?;
        match name {
            "Array" | "AssocArray" => {
                let key = self.index_key(first);
                let values: Vec<Value> = args[1..].iter().map(|a| self.eval(a)).collect();
                for (i, v) in values.iter().enumerate() {
                    match name {
                        "Array" => self.vars.insert(format!("{key}[{i}]"), v.clone()),
                        _ => self.vars.insert(assoc_key(&key, &v.string()), Value::Int(i as i64)),
                    };
                }
                if name == "Array" {
                    self.vars.insert(count_key(&key), Value::Int(values.len() as i64));
                }
                Some(Value::Null)
            }
            "Count" => {
                let key = self.index_key(first);
                Some(self.vars.get(&count_key(&key)).cloned().unwrap_or(Value::Int(0)))
            }
            "VariableValue" => {
                let key = self.variable_value_key(args)?;
                match args.get(2) {
                    None => Some(self.vars.get(&key).cloned().unwrap_or(Value::Null)),
                    Some(v) => {
                        let value = self.eval(v);
                        let old = self.vars.get(&key).cloned().unwrap_or(Value::Null);
                        if let Some(var) = self.flags.get(&key).copied() {
                            self.assign_flag(var, AssignOp::Set, &old, &value, &value);
                        }
                        self.vars.insert(key, value);
                        Some(Value::Null)
                    }
                }
            }
            "String" => {
                let format = self.eval(first).string();
                let values: Vec<Value> = args[1..].iter().map(|a| self.eval(a)).collect();
                Some(Value::Str(printf(&format, &values)))
            }
            _ => None,
        }
    }

    /// The variable `VariableValue(#, "name")` names: `#name`.
    pub(super) fn variable_value_key(&mut self, args: &'a [Expr]) -> Option<String> {
        let scope = match args.first()? {
            Expr::Word(w) if w == "#" || w == "$" => w.clone(),
            _ => return None,
        };
        let name = self.eval(args.get(1)?).string();
        Some(format!("{scope}{name}"))
    }
}

/// `%d`, `%Nd`, `%0Nd` and `%s` of C's printf.
fn printf(format: &str, values: &[Value]) -> String {
    let mut out = String::new();
    let mut values = values.iter();
    let mut chars = format.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '%' {
            out.push(c);
            continue;
        }
        let mut spec = String::new();
        while let Some(&d) = chars.peek().filter(|d| d.is_ascii_digit()) {
            spec.push(d);
            chars.next();
        }
        let v = match chars.next() {
            Some('%') => {
                out.push('%');
                continue;
            }
            Some('d') => values.next().map(Value::int).unwrap_or(0).to_string(),
            Some('s') => values.next().map(Value::string).unwrap_or_default(),
            other => {
                out.push('%');
                out.push_str(&spec);
                out.extend(other);
                continue;
            }
        };
        let width: usize = spec.parse().unwrap_or(0);
        let pad = if spec.starts_with('0') { '0' } else { ' ' };
        out.extend(std::iter::repeat_n(pad, width.saturating_sub(v.chars().count())));
        out.push_str(&v);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_like_printf() {
        let v = [Value::Int(3), Value::Str("雑魚".into())];
        assert_eq!(printf("ClearRoll%02d", &v), "ClearRoll03");
        assert_eq!(printf("%s/%04d", &[v[1].clone(), v[0].clone()]), "雑魚/0003");
        assert_eq!(printf("H_%2d 100%%", &v), "H_ 3 100%");
    }
}
