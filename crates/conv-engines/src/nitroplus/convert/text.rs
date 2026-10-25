//! Text: `<PRE>` blocks, their pages and the text window they show in.

use super::*;

impl<'a> Compiler<'a, '_> {
    /// A `<PRE @box>` block: replaces the current text object.
    pub(super) fn text_block(&mut self, t: &'a Text) {
        let window = t.window.as_deref().unwrap_or("@box0").trim_start_matches('@').to_string();
        let marker = t.items.iter().find_map(|i| match i {
            Inline::Marker(m) => Some(m.clone()),
            _ => None,
        });
        let name = format!("{window}/{}", marker.unwrap_or_else(|| format!("text{}", self.seq)));
        if let Some(prev) = self.vars.get("$SYSTEM_present_text").map(Value::string) {
            let prev = prev.trim_start_matches('@').to_string();
            self.objects.remove(&prev);
        }
        let text = TextObject { block: t, at: self.running.last().copied(), layout: self.box_layouts.get(&window).copied(), rotation: 0 };
        let (priority, x, y) = self.objects.get(&window).map_or((20001, 0, 0), |o| (o.priority, o.x, o.y));
        let o = self.create(name.clone(), Kind::Text(text), priority, 0, 0);
        (o.x, o.y) = (x, y);
        self.vars.insert("$SYSTEM_present_preprocess".into(), Value::Str(format!("@{window}")));
        self.vars.insert("$SYSTEM_present_text".into(), Value::Str(format!("@{name}")));
        if let Some(pending) = self.pending_enter.take() {
            if name.rsplit('/').next() == Some(pending.as_str()) {
                self.show_text(&name);
            } else {
                self.note("Request Enter for another text than the next");
            }
        }
    }

    /// Splits a text block into pages: a blank line ends a page, `<k>`
    /// waits mid-page. Lines holding only comments, markers, tags or code
    /// aren't lines of the page.
    pub(super) fn pages(&mut self, t: &'a Text) -> Vec<Page<'a>> {
        let mut pages = Vec::new();
        let mut page = Page::default();
        let mut line = String::new();
        // The line had something other than text (so it isn't blank).
        let mut other = false;
        let mut ruby: Option<(usize, String)> = None;
        // The line ended in `<?>`: the next text line goes on in it.
        let mut joined = false;
        // An `<FW img=''>` holds until the next code (which sets the face).
        let mut face_off = false;
        let page_len = |page: &Page<'a>, line: &str| {
            let n = page.text.chars().count();
            n + usize::from(n > 0) + line.chars().count()
        };
        let end_line = |page: &mut Page<'a>, line: &mut String| {
            if !page.text.is_empty() {
                page.text.push('\n');
            }
            page.text.push_str(&std::mem::take(line));
            // A `<center>` the line doesn't close ends with it.
            let n = page.text.chars().count();
            for s in page.styles.iter_mut().filter(|s| s.centred && s.start + s.len == usize::MAX) {
                s.len = n - s.start;
            }
        };
        for item in &t.items {
            match item {
                Inline::Marker(_) | Inline::Comment(_) => other = true,
                // `<?>` (not parsed as a tag): the text goes on after the
                // code that follows, which runs before the page anyway, on
                // the same line (a typewriter writing one character at a
                // time, `ma01_026`).
                Inline::Str(s) => {
                    page.face_off |= face_off;
                    joined = s.trim_end().ends_with("<?>");
                    line.push_str(&s.replace("<?>", ""));
                }
                Inline::Code(stmts) => {
                    // Code after a `<k>` runs after that click: end the page
                    // there and go on below it.
                    let mut text = page.text.clone();
                    if !line.is_empty() {
                        text = if text.is_empty() { line.clone() } else { format!("{text}\n{line}") };
                    }
                    if let Some(before) = text.strip_suffix(CLICK_WAIT_MARK) {
                        page.text = before.to_string();
                        line.clear();
                        page.continues = true;
                        pages.push(std::mem::take(&mut page));
                    } else if !page.text.is_empty() || !line.is_empty() {
                        // Code inside the page runs where it stands: the
                        // text before it shows, then the page goes on, on
                        // the same line when the code is inside one (or
                        // after a `<?>`).
                        // shortcut: a ruby open across the code is lost; none is.
                        let n = text.chars().count();
                        let mut open = Vec::new();
                        for s in page.styles.iter_mut().filter(|s| s.start + s.len == usize::MAX) {
                            s.len = n - s.start;
                            open.push(TextStyle { start: 0, len: usize::MAX, ..*s });
                        }
                        page.text = if line.is_empty() { text + "\n" } else { text };
                        page.text.push(conv_core::ir::GOES_ON_MARK);
                        line.clear();
                        pages.push(std::mem::take(&mut page));
                        (page.goes_on, page.styles) = (true, open);
                    }
                    page.code.extend(stmts.iter());
                    face_off = false;
                    other = true;
                    joined = false;
                }
                Inline::Tag(tag) => {
                    other = true;
                    match (tag.name.to_ascii_lowercase().as_str(), tag.close) {
                        ("voice", false) => {
                            // The voice settings go by class (村正's voices are 村正さん).
                            let name = tag.attr("class").filter(|c| !c.is_empty()).or(tag.attr("name")).unwrap_or("").to_string();
                            let src = tag.attr("src").unwrap_or("").to_string();
                            page.voice = Some((name, src));
                        }
                        ("ruby", false) => ruby = Some((page_len(&page, &line), tag.attr("text").unwrap_or("").to_string())),
                        ("ruby", true) => {
                            if let Some((start, text)) = ruby.take() {
                                let len = page_len(&page, &line) - start;
                                page.ruby.push(Ruby { start, len, text });
                            }
                        }
                        ("i", false) | ("center", false) => {
                            let centred = tag.name.eq_ignore_ascii_case("center");
                            let start = page_len(&page, &line);
                            page.styles.push(TextStyle { start, len: usize::MAX - start, italic: !centred, centred, ..Default::default() });
                        }
                        ("i", true) | ("center", true) => {
                            let centred = tag.name.eq_ignore_ascii_case("center");
                            let end = page_len(&page, &line);
                            if let Some(s) = page.styles.iter_mut().rev().find(|s| s.centred == centred && s.start + s.len == usize::MAX) {
                                s.len = end - s.start;
                            }
                        }
                        ("br", _) => end_line(&mut page, &mut line),
                        // A click wait where it stands, mid-line or not.
                        ("k", _) => line.push(CLICK_WAIT_MARK),
                        // Red and larger words in the story (incolor, size).
                        ("font", false) => {
                            let start = page_len(&page, &line);
                            let colour = tag.attr("incolor").and_then(|c| color(&Value::Str(c.to_string())));
                            let size = tag.attr("size").and_then(|s| s.parse::<i64>().ok());
                            let size_percent = size.map(|s| (s * 100 / self.font_size.max(1)) as u16);
                            page.styles.push(TextStyle { start, len: usize::MAX - start, colour, size_percent, ..Default::default() });
                        }
                        ("font", true) => {
                            let end = page_len(&page, &line);
                            if let Some(s) = page.styles.iter_mut().rev().find(|s| !s.italic && !s.centred && s.start + s.len == usize::MAX) {
                                s.len = end - s.start;
                            }
                        }
                        ("pre", _) => {}
                        // The English port's face window switched mid-page
                        // (`SwitchFw`): `img=''` takes the face away, and the
                        // text goes on in a new page in the box without one;
                        // a picture brings the face back.
                        // shortcut: the picture is taken to be the face the NSS
                        // has set (md05_033, the one use); read it if another
                        // translation uses another.
                        ("fw", false) => {
                            if !line.is_empty() {
                                end_line(&mut page, &mut line);
                            }
                            if !page.text.is_empty() || page.goes_on {
                                pages.push(std::mem::take(&mut page));
                            }
                            face_off = tag.attr("img").is_none_or(str::is_empty);
                        }
                        // A variable's value, when it is known here; one a
                        // runtime flag holds as a mark (`show_text` gives the
                        // page once for each of its texts). The English
                        // port writes `<VALUE='材料名前'>`.
                        (n, false) if n == "value" || n.starts_with("value=") => {
                            let name = tag.attr("name").map(str::to_string).or_else(|| tag.name.split_once('=').map(|(_, v)| format!("${}", v.trim_matches(['\'', '"']))));
                            match name.and_then(|n| self.vars.get(&n)).cloned() {
                                Some(v @ (Value::Str(_) | Value::Int(_))) => line.push_str(&v.string()),
                                Some(Value::Runtime(var)) if self.flag_texts.get(&var).is_some_and(|t| !t.is_empty()) => {
                                    line.push_str(&format!("{}{}{}{}", RUNTIME_MARK.0, if var.global { "g" } else { "" }, var.index, RUNTIME_MARK.1));
                                }
                                _ => self.note("text tag <value> of a variable known only when the game runs"),
                            }
                        }
                        // The English port's waits (function_textbox.nut): WaitKey and
                        // WaitVoice mid-text.
                        (n, false) if n.starts_with("wait=") || n.starts_with("waitvoice=") => {
                            let (mark, ms) = match n.strip_prefix("waitvoice=") {
                                Some(ms) => (conv_core::ir::VOICE_WAIT_MARK, ms),
                                None => (conv_core::ir::TIMED_WAIT_MARK, &n["wait=".len()..]),
                            };
                            match ms.trim().parse::<u32>() {
                                Ok(ms) => line.extend([mark.to_string(), ms.to_string(), conv_core::ir::WAIT_END_MARK.to_string()]),
                                Err(_) => self.note(format!("text tag <{n}>")),
                            }
                        }
                        // The English port's coloured ranges (SetCharacterColor)
                        // and small type (CTextEnglish_layout).
                        ("red" | "red2" | "red3" | "yellow" | "small", false) => {
                            let start = page_len(&page, &line);
                            let (colour, size_percent) = match tag.name.to_ascii_lowercase().as_str() {
                                "red" => (Some(0xFF0000), None),
                                "red2" => (Some(0xCC0000), None),
                                "red3" => (Some(0x990000), None),
                                "yellow" => (Some(0xFFFF00), None),
                                // src: exe 004a82e7 (CTextEnglish_layout: SMALL draws with the text's ruby
                                // font, setRubyFont), function_textbox.nut (Droid Serif size 20, ruby_size 10)
                                _ => (None, Some(50)),
                            };
                            page.styles.push(TextStyle { start, len: usize::MAX - start, colour, size_percent, ..Default::default() });
                        }
                        ("red" | "red2" | "red3" | "yellow" | "small", true) => {
                            let end = page_len(&page, &line);
                            if let Some(s) = page.styles.iter_mut().rev().find(|s| !s.italic && !s.centred && s.start + s.len == usize::MAX) {
                                s.len = end - s.start;
                            }
                        }
                        // The English port's end of a voice's line
                        // (function_textbox.nut `Print`: a `voice` tag without
                        // parameters empties voice_name/class/file for the
                        // text after it); the voice plays on: nothing to do.
                        ("voice", true) => {}
                        (other, _) => self.note(format!("text tag <{other}>")),
                    }
                }
                Inline::Newline => {
                    if joined && !line.is_empty() {
                        continue;
                    }
                    if !line.is_empty() {
                        end_line(&mut page, &mut line);
                    // (A line of only `<?>`, as the English leaves a
                    // typewriter's letters, isn't blank.)
                    } else if !other && !joined && (!page.text.is_empty() || page.goes_on) {
                        pages.push(std::mem::take(&mut page));
                    }
                    other = false;
                }
            }
        }
        if !line.is_empty() {
            end_line(&mut page, &mut line);
        }
        if !page.text.is_empty() || !page.code.is_empty() || page.goes_on {
            pages.push(page);
        }
        pages
    }

    /// `Request(text, Enter)`: shows the text object's pages.
    pub(super) fn show_text(&mut self, name: &str) {
        let Some(Object { kind: Kind::Text(t), x, y, .. }) = self.objects.get(name) else { return };
        let (t, x, y) = (t.clone(), *x, *y);
        self.sync(0);
        // the text tags dropped, on the block's first page shown (`metrics.json`'s "text")
        // shortcut: a block's tag notes all go on its first page; per page when a page's credit matters
        self.metrics.begin("text");
        let pages = self.pages(t.block);
        let mut notes = self.metrics.pop().map(|(_, r)| r).unwrap_or_default();
        for page in &pages {
            // (the page's code is its <PRE> file's, wherever it is shown)
            // shortcut: statements nested in it (an `if`'s) are marked with the showing file
            for &s in &page.code {
                self.code_file = t.at.map(|a| a.0);
                self.exec(s);
            }
            self.flush_sounds();
            if page.text.is_empty() && !page.goes_on {
                continue;
            }
            self.metrics.page(std::mem::take(&mut notes));
            self.sync(0);
            // Code before the page may have changed the face window.
            let mut window = self.text_window(&t, x, y, page.face_off);
            if page.goes_on && self.window.as_ref() != Some(&window) {
                // A new window would clear the page: only its picture
                // changes (a face changed mid-page); one that moved stays
                // as the page began.
                match self.window.clone() {
                    Some(w) if TextWindow { image: window.image.clone(), ..w.clone() } == window => self.ops.push(Op::WindowImage(window.image.clone())),
                    Some(w) => {
                        self.note("text window moved inside a page: kept");
                        window = w;
                    }
                    None => {}
                }
            } else if self.texts_shown == 0 {
                self.first_window = Some(window.clone());
            } else if self.window.as_ref() != Some(&window) {
                self.ops.push(Op::SetWindow(window.clone()));
            }
            self.window = Some(window);
            if let Some((ms, waited)) = self.window_fade_in.take() {
                self.ops.push(Op::WindowFade { alpha: 256, ms, from: Some(0), delay: 0, show: true });
                if waited {
                    self.ops.push(Op::Wait { ms });
                }
            }
            // 続・殺戮のジャンゴ's src leaves out the voice folder
            // (`dja002/0100010ri`; system.ini: `path = voice/`).
            let voice = page.voice.as_ref().and_then(|(_, src)| self.assets.sound(src).or_else(|| self.assets.sound(&format!("voice/{src}"))));
            let speaker = page.voice.as_ref().map(|(n, _)| n.clone()).filter(|n| !n.is_empty());
            let voice = voice.map(|file| {
                let class = speaker.clone().unwrap_or_default();
                let character = match self.voice_classes.iter().position(|c| *c == class) {
                    Some(i) => i,
                    None => {
                        self.voice_classes.push(class);
                        self.voice_classes.len() - 1
                    }
                } as u32;
                Voice { file, character }
            });
            // The face window or name plate (part of the window image)
            // shows who speaks; the name is kept for the voice switches.
            // Vertical text is full-width (CONscripter also exits on a
            // half-width character in vertical mode).
            let vertical = self.window.as_ref().is_some_and(|w| w.vertical);
            let shown = |t: &str| if vertical { full_width(t) } else { t.to_string() };
            let mut ruby = page.ruby.clone();
            if vertical {
                for r in &mut ruby {
                    r.text = full_width(&r.text);
                }
            }
            let page_op = |text: String| Op::Text {
                speaker: None,
                text: shown(&text),
                voice: voice.clone(),
                links: Vec::new(),
                ruby: ruby.clone(),
                styles: page.styles.clone(),
                hang: None,
                continues: page.continues,
            };
            // A runtime flag's text in it: the page once for each text the
            // flag can hold, behind a test of its code, and without it.
            // shortcut: styles after the mark keep their places; fine for
            // the pages that have one (mc02_031: none).
            // Only spaces left after the code (before its line's end): none.
            let mut text = page.text.clone();
            if page.goes_on && text.trim_end_matches(conv_core::ir::GOES_ON_MARK).trim().is_empty() {
                text.retain(|c| c == conv_core::ir::GOES_ON_MARK);
            }
            // The page comes from its <PRE> block, wherever it is shown.
            if let Some(at) = t.at {
                self.running.push(at);
                self.mark();
            }
            match runtime_text(&text).and_then(|(var, before, after)| Some((var, before, after, self.flag_texts.get(&var)?.clone()))) {
                Some((var, before, after, texts)) => {
                    // (Named by the file and page, so that no other label's
                    // number moves.)
                    let n = format!("{}_value{}", file_label(self.file), self.texts_shown);
                    let end = format!("{n}_end");
                    for (i, (code, s)) in texts.iter().enumerate() {
                        let next = format!("{n}_{i}");
                        self.ops.push(Op::If { var, cmp: conv_core::ir::Compare::Ne, value: Operand::Const(*code), label: next.clone() });
                        self.ops.push(page_op(format!("{before}{s}{after}")));
                        self.ops.push(Op::Jump(end.clone()));
                        self.ops.push(Op::Label(next));
                    }
                    self.ops.push(page_op(format!("{before}{after}")));
                    self.ops.push(Op::Label(end));
                }
                None => self.ops.push(page_op(text)),
            }
            if t.at.is_some() {
                self.running.pop();
                self.mark();
            }
            self.texts_shown += 1;
        }
        // (no page shown: the notes stay the showing call's, as before)
        notes.iter().for_each(|r| self.metrics.note(r));
        self.window_visible = true;
    }

    /// The text window for a text object at (x, y): its `LoadText` box,
    /// turned vertical by a 90° `Rotate`, over the `Hideable` picture
    /// that is visible behind it. `face_off`: the faces (`Fw…`) and name
    /// plates (`Nw…`) left out, and a box with a face window drawn as its
    /// pair without one ([`Game::box_text`]).
    pub(super) fn text_window(&mut self, t: &TextObject<'a>, x: i64, y: i64, face_off: bool) -> TextWindow {
        let (w, h, letter, line, font) = t.layout.unwrap_or((550, 150, 0, 29, self.font_size));
        let columns = (w / (font + letter).max(1)).max(1) as u32;
        let vertical = t.rotation.rem_euclid(360) == 90;
        // Across: as many lines as end inside the box (box02: 130 px at a
        // 27 px pitch holds five 18 px lines, the last ending at 126).
        let rows = if vertical { h / line.max(1) } else { (h - font) / line.max(1) + 1 }.max(1) as u32;
        let (mut text_x, mut text_y) = if vertical { (x - rows as i64 * line, y) } else { (x, y + self.game.text_drop) };
        let (mut columns, mut rows, mut line, mut vertical) = (columns, rows, line, vertical);
        if let (true, Some((w, h, pitch))) = (vertical, self.game.horizontal_box) {
            (columns, rows, line, vertical) = ((w / font.max(1)).max(1) as u32, ((h - font) / pitch.max(1) + 1).max(1) as u32, pitch, false);
        }
        // The window's pictures (box, face window, name plate), bottom
        // first, drawn into one window image.
        // (priority, creation order, (picture, x, y, opacity))
        type Part = (i64, u64, (String, i64, i64, i64));
        // With `face_off`, each box with a face and the box drawn for it.
        let plain_for: Vec<(&str, &str)> = if face_off { self.game.box_text.chunks(2).filter_map(|p| Some((p.first()?.0, p.get(1)?.0))).collect() } else { Vec::new() };
        let is_face = |name: &str| face_off && ["Fw", "Nw", "Face_", "@backFw"].iter().any(|p| name.starts_with(p));
        let mut parts: Vec<Part> = self
            .objects
            .iter()
            .filter(|(name, o)| o.hideable && o.opacity > 0 && !is_face(name))
            .filter_map(|(name, o)| {
                let o2 = match plain_for.iter().find(|p| p.0 == name.as_str()) {
                    Some(p) => self.objects.get(p.1)?,
                    None => o,
                };
                match &o2.kind {
                    Kind::Image { image } => Some((o.priority, o.seq, (image.clone(), o2.x, o2.y, o.opacity))),
                    _ => None,
                }
            })
            .collect();
        parts.sort_by_key(|p| (p.0, p.1));
        let parts: Vec<_> = parts.into_iter().map(|p| p.2).collect();
        let up = |name: &str| self.objects.get(name).is_some_and(|o| o.hideable && o.opacity > 0);
        let shown = |name: &str| match plain_for.iter().find(|p| p.0 == name || p.1 == name) {
            Some(p) if p.0 == name => false,
            Some(p) => up(name) || up(p.0),
            None => up(name),
        };
        let mut box_icon = None;
        // `boxLT` and `boxLL` are box `L`, `boxCTH` box `CH` (`T`: with a face).
        let stem = |b: &str| {
            let s = b.trim_start_matches("box").replace('T', "");
            let mut c = s.chars();
            match (c.next(), c.next(), c.next()) {
                (Some(a), Some(b), None) if a == b => a.to_string(),
                _ => s,
            }
        };
        let boxes = || self.game.box_text.iter().filter(|b| shown(b.0));
        let named = boxes().find(|b| self.text_box.as_deref() == Some(stem(b.0).as_str()));
        if let Some(&(_, x, y, w, icon)) = named.or_else(|| boxes().next()) {
            (text_x, text_y, columns, box_icon) = (x, y + self.game.text_drop, (w / font.max(1)).max(1) as u32, Some(icon));
            // The port's boxes across have its pitch too (`rowSpace` 1 under
            // the font's height, in every `SetTextBoxVal`).
            if let (false, Some((_, _, pitch))) = (t.rotation.rem_euclid(360) == 90, self.game.horizontal_box) {
                (rows, line) = (((h - font) / pitch.max(1) + 1).max(1) as u32, pitch);
            }
        }
        let (image, wx, wy) = match self.assets.composite(&parts) {
            Some(c) => c,
            None => (self.assets.transparent(), 0, 0),
        };
        // The click-wait icon's place, set by each TypeBegin helper.
        let icon = |k: &str| match self.vars.get(k) {
            Some(Value::Int(v)) => Some(*v as i32),
            _ => None,
        };
        let wait_icon_at = box_icon.or_else(|| icon("$SYSTEM_position_x_text_icon").zip(icon("$SYSTEM_position_y_text_icon")));
        TextWindow {
            wait_icon_at,
            image,
            x: wx as i32,
            y: wy as i32,
            text_x: text_x as i32,
            text_y: text_y as i32,
            columns,
            rows,
            font_size: font as u32,
            line_gap: (line - font).max(0) as u32,
            letter_spacing: letter.max(0) as u32, // src: exe FUN_00484d30 (Muramasa), FUN_00462de0 (Django): advance = the glyph's GDI gmCellIncX (the font size for full-width letters) + LoadText's letter space
            vertical,
            name_plate: None,
        }
    }
}

/// Half-width katakana and signs U+FF61-FF9F as full-width ones.
pub(super) const HALF_KANA: &str = "。「」、・ヲァィゥェォャュョッーアイウエオカキクケコサシスセソタチツテトナニヌネノハヒフヘホマミムメモヤユヨラリルレロワン゛゜";

/// ASCII letters, digits and signs, and half-width katakana, as their
/// full-width forms, one character for one (ruby positions stay valid).
pub(super) fn full_width(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            ' ' => '　',
            '!'..='~' => char::from_u32(c as u32 - 0x21 + 0xFF01).unwrap_or(c),
            '\u{FF61}'..='\u{FF9F}' => HALF_KANA.chars().nth(c as usize - 0xFF61).unwrap_or(c),
            _ => c,
        })
        .collect()
}

/// Marks a choice's runtime text inside a string (`"『" + $テキストデータ１ +
/// "』"`): private-use characters around the flag's index, read back by
/// `SetBacklog` ([`runtime_text`]).
pub(super) const RUNTIME_MARK: (char, char) = ('\u{E000}', '\u{E001}');

/// `text`'s first runtime mark: its flag, the text before and after it
/// (other marks dropped).
pub(super) fn runtime_text(text: &str) -> Option<(conv_core::ir::Var, String, String)> {
    let (before, rest) = text.split_once(RUNTIME_MARK.0)?;
    let (index, after) = rest.split_once(RUNTIME_MARK.1)?;
    let strip = |s: &str| {
        let mut out = String::new();
        let mut inside = false;
        for c in s.chars() {
            match c {
                c if c == RUNTIME_MARK.0 => inside = true,
                c if c == RUNTIME_MARK.1 => inside = false,
                c if !inside => out.push(c),
                _ => {}
            }
        }
        out
    };
    let (global, index) = match index.strip_prefix('g') {
        Some(i) => (true, i),
        None => (false, index),
    };
    Some((conv_core::ir::Var { global, index: index.parse().ok()? }, strip(before), strip(after)))
}
