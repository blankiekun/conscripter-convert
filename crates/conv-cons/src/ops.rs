//! The script's body: each op as NScripter lines ([`Emitter::op`]), with
//! the state the ops share while the script is written in order.

use super::*;
use crate::strtab::{CHOICE, LOG, NAME, TEXT, WINDOW};

/// What [`emit`] keeps while it writes the ops in order: what the script
/// has (worked out first) and what the ops before left behind.
pub(crate) struct Emitter<'a> {
    pub script: &'a Script,
    pub out: String,
    pub effects: BTreeMap<(u32, Effect), String>,
    pub has_ruby: bool,
    pub has_overlay: bool,
    pub has_noise: bool,
    pub has_chapters: bool,
    pub has_animation: bool,
    pub clear_fade: String,
    pub root_returns: usize,
    pub link_depth: Option<&'static str>,
    pub has_replay: bool,
    pub has_vertical: bool,
    pub restore_overlay: String,
    pub border: u32,
    pub speed: &'a str,
    pub voice_switches: bool,
    pub duck: bool,
    pub back_to_choice: bool,
    pub link_targets: Vec<&'a str>,
    pub zoomed_layers: std::collections::HashSet<u32>,
    pub root_resumes: usize,
    pub puzzles: usize,
    pub button_screens: usize,
    pub plate_choices: usize,
    pub picture_screens: usize,
    pub se: SeChannels,
    pub se_not_skipped: bool,
    pub noise_fade_in: Option<u32>,
    /// Sprites (slot → ms) whose fade in started with no time passed since:
    /// a motion of theirs takes the fade into its keys.
    pub fading: BTreeMap<u32, u32>,
    pub move_ease: Option<(u8, u32)>,
    /// The next timed move isn't waited for ([`Op::MoveNoWait`]).
    pub move_nowait: bool,
    /// The next timed move's bow ([`Op::MoveArc`]), 0 for none.
    pub move_arc: Option<i32>,
    /// The eased curves used, (ease, exponent), and their `spcurve` ids: tables at `*start`.
    pub eased: BTreeMap<(u8, u32), u32>,
    pub needs_click_wait: bool,
    pub plate_visible: bool,
    pub link_pages: usize,
    pub transition: Option<Vec<String>>,
    pub window: Option<TextWindow>,
    pub page_rows: usize,
    pub page_tag: Option<String>,
    /// The last page stopped for code inside it ([`conv_core::ir::GOES_ON_MARK`]): the
    /// next text goes on in it.
    pub text_goes_on: bool,
    /// Where text goes: inline or in the text table.
    pub strings: crate::strtab::Strings,
}

impl<'a> Emitter<'a> {
    /// Writes op `i` of the script.
    pub(crate) fn op(&mut self, i: usize, op: &'a Op) {
        if !matches!(op, Op::Sprites { .. } | Op::SpritesFading { .. } | Op::SpriteMotion { .. } | Op::SpriteMotion2 { .. } | Op::SpriteMotionAdd { .. } | Op::SpriteMove { .. } | Op::MoveEase { .. } | Op::MoveNoWait | Op::MoveArc { .. }) {
            self.fading.clear();
        }
        let &mut Emitter { has_ruby, has_overlay, has_noise, has_chapters, has_animation, root_returns, has_replay, has_vertical, border, voice_switches, duck, back_to_choice, speed, link_depth, script, ref clear_fade, ref restore_overlay, ref effects, ref link_targets, ref zoomed_layers, ref mut out, ref mut root_resumes, ref mut puzzles, ref mut button_screens, ref mut plate_choices, ref mut picture_screens, ref mut se, ref mut se_not_skipped, ref mut noise_fade_in, ref mut move_ease, ref mut move_nowait, ref mut move_arc, ref mut eased, ref mut needs_click_wait, ref mut plate_visible, ref mut link_pages, ref mut transition, ref mut window, ref mut page_rows, ref mut page_tag, ref mut text_goes_on, ref mut strings, fading: ref mut fading_slots } = self;
        let effect = |ms: u32| if ms == 0 { "1".to_string() } else { effects[&(ms, Effect::Fade)].clone() };
        let visited = |target: &str| format!("?{LINK_VISITED}[{}]", link_targets.iter().position(|&t| t == target).unwrap());
    if let Op::Text { .. } = op {
        for line in transition.take().unwrap_or_default() {
            writeln!(out, "{line}").unwrap();
        }
    }
    match op {
        Op::Label(name) => writeln!(out, "*{}", label(name)).unwrap(),
        Op::Jump(name) => writeln!(out, "goto *{}", label(name)).unwrap(),
        Op::Gosub(name) => writeln!(out, "gosub *{}", label(name)).unwrap(),
        Op::Return => writeln!(out, "return").unwrap(),
        Op::Arith { var, op, value } => {
            let value = operand(*value, border);
            let target = var_name(*var, border);
            let cmd = match op {
                ArithOp::Set => "mov",
                ArithOp::Add => "add",
                ArithOp::Sub => "sub",
                ArithOp::Mul => "mul",
                ArithOp::Div => "div",
                // CONscripter's bitand/bitor (NScripter has no bitwise operators).
                ArithOp::BitAnd | ArithOp::BitOr => {
                    let cmd = if *op == ArithOp::BitAnd { "bitand" } else { "bitor" };
                    writeln!(out, "{cmd} {target},{target},{value}").unwrap();
                    return;
                }
            };
            writeln!(out, "{cmd} {target},{value}").unwrap();
        }
        Op::Random { var, below } => writeln!(out, "rnd {},{below}", var_name(*var, border)).unwrap(),
        Op::If { var, cmp, value, label: target } => {
            writeln!(out, "if {} {} {} goto *{}", var_name(*var, border), cmp.operator(), operand(*value, border), label(target)).unwrap()
        }
        Op::Background { image, fade_ms } => {
            // NScripter's bg keeps lsp sprites; the game removes characters
            // when the background or CG changes.
            writeln!(out, "csp -1{clear_fade}").unwrap();
            out.push_str(restore_overlay);
            // Effect 0 = update the buffer only (shown by the transition).
            let e = if transition.is_some() { "0".to_string() } else { effect(*fade_ms) };
            writeln!(out, "bg \"{}\",{e}", path(image)).unwrap();
            *plate_visible = false;
        }
        Op::Sprites { changes, fade_ms } | Op::SpritesFading { changes, fade_ms } => {
            // SpritesFading: an spmotion fade, in (loop 0) and out (loop 3).
            let fading = matches!(op, Op::SpritesFading { .. }) && transition.is_none() && *fade_ms > 0;
            for c in changes {
                let slot = SPRITE_BASE + c.layer;
                match &c.sprite {
                    Some(sp) => {
                        if !c.keep_filters {
                            // Only reloads the old image when it has filters.
                            writeln!(out, "spfilter {slot},0").unwrap();
                        }
                        writeln!(out, "{}", lsp(slot, sp)).unwrap();
                        if zoomed_layers.contains(&c.layer) {
                            writeln!(out, "spzoom {slot},100,100").unwrap();
                        }
                        if fading {
                            writeln!(out, "{}", motion::fade_in(slot, *fade_ms)).unwrap();
                            fading_slots.insert(slot, *fade_ms);
                        }
                    }
                    None if fading => {
                        writeln!(out, "{}", motion::fade_out(slot, *fade_ms)).unwrap();
                        fading_slots.remove(&slot);
                    }
                    None => {
                        writeln!(out, "csp {slot}").unwrap();
                        fading_slots.remove(&slot);
                    }
                }
            }
            if transition.is_none() {
                writeln!(out, "print {}", if fading { "1".to_string() } else { effect(*fade_ms) }).unwrap();
                if !fading && *fade_ms > 0 {
                    fading_slots.clear(); // the effect takes time
                }
            }
        }
        Op::SpriteFilters { layer, filters } => {
            let slot = SPRITE_BASE + layer;
            writeln!(out, "spfilter {slot},0").unwrap();
            for f in filters {
                let args = [&f.kind, &f.blend, &f.strength, &f.params[0], &f.params[1], &f.params[2]].map(|o| operand(*o, border));
                match f.kind {
                    Operand::Const(0) => {}
                    Operand::Const(_) => writeln!(out, "spfilter {slot},{}", args.join(",")).unwrap(),
                    // spfilter's kind 0 clears every filter; skip it instead.
                    Operand::Var(_) => writeln!(out, "if {} != 0 spfilter {slot},{}", args[0], args.join(",")).unwrap(),
                }
            }
            if transition.is_none() {
                writeln!(out, "print 1").unwrap();
            }
        }
        Op::SpriteOpacity { layer, opacity } => {
            // msp adds to the alpha and clamps it: down to 0, then up.
            let alpha = opacity.min(&256) * 255 / 256;
            match layer {
                Some(layer) => writeln!(out, "msp {0},0,0,-255:msp {0},0,0,{alpha}", SPRITE_BASE + layer).unwrap(),
                None => writeln!(out, "if {OVERLAY_SLOT} != 0 msp {OVERLAY_SLOT},0,0,-255:msp {OVERLAY_SLOT},0,0,{alpha}").unwrap(),
            }
            if transition.is_none() {
                writeln!(out, "print 1").unwrap();
            }
        }
        Op::SpriteFlashing { layer, keys, times } => {
            let slot = SPRITE_BASE + layer;
            writeln!(out, "spblend {slot},1:spmotion {slot},{},{}", crate::motion::alpha_loop(*times), crate::motion::alpha_keys(keys)).unwrap();
            if transition.is_none() {
                writeln!(out, "print 1").unwrap();
            }
        }
        Op::StopAutoSkip => writeln!(out, "skipoff:autooff").unwrap(),
        Op::BacklogLine { text, voice, character } => match (voice, character) {
            (Some(v), Some(c)) if voice_switches && crate::line_log_screen::engine_log(script).is_some() => {
                writeln!(out, "logadd \"{}\",\"{}\",{c}", strings.string(LOG, escape(text)), path(v)).unwrap()
            }
            (Some(v), _) => writeln!(out, "logadd \"{}\",\"{}\"", strings.string(LOG, escape(text)), path(v)).unwrap(),
            (None, _) => writeln!(out, "logadd \"{}\"", strings.string(LOG, escape(text))).unwrap(),
        },
        Op::RuntimeBacklogLine { text, runtime } => {
            let s = crate::RUNTIME_LOG;
            writeln!(out, "mov {s},\"{}\"", strings.string(LOG, escape(text))).unwrap();
            for (value, t) in &runtime.texts {
                writeln!(out, "if {} == {value} mov {s},\"{}\"", var_name(runtime.var, border), strings.string(LOG, escape(t))).unwrap();
            }
            writeln!(out, "logadd {s}").unwrap();
        }
        Op::SpriteClip { layer, rect: (x, y, w, h) } => {
            writeln!(out, "spclip {},{x},{y},{w},{h}", SPRITE_BASE + layer).unwrap();
            if transition.is_none() {
                writeln!(out, "print 1").unwrap();
            }
        }
        Op::SpriteClipFeather { layer, rect: (x, y, w, h), feather, colour } => {
            match colour {
                Some(c) => writeln!(out, "spclip {},{x},{y},{w},{h},{feather},\"#{c:06x}\"", SPRITE_BASE + layer).unwrap(),
                None => writeln!(out, "spclip {},{x},{y},{w},{h},{feather}", SPRITE_BASE + layer).unwrap(),
            }
            if transition.is_none() {
                writeln!(out, "print 1").unwrap();
            }
        }
        Op::Language(code) => writeln!(out, "language \"{code}\"").unwrap(),
        Op::SaveState { slot } => writeln!(out, "savegame {slot}").unwrap(),
        Op::LoadState { slot } => writeln!(out, "loadgame {slot}").unwrap(),
        Op::SeStopEvery => {
            se.stop_all();
            se.next = 0;
            writeln!(out, "{}", crate::SE_CHANNELS.chain(crate::SE_OVERFLOW).map(|c| format!("dwavestop {c}")).collect::<Vec<_>>().join(":")).unwrap();
        }
        Op::SpriteMask { layer, image } => {
            writeln!(out, "spmask {},\"{}\"", SPRITE_BASE + layer, path(image)).unwrap();
            if transition.is_none() {
                writeln!(out, "print 1").unwrap();
            }
        }
        Op::SpriteFadeTo { layer, mask, ms, from, to, softness, form, ease } => {
            let curve = ease.filter(|(e, _)| (1..=7).contains(e)).and_then(|(e, x)| crate::ease::curve_id(e, x, eased)).unwrap_or(0);
            let tail = match (*form, curve) {
                (0, 0) => String::new(),
                (f, 0) => format!(",{f}"),
                (f, c) => format!(",{f},{c}"),
            };
            writeln!(out, "spfadeto {},\"{}\",{ms},{from},{to},{softness}{tail}", SPRITE_BASE + layer, path(mask)).unwrap();
            if transition.is_none() {
                writeln!(out, "print 1").unwrap();
            }
        }
        Op::SpriteBlend { layer, mode } => {
            writeln!(out, "spblend {},{mode}", SPRITE_BASE + layer).unwrap();
            if transition.is_none() {
                writeln!(out, "print 1").unwrap();
            }
        }
        Op::SpriteWave { layer, period: 0, .. } => writeln!(out, "spwave {}", SPRITE_BASE + layer).unwrap(),
        Op::SpriteWave { layer, ms, period, from, to, ease } => {
            let c = ease::curve(*ease, eased);
            writeln!(out, "spwave {},{ms},{period},{from},{to},{c}", SPRITE_BASE + layer).unwrap();
        }
        Op::SpriteTrail { layer, passes: 0, .. } => writeln!(out, "sptrail {}", SPRITE_BASE + layer).unwrap(),
        Op::SpriteTrail { layer, passes, alpha, lag, over } => {
            writeln!(out, "sptrail {},{passes},{alpha},{lag}{}", SPRITE_BASE + layer, if *over { ",1" } else { "" }).unwrap()
        }
        Op::SpriteOverText { layer } => {
            writeln!(out, "spovertext {}", SPRITE_BASE + layer).unwrap();
            if transition.is_none() {
                writeln!(out, "print 1").unwrap();
            }
        }
        Op::SpriteAngle { layer, degrees, zoom } => {
            writeln!(out, "spzoom {},{},{},{degrees}", SPRITE_BASE + layer, zoom.0, zoom.1).unwrap();
            if transition.is_none() {
                writeln!(out, "print 1").unwrap();
            }
        }
        Op::SpriteTurn { layer, image, from, to, degrees, ms } => {
            // The layer's sprite moves and turns by itself (spmotion's
            // angle turns counter-clockwise), then the layer is cleared.
            let (slot, ms) = (SPRITE_BASE + layer, (*ms).max(1));
            let ((x0, y0), (x1, y1)) = (*from, *to);
            writeln!(out, "lsp {slot},\":a;{}\",0,0:getspsize {slot},%1,%2:amsp {slot},{x0}-%1/2,{y0}-%2/2", path(image)).unwrap();
            writeln!(out, "spmotion {slot},0,0,0,0,100,100,0,256,0,{ms},{},{},100,100,{},256,0:spmotionwait {slot}", x1 - x0, y1 - y0, -degrees).unwrap();
            writeln!(out, "csp {slot}\nprint 1").unwrap();
        }
        Op::MoveNoWait => *move_nowait = true,
        Op::StartProcess { name, function, args } => {
            let args: String = args.iter().map(|a| format!(",{}", operand(*a, border))).collect();
            writeln!(out, "cons_start {},{}{args}", lua::quoted(name), lua::quoted(function)).unwrap();
        }
        Op::StopProcess { name } => writeln!(out, "cons_stop {}", lua::quoted(name)).unwrap(),
        Op::MoveArc { bow } => *move_arc = Some(*bow),
        Op::MotionWait { layer } => writeln!(out, "spmotionwait {}", SPRITE_BASE + layer).unwrap(),
        Op::MoveEase { ease, exponent } => *move_ease = Some((*ease, *exponent)).filter(|&(e, _)| (1..=7).contains(&e)),
        Op::SpriteMove { layer, x, y, ms } => {
            let slot = SPRITE_BASE + layer;
            let ease = move_ease.take();
            let wait = !std::mem::take(move_nowait);
            let arc = move_arc.take();
            if let Some(bow) = arc.filter(|_| *ms > 0 && transition.is_none()) {
                motion::arc_to(out, slot, *x, *y, *ms, None, ease, bow, fading_slots.remove(&slot), wait);
            } else if *ms > 0 && transition.is_none() {
                motion::move_to(out, slot, *x, *y, *ms, None, ease::curve(ease, eased), fading_slots.remove(&slot), wait);
            } else {
                writeln!(out, "getspsize {slot},%1,%2:amsp {slot},{x}-%1/2,{y}-%2/2").unwrap();
                if transition.is_none() {
                    writeln!(out, "print 1").unwrap();
                }
            }
        }
        // isskip: 1 while skipping (2 in auto mode).
        Op::IfSkipping { label: target } => {
            writeln!(out, "isskip {0}\nif {0} == 1 goto *{1}", menu::TMP, label(target)).unwrap();
            if self.script.click_skips_effects {
                writeln!(out, "getclickskip {0}\nif {0} == 1 goto *{1}", menu::TMP, label(target)).unwrap();
            }
        }
        Op::IfFastSkip { label: target } => {
            let fast = menu::FAST_SKIP;
            writeln!(out, "isskip {0}\nif {0} != 1 mov {fast},0\nif {fast} == 1 goto *{1}", menu::TMP, label(target)).unwrap();
        }
        Op::IfSwitch { switch, on, label: target } => {
            writeln!(out, "if {} == {} goto *{}", menu::switch_off(*switch), u8::from(!*on), label(target)).unwrap();
        }
        Op::SpriteMotion { layer, keys, looped } => motion::emit(out, *layer, keys, *looped, fading_slots.remove(&(SPRITE_BASE + layer))),
        Op::SpriteMotion2 { layer, keys, end } => {
            // Its opacity just set for the keys' fade (Op::SpriteOpacity):
            // not drawn at it first, the motion sets its first key's at once.
            // (Patched here: SpriteOpacity's emitter can't see the next op.)
            let set = format!("\nmsp {0},0,0,-255:msp {0},0,0,", SPRITE_BASE + layer);
            if out.ends_with("\nprint 1\n") && out[..out.len() - 9].rfind('\n').is_some_and(|i| out[i..].starts_with(&set)) {
                out.truncate(out.len() - 8);
            }
            let (ease, fade) = (move_ease.take(), fading_slots.remove(&(SPRITE_BASE + layer)));
            // shortcut: a fade folded into the keys splits them, so the motion goes straight (keys with their own curve keep it); ease each part when a game needs it.
            let curve = if fade.is_none() { ease::curve(ease, eased) } else { 0 };
            motion::emit2(out, *layer, keys, *end, fade, curve, eased);
        }
        Op::SpriteMotionAdd { layer, keys, end } => {
            let curve = ease::curve(move_ease.take(), eased);
            motion::emit_cmd(out, "spmotionadd", SPRITE_BASE + layer, keys, *end, None, curve, eased);
        }
        Op::MotionClock { layer, from } => writeln!(out, "spmotionclock {},{}", SPRITE_BASE + layer, SPRITE_BASE + from).unwrap(),
        Op::SpriteZoom { layer, x, y, from, to, ms } => {
            let ease = move_ease.take();
            let wait = !std::mem::take(move_nowait);
            let arc = move_arc.take();
            if let Some(bow) = arc.filter(|_| *ms > 0 && transition.is_none()) {
                motion::arc_to(out, SPRITE_BASE + layer, *x, *y, *ms, Some((*from, *to)), ease, bow, None, wait);
            } else {
                let curve = if *ms > 0 { ease::curve(ease, eased) } else { 0 };
                zoom::emit(out, *layer, *x, *y, *from, *to, *ms, transition.is_some(), curve, wait);
            }
        }
        Op::ScreenFilter { slot, filter, fade_ms } => {
            match filter {
                None if *slot == 0 => writeln!(out, "screenfilter 0").unwrap(),
                None => writeln!(out, "screenfilter {slot},0").unwrap(),
                Some(f) => {
                    let args = [&f.kind, &f.blend, &f.strength, &f.params[0], &f.params[1], &f.params[2]].map(|o| operand(*o, border));
                    writeln!(out, "screenfilter {slot},{}", args.join(",")).unwrap();
                }
            }
            if transition.is_none() {
                writeln!(out, "print {}", effect(*fade_ms)).unwrap();
            }
        }
        // screenfilter's z: applied right after the lowest sprite.
        Op::ScreenFilterUnderSprites { slot, filter: f } => {
            let args = [&f.kind, &f.blend, &f.strength, &f.params[0], &f.params[1], &f.params[2]].map(|o| operand(*o, border));
            writeln!(out, "screenfilter {slot},{},{MAX_SPRITE}", args.join(",")).unwrap();
            if transition.is_none() {
                writeln!(out, "print 1").unwrap();
            }
        }
        Op::HideTextWindow => writeln!(out, "textoff").unwrap(),
        Op::WindowFade { alpha, ms, from, delay, show } => {
            match from {
                Some(f) => writeln!(out, "windowfade {alpha},{ms},{f},{delay}").unwrap(),
                None => writeln!(out, "windowfade {alpha},{ms}").unwrap(),
            }
            if *show {
                writeln!(out, "textclear:texton").unwrap();
            }
        }
        Op::InstantText(on) => writeln!(out, "textspeed {}", if *on { "0" } else { speed }).unwrap(),
        Op::SetWindow(w) => {
            writeln!(out, "{}", strings.lines(WINDOW, &setwindow(w, speed, has_ruby, has_vertical, true, script))).unwrap();
            writeln!(out, "csp {NAME_PLATE_SPRITE}:csp {NAME_TEXT_SPRITE}").unwrap();
            *plate_visible = false;
            *page_rows = 0;
            *window = Some(w.clone());
        }
        Op::WindowImage(image) => {
            writeln!(out, "setwindow2 \":a;{}\"", path(image)).unwrap();
            if let Some(w) = window {
                w.image = image.clone();
            }
        }
        Op::ClearSprites => {
            writeln!(out, "csp -1{clear_fade}").unwrap();
            out.push_str(restore_overlay);
            *plate_visible = false;
            if transition.is_none() {
                writeln!(out, "print 1").unwrap();
            }
        }
        Op::Overlay { image, over_sprites } => {
            writeln!(out, "if {OVERLAY_SLOT} != 0 csp {OVERLAY_SLOT}").unwrap();
            match image {
                Some(image) => {
                    let slot = if *over_sprites { OVERLAY_SPRITE.to_string() } else { MAX_SPRITE.to_string() };
                    writeln!(out, "mov {OVERLAY_SLOT},{slot}:mov {OVERLAY_IMAGE},\":a;{}\"", path(image)).unwrap();
                    writeln!(out, "lsp {slot},{OVERLAY_IMAGE},0,0").unwrap();
                }
                None => writeln!(out, "mov {OVERLAY_SLOT},0").unwrap(),
            }
            if transition.is_none() {
                writeln!(out, "print 1").unwrap();
            }
        }
        // An spmotion fade.
        Op::NoiseFadeIn { ms } => *noise_fade_in = Some(*ms),
        // CONscripter's spz: drawn right over the overlay's sprite.
        Op::SpriteInFront { layer, front: true } => writeln!(out, "spz {},{OVERLAY_SPRITE}", SPRITE_BASE + layer).unwrap(),
        Op::SpriteInFront { layer, front: false } => writeln!(out, "spz {}", SPRITE_BASE + layer).unwrap(),
        Op::Noise(noise) => {
            writeln!(out, "csp {NOISE_SPRITE}").unwrap();
            match noise {
                Some(n) => {
                    let alpha = ((n.opacity * 255 + 128) / 256).clamp(1, 255);
                    let image = format!(":c/{},{},0;{}", n.frames, n.frame_ms, path(&n.image));
                    writeln!(out, "mov {NOISE_ALPHA},{alpha}:mov {NOISE_IMAGE},\"{image}\"").unwrap();
                    writeln!(out, "lsp {NOISE_SPRITE},{NOISE_IMAGE},0,0,{NOISE_ALPHA}").unwrap();
                    if let Some(ms) = noise_fade_in.take() {
                        writeln!(out, "{}", motion::fade_in(NOISE_SPRITE, ms)).unwrap();
                    }
                }
                None => writeln!(out, "mov {NOISE_ALPHA},0").unwrap(),
            }
            if transition.is_none() {
                writeln!(out, "print 1").unwrap();
            }
        }
        Op::Animation(animation) => {
            writeln!(out, "csp {ANIMATION_SPRITE}").unwrap();
            match animation {
                Some(a) => {
                    let alpha = ((a.opacity * 255 + 128) / 256).clamp(1, 255);
                    let ms: Vec<String> = a.frame_ms.iter().map(u32::to_string).collect();
                    // Mode 0 loops, 1 stops on the last frame.
                    let image =
                        format!(":c/{},<{}>,{};{}", a.frame_ms.len(), ms.join(","), u8::from(!a.looped), path(&a.image));
                    writeln!(out, "mov {ANIMATION_ALPHA},{alpha}:mov {ANIMATION_IMAGE},\"{image}\"").unwrap();
                    writeln!(out, "lsp {ANIMATION_SPRITE},{ANIMATION_IMAGE},0,0,{ANIMATION_ALPHA}").unwrap();
                }
                None => writeln!(out, "mov {ANIMATION_ALPHA},0").unwrap(),
            }
            if transition.is_none() {
                writeln!(out, "print 1").unwrap();
            }
        }
        Op::Particles(particles) => {
            particles::emit(out, particles);
            if transition.is_none() {
                writeln!(out, "print 1").unwrap();
            }
        }
        Op::ScreenEffect { slot, effect } => {
            match effect {
                None => writeln!(out, "screenshake {slot}").unwrap(),
                Some(ScreenEffect::Shake { time_ms, interval_ms, path, speed, range, end_ms, delay_ms }) => {
                    let [(x0, y0), (x1, y1)] = range;
                    writeln!(
                        out,
                        "screenshake {slot},{time_ms},{interval_ms},{},{},{},{x0},{y0},{x1},{y1}{}",
                        u8::from(*path),
                        speed.0,
                        speed.1,
                        match (*end_ms, *delay_ms) {
                            (0, 0) => String::new(),
                            (e, 0) => format!(",{e}"),
                            (e, d) => format!(",{e},0,{d}"),
                        }
                    )
                    .unwrap();
                }
                Some(ScreenEffect::Ripple { map, time_ms, interval_ms, opacity, amplitude, angle, angle_step, phase_step, multiply, phase }) => {
                    writeln!(
                        out,
                        "screenripple {slot},\"{}\",{time_ms},{interval_ms},{},{},{},{},{angle},{angle_step},{phase_step}{}",
                        path(map),
                        opacity.0,
                        opacity.1,
                        amplitude.0,
                        amplitude.1,
                        match (*multiply, *phase) {
                            (false, 0) => String::new(),
                            (m, 0) => format!(",{}", u8::from(m)),
                            (m, p) => format!(",{},{p}", u8::from(m)),
                        }
                    )
                    .unwrap();
                }
                Some(ScreenEffect::ShakeList { interval_ms, offsets, looped, delay_ms }) => {
                    let list: Vec<String> = offsets.iter().map(|(x, y)| format!("{x},{y}")).collect();
                    let tail = match (*looped, *delay_ms) {
                        (false, 0) => String::new(),
                        (l, 0) => format!(",{}", u8::from(l)),
                        (l, d) => format!(",{},0,{d}", u8::from(l)),
                    };
                    writeln!(out, "screenshake {slot},0,{interval_ms},3,\"{}\"{tail}", list.join(" ")).unwrap();
                }
                Some(ScreenEffect::Zoom { interval_ms, zoom, centre, zoom_ms, fade_ms, ease, power }) => {
                    let time = zoom_ms.max(fade_ms).max(&1);
                    writeln!(out, "screenzoom {slot},{time},{interval_ms},{zoom},{},{},{zoom_ms},{fade_ms},{ease},{power}", centre.0, centre.1).unwrap();
                }
                Some(ScreenEffect::Pulse { time_ms, interval_ms, zoom, opacity, colour, tint_opacity, tint }) => {
                    writeln!(
                        out,
                        "screenpulse {slot},{time_ms},{interval_ms},{},{},{opacity},{colour},{tint_opacity},{tint}",
                        zoom.0, zoom.1
                    )
                    .unwrap();
                }
                Some(ScreenEffect::Jolt { time_ms, interval_ms, range: [(x0, y0), (x1, y1)], ease, end_ms }) => {
                    let c = ease::curve(*ease, eased);
                    writeln!(out, "screenshake {slot},{time_ms},{interval_ms},2,0,0,{x0},{y0},{x1},{y1},{end_ms},{c}").unwrap();
                }
            }
            if transition.is_none() {
                writeln!(out, "print 1").unwrap();
            }
        }
        Op::BeginTransition { effect: name, fade_ms, mask, style } => {
            writeln!(out, "; transition: {name}").unwrap();
            let steps = transition_steps(*fade_ms, mask.as_deref(), *style);
            let mut lines: Vec<String> = steps.iter().map(|s| format!("print {}", effects[s])).collect();
            if lines.is_empty() {
                lines.push("print 1".into());
            } else if *style == TransitionStyle::ThroughWhite && lines.len() == 2 {
                // Old to white, then white to new: white over the new picture.
                let (w, h) = (script.width, script.height);
                lines.insert(0, format!("lsp2 {FLASH_SPRITE2},\":c;>{w},{h},#ffffff\",{},{},100,100,0,255", w / 2, h / 2));
                lines.insert(2, format!("csp2 {FLASH_SPRITE2}"));
            }
            *transition = Some(lines);
        }
        Op::TransitionEase { ease, exponent } if (1..=7).contains(ease) => {
            // Before the transition's first timed print.
            if let Some(i) = transition.as_ref().and_then(|l| l.iter().position(|l| l.starts_with("print ") && l != "print 1"))
                && let Some(id) = crate::ease::curve_id(*ease, *exponent, eased)
            {
                transition.as_mut().unwrap().insert(i, format!("effectcurve {id}"));
            }
        }
        Op::TransitionEase { .. } => {}
        Op::TransitionSoft { softness, reverse, form } => {
            // Before the transition's masked print (numbered or not).
            if let Some(i) = transition.as_ref().and_then(|l| l.iter().position(|l| l.starts_with("print ") && l != "print 1")) {
                let line = match (*reverse, *form) {
                    (r, 0) if !r => format!("effectsoft {softness}"),
                    (r, 0) => format!("effectsoft {softness},{}", r as u8),
                    (r, f) => format!("effectsoft {softness},{},{f}", r as u8),
                };
                transition.as_mut().unwrap().insert(i, line);
            }
        }
        Op::EndTransition => {
            for line in transition.take().unwrap_or_default() {
                writeln!(out, "{line}").unwrap();
            }
        }
        Op::Text { speaker, text, voice, links, ruby, styles, hang, continues } => {
            // Going on in a page that stopped for code: its speaker, plate
            // and ducking stay.
            let goes_on = text.ends_with(conv_core::ir::GOES_ON_MARK);
            let text = text.strip_suffix(conv_core::ir::GOES_ON_MARK).unwrap_or(text);
            let resumed = std::mem::replace(text_goes_on, goes_on);
            // Ducking: the music lowered while a line is voiced.
            if duck && !resumed {
                let off = menu::switch_off(Switch::BgmFullForVoice);
                let to = if voice.is_some() { "50,200" } else { "100,400" };
                writeln!(out, "if {off} == 1 chvolfade -1,{to}").unwrap();
            }
            if let Some(v) = voice {
                // The engine's backlog keeps a voice switched off, its button
                // hidden while the switch is off.
                if voice_switches && crate::line_log_screen::engine_log(script).is_some() {
                    writeln!(out, "logvoice \"{}\",{}", path(&v.file), v.character).unwrap();
                }
                if voice_switches {
                    write!(out, "if %{} == 0 ", menu::VOICE_OFF_BASE + v.character).unwrap();
                }
                writeln!(out, "dwave 0,\"{}\"", path(&v.file)).unwrap();
            }
            let clickable: Vec<&TextLink> = links.iter().filter(|l| l.target.is_some()).collect();
            let page = if clickable.is_empty() {
                None
            } else {
                // The page is drawn again after each link scene returns.
                *link_pages += 1;
                let page = format!("conv_link{link_pages}");
                writeln!(out, "*{page}").unwrap();
                *plate_visible = true; // unknown when entered again: always set it below
                Some(page)
            };
            let plate = window.as_ref().and_then(|w| w.name_plate.as_ref());
            let body = if script.single_byte_text { single_byte_text(text, styles) } else { with_markup(text, links, ruby, styles) }
                .replace(conv_core::ir::CLICK_WAIT_MARK, "@");
            let mut body = crate::wait_tags(&body);
            // Sprite changes only show after a print, so print when the
            // plate appears, changes or goes away.
            match (speaker, plate) {
                _ if resumed => {}
                (Some(name), Some(p)) => {
                    writeln!(out, "lsp {NAME_PLATE_SPRITE},\":a;{}\",{},{}", path(&p.image), p.x, p.y).unwrap();
                    writeln!(
                        out,
                        "strsp {NAME_TEXT_SPRITE},\"{}\",{},{},16,1,{},{},1,0,0,1",
                        strings.string(NAME, text_safe(name)), p.text_x, p.text_y, p.font_size, p.font_size
                    )
                    .unwrap();
                    writeln!(out, "print 1").unwrap();
                    *plate_visible = true;
                }
                (Some(name), None) => body = format!("【{}】{body}", text_safe(name)),
                (None, Some(_)) if *plate_visible => {
                    writeln!(out, "csp {NAME_PLATE_SPRITE}").unwrap();
                    writeln!(out, "csp {NAME_TEXT_SPRITE}").unwrap();
                    writeln!(out, "print 1").unwrap();
                    *plate_visible = false;
                }
                (None, _) => {}
            }
            // Each line of the page is its own source line; the last one
            // ends with the page wait. A line whose text starts with ASCII
            // would be read as a command (a colour tag is fine).
            let lines: Vec<String> = body
                .split('\n')
                .zip(text.split('\n'))
                .map(|(l, src)| match () {
                    _ if script.single_byte_text => l.to_string(),
                    _ => align_single_bytes(&if starts_ascii(src) { format!("　{l}") } else { l.to_string() }),
                })
                .collect();
            // A name the plate shows (else it's in the text already).
            let tag_name = speaker.as_deref().filter(|_| plate.is_some()).map(escape).unwrap_or_default();
            let tag_voice = voice.as_ref().map(|v| v.file.replace('\\', "/")).unwrap_or_default();
            if script.backlog.is_some() && page_tag.is_none() && !(tag_name.is_empty() && tag_voice.is_empty()) {
                *page_tag = Some(format!("[{}|{tag_voice}]", tag_name.replace('|', "｜")));
            }
            let lines: Vec<String> = match &page_tag {
                Some(tag) => lines.into_iter().map(|l| format!("{tag}{l}")).collect(),
                None => lines,
            };
            // A backquote starts a single-byte text line, whatever follows.
            let lines: Vec<String> = if script.single_byte_text { lines.into_iter().map(|l| format!("`{l}")).collect() } else { lines };
            if !(*continues || goes_on) || page.is_some() {
                *page_tag = None;
            }
            let rows_before = *page_rows;
            *page_rows = if *continues || goes_on { *page_rows + lines.len() } else { 0 };
            let indent = hang.map_or(0, |n| hang_indent(text, n));
            if indent > 0 {
                writeln!(out, "indent {indent}").unwrap();
            }
            let Some(page) = page else {
                // `@` waits and keeps the text (the next page continues
                // below); `\` waits and clears; a page that goes on after
                // code doesn't wait: `/` keeps the line open (after a
                // backquoted line, closed first), else the line ends.
                let mut lines = lines;
                let end = match () {
                    _ if goes_on && text.ends_with('\n') => {
                        lines.pop();
                        ""
                    }
                    _ if goes_on && script.single_byte_text => "`/",
                    _ if goes_on => "/",
                    _ if *continues => "@",
                    _ => "\\",
                };
                writeln!(out, "{}", strings.lines(TEXT, &format!("{}{end}", lines.join("\n")))).unwrap();
                if indent > 0 {
                    writeln!(out, "indent 0").unwrap();
                }
                return;
            };
            *page_rows = 0; // link pages always end with textclear
            // No page wait: the words become buttons and btnwait waits for
            // a click on one of them or anywhere else (= next page).
            writeln!(out, "{}", strings.lines(TEXT, &lines.join("\n"))).unwrap();
            if indent > 0 {
                writeln!(out, "indent 0").unwrap();
            }
            let clickable = &clickable[..clickable.len().min(LINK_UNDERLINE_SPRITES.len() / 2)];
            let window = window.as_ref();
            // Each word again as a sprite exactly over the text (the
            // button), and its underline, in the colours of its state.
            let mut underlines = String::new();
            for ((link, slot), line) in clickable.iter().zip((0..LINK_SPRITE_COUNT).map(link_sprite)).zip(LINK_UNDERLINE_SPRITES.step_by(2)) {
                let word: String = text.chars().skip(link.start).take(link.len).collect();
                let (x, y) = window.map_or((0, 0), |w| char_position(w, text, link.start));
                let y = y + window.map_or(0, |w| rows_before as i32 * (w.font_size + w.line_gap) as i32);
                let (size, gap) = window.map_or((24, 0), |w| (w.font_size, w.line_gap));
                // rubyon draws sprite text a line gap lower too.
                let sprite_y = if has_ruby { y - gap as i32 } else { y };
                let flag = visited(link.target.as_deref().unwrap());
                for (test, colour, hover) in [("==", LINK_COLOR, LINK_HOVER_COLOR), ("!=", LINK_VISITED_COLOR, LINK_VISITED_HOVER_COLOR)] {
                    writeln!(
                        out,
                        "if {flag} {test} 0 strsp {slot},\"{}\",{x},{sprite_y},{},1,{size},{size},1,{gap},0,1,\"{colour}\",\"{hover}\"",
                        escape(&word),
                        // ONScripter wraps a full-width character one column early.
                        link.len + 1
                    )
                    .unwrap();
                }
                // 1 px under the glyphs, as wide as the word's cells.
                let (ux, uy, uw) = (x + 2, y + size as i32 + 1, link.len as u32 * (size + 1) - 1);
                for (test, colour) in [("==", LINK_COLOR), ("!=", LINK_VISITED_COLOR)] {
                    writeln!(underlines, "if {flag} {test} 0 lsp {line},\":c;>{uw},1,{colour}\",{ux},{uy}").unwrap();
                }
                writeln!(underlines, "lsp {},\":c;>{uw},1,#000000\",{},{}", line + 1, ux + 1, uy + 1).unwrap();
            }
            let bar = script.button_bar.as_ref();
            writeln!(out, "*{page}_wait").unwrap();
            out.push_str(&underlines);
            if bar.is_some() {
                // Clears the buttons: register the words after it.
                writeln!(out, "gosub *{}", menu::BAR_DRAW).unwrap();
            }
            for (n, slot) in (0..clickable.len()).map(link_sprite).enumerate() {
                if let Some(depth) = link_depth {
                    write!(out, "if {depth} < {MAX_LINK_DEPTH} ").unwrap();
                }
                writeln!(out, "spbtn {slot},{}", n + 1).unwrap();
            }
            if bar.is_none() {
                writeln!(out, "print 1").unwrap();
            }
            if script.settings.as_ref().is_some_and(menu::link_wait_keeps) {
                writeln!(out, "gosub *{}", menu::LINK_WAIT).unwrap();
            } else {
                writeln!(out, "btnwait {BUTTON_VAR}").unwrap();
            }
            if bar.is_some() {
                // A bar button or a right-click: handle it and wait again.
                writeln!(out, "if {BUTTON_VAR} >= 0 && {BUTTON_VAR} <= {} goto *{page}_c", menu::BAR_BUTTON_BASE)
                    .unwrap();
                writeln!(out, "gosub *{}", menu::BAR_ACTION).unwrap();
                writeln!(out, "goto *{page}_wait").unwrap();
                writeln!(out, "*{page}_c").unwrap();
                if bar.is_some_and(|b| b.top.is_some()) {
                    // A link: keep this page to come back to. Saved
                    // before textclear, which would make the snapshot
                    // this line instead of the page.
                    writeln!(out, "if {BUTTON_VAR} > 0 savegame {}+{}", menu::SCENE_SLOT, menu::SCENE_DEPTH).unwrap();
                }
            }
            for slot in (0..clickable.len()).map(link_sprite).chain(LINK_UNDERLINE_SPRITES.take(2 * clickable.len()).map(|n| n.to_string())) {
                writeln!(out, "csp {slot}").unwrap();
            }
            writeln!(out, "print 1").unwrap();
            writeln!(out, "textclear").unwrap();
            for (n, link) in clickable.iter().enumerate() {
                let target = link.target.as_deref().unwrap();
                writeln!(out, "if {BUTTON_VAR} != {} goto *{page}_{}", n + 1, n + 1).unwrap();
                writeln!(out, "mov {},1", visited(target)).unwrap();
                let scenes = bar.is_some_and(|b| b.top.is_some());
                if scenes {
                    // Enter a new level (dropping any below it).
                    let (depth, max) = (menu::SCENE_DEPTH, menu::SCENE_MAX_DEPTH);
                    writeln!(out, "add {depth},1").unwrap();
                    writeln!(out, "mov {max},{depth}").unwrap();
                    writeln!(out, "mov {},{}+{depth}", menu::TMP, menu::SCENE_TITLES).unwrap();
                    writeln!(out, "mov ${},\"{}\"", menu::TMP, sprite_text(link.scene.as_deref().unwrap_or("　"))).unwrap();
                }
                // The scene may change the whole screen: save the
                // background, the game's sprites and the BGM around it.
                writeln!(out, "pushscreen {SPRITE_BASE},{}", MAX_SPRITE).unwrap();
                // The variables bring the overlay, static and animation
                // back (their handles are outside pushscreen's range).
                if has_overlay || has_noise || has_animation {
                    writeln!(out, "add {OVERLAY_DEPTH},1").unwrap();
                }
                if has_overlay {
                    writeln!(out, "mov {},{OVERLAY_SAVED}+{OVERLAY_DEPTH}", menu::TMP).unwrap();
                    writeln!(out, "mov %{},{OVERLAY_SLOT}:mov ${},{OVERLAY_IMAGE}", menu::TMP, menu::TMP).unwrap();
                }
                if has_noise {
                    writeln!(out, "mov {},{NOISE_SAVED}+{OVERLAY_DEPTH}", menu::TMP).unwrap();
                    writeln!(out, "mov %{},{NOISE_ALPHA}:mov ${},{NOISE_IMAGE}", menu::TMP, menu::TMP).unwrap();
                }
                if has_animation {
                    writeln!(out, "mov {},{ANIMATION_SAVED}+{OVERLAY_DEPTH}", menu::TMP).unwrap();
                    writeln!(out, "mov %{},{ANIMATION_ALPHA}:mov ${},{ANIMATION_IMAGE}", menu::TMP, menu::TMP).unwrap();
                }
                if root_returns > 0 {
                    writeln!(out, "add {LINK_NEST},1").unwrap();
                }
                writeln!(out, "gosub *{}", label(target)).unwrap();
                if root_returns > 0 {
                    writeln!(out, "sub {LINK_NEST},1").unwrap();
                }
                if scenes {
                    // The scene ended: it can't be gone forward to.
                    let (depth, max, floor) = (menu::SCENE_DEPTH, menu::SCENE_MAX_DEPTH, menu::SCENE_FLOOR);
                    writeln!(out, "sub {depth},1").unwrap();
                    writeln!(out, "mov {max},{depth}").unwrap();
                    writeln!(out, "if {floor} > {depth} mov {floor},{depth}").unwrap();
                }
                // The scene's fade to a colour isn't kept by pushscreen.
                writeln!(out, "popscreen{clear_fade}").unwrap();
                if has_overlay {
                    writeln!(out, "mov {},{OVERLAY_SAVED}+{OVERLAY_DEPTH}", menu::TMP).unwrap();
                    writeln!(out, "mov {OVERLAY_SLOT},%{}:mov {OVERLAY_IMAGE},${}", menu::TMP, menu::TMP).unwrap();
                }
                if has_noise {
                    writeln!(out, "mov {},{NOISE_SAVED}+{OVERLAY_DEPTH}", menu::TMP).unwrap();
                    writeln!(out, "mov {NOISE_ALPHA},%{}:mov {NOISE_IMAGE},${}", menu::TMP, menu::TMP).unwrap();
                }
                if has_animation {
                    writeln!(out, "mov {},{ANIMATION_SAVED}+{OVERLAY_DEPTH}", menu::TMP).unwrap();
                    writeln!(out, "mov {ANIMATION_ALPHA},%{}:mov {ANIMATION_IMAGE},${}", menu::TMP, menu::TMP).unwrap();
                }
                if has_overlay || has_noise || has_animation {
                    writeln!(out, "sub {OVERLAY_DEPTH},1").unwrap();
                    let handles = [(has_overlay, OVERLAY_SPRITE), (has_noise, NOISE_SPRITE), (has_animation, ANIMATION_SPRITE)];
                    let csp: Vec<String> = handles.iter().filter(|h| h.0).map(|h| format!("csp {}", h.1)).collect();
                    writeln!(out, "{}", csp.join(":")).unwrap();
                    out.push_str(restore_overlay);
                }
                if let Some(w) = window {
                    // The scene may have switched the text window.
                    writeln!(out, "{}", strings.lines(WINDOW, &setwindow(w, speed, has_ruby, has_vertical, true, script))).unwrap();
                }
                if root_returns > 0 {
                    writeln!(out, "if {ROOT_RESUME} != 0 goto *{UNWIND}").unwrap();
                }
                writeln!(out, "print 1").unwrap();
                writeln!(out, "goto *{page}").unwrap();
                writeln!(out, "*{page}_{}", n + 1).unwrap();
            }
            if bar.is_some_and(|b| b.top.is_some()) {
                // Going on from the page drops the levels below it.
                writeln!(out, "mov {},{}", menu::SCENE_MAX_DEPTH, menu::SCENE_DEPTH).unwrap();
            }
        }
        Op::BgmPlay { file, fade_in_ms } => {
            if se.volumes && !std::mem::take(&mut se.preset) {
                writeln!(out, "chvolfade -1,100").unwrap();
            }
            writeln!(out, "bgmfadein {fade_in_ms}").unwrap();
            writeln!(out, "bgm \"{}\"", path(file)).unwrap();
        }
        Op::BgmCrossfade { file, ms } => {
            if se.volumes && !std::mem::take(&mut se.preset) {
                writeln!(out, "chvolfade -1,100").unwrap();
            }
            writeln!(out, "bgm \"{}\",{ms}", path(file)).unwrap();
        }
        Op::BgmLater { file, after_ms: 0, looped } => {
            writeln!(out, "bgmfadein 0").unwrap();
            // mp3save: played once, but a load plays it again (bgmonce's isn't saved)
            writeln!(out, "{} \"{}\"", if *looped { "bgm" } else { "mp3save" }, path(file)).unwrap();
        }
        Op::BgmLater { file, after_ms, looped } => {
            writeln!(out, "bgmlater {after_ms},\"{}\"{}", path(file), if *looped { "" } else { ",0" }).unwrap();
        }
        Op::MusicRoll { .. } => roll::emit(out, i, op, script),
        Op::HoldMovie { file, prompt, sound } => hold_movie::hold_movie(out, se, file, prompt.as_ref(), sound.as_ref()),
        Op::BgmStop { fade_out_ms } => {
            writeln!(out, "bgmfadeout {fade_out_ms}").unwrap();
            writeln!(out, "bgmstop").unwrap();
        }
        Op::Wait { ms } => writeln!(out, "{}delay {ms}", unless_auto(script, i)).unwrap(),
        Op::Pause { ms } => writeln!(out, "wait {ms}").unwrap(),
        Op::Flash { colour, opacity, ms } => {
            // An additive sprite2: over the picture, under the text window.
            let alpha = (opacity * 255 / 256).min(255);
            let (w, h) = (script.width, script.height);
            writeln!(out, "lsp2add {FLASH_SPRITE2},\":c;>{w},{h},#{colour:06x}\",{},{},100,100,0,{alpha}", w / 2, h / 2).unwrap();
            writeln!(out, "print 1\ndelay {ms}\ncsp2 {FLASH_SPRITE2}\nprint 1").unwrap();
        }
        // CONscripter's screenflash adds the colour over everything.
        Op::ScreenFlash { colour, opacity, ms } => {
            writeln!(out, "screenflash {},\"#{colour:06x}\"\nprint 1\ndelay {ms}\nscreenflash 0\nprint 1", opacity.min(&256)).unwrap();
        }
        Op::ScreenFade { colour, ms } => {
            // An opaque sprite2 faded in: over the picture, under the
            // text window and the bars.
            let (w, h) = (script.width, script.height);
            writeln!(out, "lsp2 {FADE_SPRITE2},\":c;>{w},{h},#{colour:06x}\",{},{},100,100,0,255", w / 2, h / 2).unwrap();
            writeln!(out, "print {}", effect(*ms)).unwrap();
        }
        Op::SePlay { file, looped } => {
            let ch = se.start(file, *looped);
            if se.volumes && !std::mem::take(&mut se.preset) {
                writeln!(out, "chvolfade {ch},100").unwrap();
            }
            // A channel panned before plays this sound centred, unless
            // panned for it.
            if !std::mem::take(&mut se.pan_preset) && se.panned.remove(&ch) {
                writeln!(out, "chpan {ch},0").unwrap();
            }
            let cmd = if *looped { "dwaveloop" } else { "dwave" };
            let when = if std::mem::take(se_not_skipped) { format!("if {} != 1 ", menu::TMP) } else { String::new() };
            writeln!(out, "{when}{cmd} {ch},\"{}\"", path(file)).unwrap();
        }
        // isskip: 1 while skipping (2 in auto mode).
        Op::SeNotSkipped => {
            writeln!(out, "isskip {}", menu::TMP).unwrap();
            *se_not_skipped = true;
        }
        Op::SeStop { file: Some(file) } => {
            if let Some(ch) = se.stop(file) {
                writeln!(out, "dwavestop {ch}").unwrap();
            }
        }
        Op::SeStop { file: None } => {
            for ch in se.stop_all() {
                writeln!(out, "dwavestop {ch}").unwrap();
            }
        }
        // CONscripter's channel fades (`chvolfade`).
        Op::SeFadeIn { ms } => {
            let when = if *se_not_skipped { format!("if {} != 1 ", menu::TMP) } else { String::new() };
            writeln!(out, "{when}dwavefadein {ms}").unwrap();
        }
        Op::SeFadeOut { file, ms } => {
            let channels = match file {
                Some(file) => se.stop(file).into_iter().collect(),
                None => se.stop_all(),
            };
            let curve = sound_curve(script, i, eased).map_or(String::new(), |c| format!(",{c}"));
            for ch in channels {
                writeln!(out, "chvolfade {ch},0,{ms},1{curve}").unwrap();
            }
        }
        // (Read by the sound op after it.)
        Op::SoundEase { .. } => {}
        // The wait right after it runs only when this one didn't.
        Op::AutoWait { ms } => writeln!(out, "isskip {0}\nif {0} == 2 delay {ms}", menu::TMP).unwrap(),
        Op::SoundVolume { file, percent, ms } => {
            // Right before the sound starts: for the channel it will
            // play on (CONscripter keeps a channel's volume for its next
            // sounds).
            let next = script.ops.get(i + 1);
            se.preset = match (file, next) {
                (None, Some(Op::BgmPlay { .. } | Op::BgmCrossfade { .. })) => true,
                (Some(f), Some(Op::SePlay { file: g, .. })) => f == g,
                _ => false,
            };
            let ms = match sound_curve(script, i, eased) {
                Some(c) if *ms > 0 => format!(",{ms},0,{c}"),
                _ if *ms > 0 => format!(",{ms}"),
                _ => String::new(),
            };
            match file {
                None => writeln!(out, "chvolfade -1,{percent}{ms}").unwrap(),
                Some(f) => {
                    let ch = if se.preset { Some(se.peek()) } else { se.playing.get(f).copied() };
                    if let Some(ch) = ch {
                        writeln!(out, "chvolfade {ch},{percent}{ms}").unwrap();
                    }
                }
            }
        }
        // CONscripter's chpan, right before the sound
        // starts for the channel it will play on, as SoundVolume.
        Op::SoundPan { file, pan, ms } => {
            let starts = |op: Option<&Op>| matches!(op, Some(Op::SePlay { file: g, .. }) if g == file);
            let next = match script.ops.get(i + 1) {
                Some(Op::SoundVolume { .. }) => script.ops.get(i + 2),
                next => next,
            };
            se.pan_preset = starts(next);
            let ch = if se.pan_preset { Some(se.peek()) } else { se.playing.get(file).copied() };
            if let Some(ch) = ch {
                let ms = if *ms > 0 { format!(",{ms}") } else { String::new() };
                writeln!(out, "chpan {ch},{pan}{ms}").unwrap();
                se.panned.insert(ch);
            }
        }
        Op::SoundRate { file, rate, ms } => {
            if let Some(ch) = se.playing.get(file) {
                let ms = match sound_curve(script, i, eased) {
                    Some(c) if *ms > 0 => format!(",{ms},{c}"),
                    _ if *ms > 0 => format!(",{ms}"),
                    _ => String::new(),
                };
                writeln!(out, "chfreq {ch},{rate}{ms}").unwrap();
            }
        }
        Op::ClickWait { skippable } => {
            *needs_click_wait = true;
            writeln!(out, "{}mov {BUTTON_VAR},{}:gosub *{CLICK_WAIT}", unless_auto(script, i), u8::from(*skippable)).unwrap();
        }
        Op::Movie { file, skippable } => {
            // The web build plays it in a <video> element over the game.
            writeln!(out, "bgmstop").unwrap();
            writeln!(out, "mpegplay \"{}\",{}", path(file), u8::from(*skippable)).unwrap();
            if let Some(seen) = movies::seen(script, file) {
                writeln!(out, "{seen}").unwrap();
            }
        }
        Op::SceneMovie { file, skippable } => {
            writeln!(out, "mpegplay \"{}\",{},1", path(file), u8::from(*skippable)).unwrap();
        }
        Op::AnimatedSprite { layer, sprite, cells, frame_ms, looped, zoom } => {
            let slot = SPRITE_BASE + layer;
            let mode = sprite_mode(&sprite.image);
            // Loop mode 0 repeats the cells, 1 stops on the last.
            let tag = format!("{mode}/{cells},{frame_ms},{}", u8::from(!*looped));
            writeln!(out, "spfilter {slot},0").unwrap();
            writeln!(out, "lsp {slot},\":{tag};{}\",{},{}", path(&sprite.image), sprite.x, sprite.y).unwrap();
            if *zoom != (100, 100) || zoomed_layers.contains(layer) {
                writeln!(out, "spzoom {slot},{},{}", zoom.0, zoom.1).unwrap();
            }
            if transition.is_none() {
                writeln!(out, "print 1").unwrap();
            }
        }
        Op::Chapter { title } => {
            writeln!(out, "mov {},\"{}\"", menu::CHAPTER, sprite_text(title)).unwrap();
            if has_chapters && !title.is_empty() {
                writeln!(out, "gosub *{}", chapters::KEEP).unwrap();
            }
        }
        Op::ChapterHistoryClear if has_chapters => writeln!(out, "mov {},0:mov {},0", chapters::COUNT, chapters::CURRENT).unwrap(),
        Op::ChapterHistoryClear => {}
        Op::UnlockScene { scene } => writeln!(out, "mov {},1", SCENE_UNLOCKED.at(*scene)).unwrap(),
        // Starts the script over: the title screen.
        Op::ReturnToTitle => writeln!(out, "reset").unwrap(),
        Op::ReturnToRoot => {
            *root_resumes += 1;
            writeln!(out, "if {LINK_NEST} > 0 mov {ROOT_RESUME},{root_resumes}:return").unwrap();
            writeln!(out, "*conv_root{root_resumes}").unwrap();
        }
        Op::Progress { bit } => writeln!(out, "mov {},1", PROGRESS.at(*bit)).unwrap(),
        Op::CgSeen { number } => writeln!(out, "mov {},1", CG_SEEN.at(*number)).unwrap(),
        Op::SceneEnd if has_replay => writeln!(out, "if {} == 1 goto *{}", replay::REPLAYING, replay::END).unwrap(),
        // a scene the picture extras' replay gosubbed: back to its list
        Op::SceneEnd if script.picture_extras.as_ref().is_some_and(|e| e.replay.is_some()) => {
            writeln!(out, "if {} == 1 return", crate::picture_replay::REPLAYING).unwrap()
        }
        Op::SceneEnd => writeln!(out, "; scene end").unwrap(),
        op @ (Op::Choice { .. } | Op::RuntimeChoice { .. }) => {
            let (options, texts, style, disabled) = match op {
                Op::Choice { options, style, disabled } => (options, &[][..], style, &disabled[..]),
                Op::RuntimeChoice { options, texts, style } => (options, &texts[..], style, &[][..]),
                _ => unreachable!(),
            };
            assert!(texts.len() <= crate::RUNTIME_OPTIONS, "a choice with more than {} options set at run time", crate::RUNTIME_OPTIONS);
            let plates = style.as_ref().and_then(|s| s.plates.as_ref());
            if let Some(s) = style.as_ref().filter(|_| plates.is_none()) {
                writeln!(out, "selectcolor \"#{:06X}\",\"#{:06X}\"", s.on_color, s.off_color).unwrap();
                writeln!(out, "{}", strings.lines(WINDOW, &setwindow(&s.window, speed, has_ruby, has_vertical, false, script))).unwrap();
                writeln!(out, "csp {NAME_PLATE_SPRITE}:csp {NAME_TEXT_SPRITE}").unwrap();
                *plate_visible = false;
                *page_rows = 0;
                *window = Some(s.window.clone());
            }
            if back_to_choice {
                // For going back here (the page before it, as saves are).
                writeln!(out, "savegame {}", menu::BACK_SLOT).unwrap();
            }
            if let Some(p) = plates {
                assert!(disabled.is_empty(), "disabled options on picture plates");
                choice_plates::emit(out, strings, options, texts, p, *plate_choices, script.button_bar.is_some(), border);
                *plate_choices += 1;
            } else {
                let mut items = Vec::new();
                for (i, (t, l)) in options.iter().enumerate() {
                    match texts.get(i).and_then(Option::as_ref) {
                        Some(rt) => {
                            let (s, _) = crate::runtime_option(i);
                            writeln!(out, "mov {s},\"{}\"", strings.string(CHOICE, text_safe(t))).unwrap();
                            for (value, text) in &rt.texts {
                                writeln!(out, "if {} == {value} mov {s},\"{}\"", var_name(rt.var, border), strings.string(CHOICE, text_safe(text))).unwrap();
                            }
                            items.push(format!("{s},*{}", label(l)));
                        }
                        None => items.push(format!("\"{}\",*{}", strings.string(CHOICE, text_safe(t)), label(l))),
                    }
                }
                if !disabled.is_empty() {
                    // just before the select: a load (BACK_SLOT too) runs it again
                    writeln!(out, "seldisable {}", disabled.iter().map(|i| (i + 1).to_string()).collect::<Vec<_>>().join(",")).unwrap();
                }
                writeln!(out, "select {}", items.join(",")).unwrap();
            }
        }
        Op::TilePuzzle(p) => {
            puzzle::emit(out, p, *puzzles, SPRITE_BASE, &effect, (script.width, script.height), border);
            *puzzles += 1;
            *plate_visible = false;
        }
        Op::Unsupported { what, at } => {
            writeln!(out, "; unsupported: {what} ({} #{})", at.file, at.index).unwrap()
        }
        Op::ButtonScreen(s) => {
            button_screen::emit(out, s, *button_screens, &effect(s.fade_ms), script.button_bar.is_some());
            *button_screens += 1;
        }
        Op::PictureScreen(s) => {
            picture_screen::emit(out, s, *picture_screens, &effect(s.fade_ms), script.button_bar.is_some(), border);
            *picture_screens += 1;
        }
    }
    }
}

/// The `chvolfade`/`chfreq` curve of the sound op at `i`: the
/// [`Op::SoundEase`] right before it, as an `spmotion` curve (`None`
/// straight).
/// The condition that keeps a wait to normal play when an [`Op::AutoWait`]
/// right before it waits in auto mode.
fn unless_auto(script: &Script, i: usize) -> String {
    match i.checked_sub(1).and_then(|j| script.ops.get(j)) {
        Some(Op::AutoWait { .. }) => format!("if {} != 2 ", menu::TMP),
        _ => String::new(),
    }
}

fn sound_curve(script: &Script, i: usize, eased: &mut BTreeMap<(u8, u32), u32>) -> Option<u32> {
    match i.checked_sub(1).and_then(|j| script.ops.get(j)) {
        Some(&Op::SoundEase { ease, exponent }) => Some(crate::ease::curve(Some((ease, exponent)), eased)).filter(|&c| c > 0),
        _ => None,
    }
}
