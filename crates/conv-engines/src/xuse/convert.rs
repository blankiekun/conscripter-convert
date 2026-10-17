//! Lowers a Xuse `.cd` script to the IR and writes the assets it uses.
//!
//! Assumptions that still need checking in a running conversion are marked
//! "assumption".

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use thiserror::Error;
use conv_core::ir::{
    Compare,
    Animation, ArithOp, Backlog, CgGallery, CgViewer, ExtrasAction, ExtrasItem, ExtrasMenu, GallerySlot, ReplayList, ReplayScene, SceneReplay, MusicRoom, MusicSong, GalleryTab, BarAction, BarButton, ButtonBar, ConfirmDialog, NamePlate, Noise, ScreenEffect, Setting, SettingRow, SettingSwitch, SettingsMenu, Switch, Op, Operand, PageButton, Placed, Rect, Ruby, SaveMenu, TextStyle, SourceLoc, Voice, VoiceToggle,
    SpriteChange, SpriteFilter, SpritePlacement, TextLink, TextSpot, TextWindow, TitleAction, TitleButton, TitleScreen, TopBars, ChapterHistory,
    SlideLayer, TransitionStyle, Var, ZoomStyle, FILTER_BLEND_ADD, FILTER_BLEND_COPY, FILTER_BLEND_MULTIPLY, FILTER_BLEND_SCREEN, FILTER_BRIGHTNESS, FILTER_COLOUR, FILTER_MONOTONE,
    FILTER_GREYSCALE, FILTER_NEGATIVE, FILTER_BLUR, FILTER_HUE_SATURATION, FILTER_LIGHTNESS_INVERT,
};

use crate::xuse::cd::{
    PRESET_KIND_ZIGZAG, CdError, CdScript, EnvModule, Instr, Link, PresetModule, Quake, Surprise, SURPRISE_PRESETS, ENV_KIND_NOISE, FLASH_MS, FLASH_OPACITY, OP_FLASH, OP_QUAKE, OP_SURPRISE, Ruby as RubyRecord, ScreenModule, Sound, SpriteEntry, TextStyle as StyleRecord, OP_TEXT_INDENT, PRESET_KIND_MONOTONE, PRESET_BLEND_ADD, PRESET_BLEND_ALPHA, PRESET_BLEND_COPY,
    PRESET_BLEND_MULTI, PRESET_BLEND_SCREEN, PRESET_KIND_BRIGHTNESS, PRESET_KIND_COLOUR, PRESET_KIND_GREYSCALE,
    PRESET_KIND_NEGATIVE, PRESET_KIND_SMOOTH, PRESET_KIND_HUE_SATURATION, PRESET_KIND_BRIGHTNESS_INVERT, PRESET_KIND_REPLACE,
    ModuleData,
};
use crate::media;
use crate::xuse::gaf::{Gaf, GafError};
use crate::xuse::gallery;
use crate::xuse::music;
use crate::xuse::replay;
use crate::xuse::staffroll;
use crate::xuse::symbols::SymbolTable;
use crate::xuse::xarc::{Xarc, XarcError};
use crate::xuse::xsa::{Xsa, XsaError};

mod screens;

/// Control flow, from the engine's handlers.
///
/// - 0x10/0x11 take a label *number* in the current script (`arg`); the
///   engine resolves it with (current script, label) at 0x4706a0.
/// - 0x15/0x16/0x22 (and links) take a label-table *index* (`sub`), resolved
///   at 0x470830. The engine's in-memory table has one extra entry at index
///   0 (the header counts it, the file doesn't store it), so index `n` is
///   stored entry `n - 1`.
const OP_GOTO: u16 = 0x10;
const OP_GOTO_2: u16 = 0x11;
const OP_GOTO_SUB: u16 = 0x15;
/// `if v[a] <cmp> b goto label[sub]`, (a, b) as two u32 at pool[arg].
const OP_IF_EQUAL: u16 = 0x16;
/// The last of the comparisons (0x16-0x1b: ==, !=, <, >, <=, >=).
const OP_IF_GE: u16 = 0x1b;
const OP_GOSUB: u16 = 0x22;
const OP_RETURN: u16 = 0x23;
/// Ends a scene entered through a link and goes back to the page with the
/// link. The handler (`ScriptText.cpp`) fails with "in the root scene, can't
/// go back further" when no link scene is active.
const OP_LINK_RETURN: u16 = 0x47;
/// Timed waits: the low 16 bits of `arg` are milliseconds (the high half
/// is a flag, meaning unknown). Both handlers install a wait callback that
/// also checks for clicks.
/// Arms a screen transition: changes until the next [`OP_TRANSITION_WAIT`]
/// are revealed through it. `sub` = effect (jump table at 0x44a138);
/// pool[arg] = u32 duration ms, u32 flags, u32 curve/parameter.
const OP_TRANSITION: u16 = 0x76;
/// Arms a mask ("rule") transition (`Trans_MaskBlend2`): pool[arg] = u32
/// mask number (`mask%04u.png` in `HD/mask.arc`), i32 softness (sign =
/// direction), then the same 12-byte timing block as 0x76.
const OP_MASK_TRANSITION: u16 = 0x77;
/// Waits for the running transition (a click finishes it early).
const OP_TRANSITION_WAIT: u16 = 0x71;
/// Text window preset parts, found only in script 2's preset subroutines:
/// mode (0 = message box, 1 = full screen), style (`TextWnd{style}.004` in
/// `HD/wnd.arc`), position (x | y << 16).
const OP_WINDOW_MODE: u16 = 0x4c;
const OP_WINDOW_STYLE: u16 = 0x4f;
const OP_WINDOW_POS: u16 = 0x50;
/// Text control tags between messages (handler names "brn" and "p"):
/// continue below after a click / start a new page. Only used in
/// full-screen modes, where untagged messages continue too.
const OP_TEXT_BRN: u16 = 0x52;
const OP_TEXT_PAGE: u16 = 0x53;
/// Hides the text window (handler 0x4772a0; arg 0 waits for the fade, 1
/// doesn't). Used before most transitions.
const OP_HIDE_TEXT: u16 = 0x43;
/// How the current text window shows its messages (handler 0x467a50; byte
/// +0x1c of the window, copied to the message at 0x46e882): 0 types one
/// character at a time, 1 shows the whole message at once, 2 types all
/// its lines at the same time (seen in the original under wine). The
/// window presets set it (1 in three full-screen ones, else 0); the story
/// sets 1 for full-screen adverts and 2 for lines two characters say
/// together. ONScripter can't type lines
/// in parallel, so 2 types them one after another.
const OP_TEXT_DISPLAY: u16 = 0x4d;
/// Starts a chapter: pool[arg] = (own label number, label-table index of
/// the chapter's "|chpe" label). The engine shows the name of the label it
/// sits on (`0.cd`): "chapter<route>]?<title>". 0x37 and 0x38 sit on the
/// chapter's "|chpe" and "|chpl" labels and link back (chapter history).
const OP_CHAPTER: u16 = 0x36;
/// Ends the chapter (handler 0x475780, on "|chpe"): pool[arg] = (own label
/// number, table index of "|chpl"). Outside the replay it closes the
/// chapter 0x36 opened, marks the label read and jumps to "|chpl", which is
/// always the next instruction (0x38), so nothing is emitted.
const OP_CHAPTER_END: u16 = 0x37;
/// After a chapter (handler 0x475970, on "|chpl"): outside the replay it
/// clears the chapter name and redraws the chapter bar (0x49bb70,
/// ChapterBar.cpp), which stays empty until the next 0x36.
const OP_CHAPTER_CLEAR: u16 = 0x38;
/// Unlocks scene arg of the scene replay (handler 0x478830, Story.cpp): it
/// sets bit arg of a 600-bit global set (0x51eaa0, saved as "RMSN"), which
/// the replay list (0x451090) reads. One per scene, before its end.
const OP_UNLOCK_SCENE: u16 = 0x93;
/// Ends a scene the replay can play (handler 0x4789a0): in replay modes 1
/// and 2 (0x51ec70) it returns to the replay; in normal play it does nothing.
const OP_SCENE_END: u16 = 0x94;
/// Sets progress flag `sub` (handler 0x478b00: bit `sub` of the global
/// word 0x51eb3c, saved with the system data at 0x45f801). arg is -1. The
/// game sets bits 0-3 at the start (0000.cd, 0003.cd) and bits 4-7 at the
/// end of each heroine's route (0009-0013.cd). The extras read them: the
/// title (bit 3, 0x45cce9), getters for bits 1 and 2 (0x44cc60, 0x4579e0)
/// and the voice mode (`VoiceMode.cpp`, `wss.bin`: bit 4 + route, table
/// 0x4fbc00).
const OP_PROGRESS: u16 = 0x99;
/// Marks CG `arg` (1-based, as in 0x54) as seen in the CG gallery without
/// showing it (handler 0x478b30: bit arg - 1 of the global set at
/// 0x51ea18, which `CGMode.cpp` reads at 0x44cd56 and 0x44d4af; 0x55
/// with its "mark as seen" byte sets the same bits, 0x42236b). Used 6
/// times, for CGs 519-524.
const OP_CG_SEEN: u16 = 0x9d;
/// The replay set's size.
const REPLAY_SCENES: i32 = 600;
/// Waits for a click with the text window, button tray and top bars hidden
/// (handler 0x4784b0, wait callback 0x478370). arg 0 also turns skip mode
/// off; with arg 1 skip mode passes through.
const OP_CLICK_WAIT: u16 = 0x68;
/// The same wait without an argument (handler 0x473940, wait callback
/// 0x473810): the picture alone with the wait icon in the corner (seen in
/// the original under wine); skip mode passes it.
const OP_CLICK_WAIT_SKIPPABLE: u16 = 0x69;
/// Full-screen animations (fx), four opcodes on one object (layer u8 0,
/// depth u32 1000; for 0x7a and 0x7b with layer 2 see [`OP_PICTURE`]):
/// - 0x7a creates it (handler 0x414590): u8 layer, u32 depth, u8 group
///   (3; the sort band through the table at 0x503e68), u32 sort depth, u16
///   x, y, width, height, u8 blend (0 = normal), u16 opacity (0-256), u8
///   visible;
/// - 0x8b loads animation module 1 into it (0x416310, Aom_Animation_Parts):
///   u8 layer, u32 depth, u32 module, u32 ?, 32-byte name (`fxNNNN`,
///   an `.004` and an `.ap` in `HD/fx.arc`); it starts playing at once;
/// - 0x8a waits until the module's last frame (0x416200, wait 0x4160d0;
///   never for one that loops): u8 layer, u32 depth, u32 module, u8 a click
///   or skip ends the wait;
/// - 0x7b removes the object (0x414820): u8 layer, u32 depth, u8 ?.
const OP_FX_CREATE: u16 = 0x7a;
const OP_FX_LOAD: u16 = 0x8b;
const OP_FX_WAIT: u16 = 0x8a;
const OP_FX_REMOVE: u16 = 0x7b;
/// Removes every character bust-up (arg is always -1).
const OP_CLEAR_SPRITES: u16 = 0x8f;
/// A second background object, "bg-<slot>" (handlers 0x422440, 0x4222c0,
/// 0x412fe0), mostly one of the `frameNN` pictures: letterbox bars, speed
/// lines, a heart border. Record: u8 slot (always 2), u8 layer, u32 depth,
/// u32 background (1-based as in 0x54, image id 10,000,000 + n - 1; 0
/// removes it), u8 "mark as seen". Layers 0 and 1 are the background's
/// (under the bust-ups), 2 the bust-ups' (0x10); 3 and 4 aren't used.
/// Assumption: within a layer the higher depth is drawn on top, so the
/// usual depth 1000 puts it over the bust-ups (10-100 in 0x8e). No fade of
/// its own. It stays when the background changes or the bust-ups go.
const OP_OVERLAY: u16 = 0x55;
/// Fills a background slot with a colour (handler 0x4224a0, then 0x412fe0
/// with kind 1: a screen-sized object whose module 7 fills it). Record: u8
/// slot (0 removes slots 1-10), u8 layer, u32 depth, u32 COLORREF. Slot 1
/// is the background (0x54's), slot 2 the overlay (0x55's). Uses: black
/// on slot 1 at boot (0000.cd), white on the overlay in the shared impact
/// routine (0002.cd `s2_l2`, 9 calls: 0x83 sets its blend to 6, dissolve,
/// and 0x86 fades it out over 1 s), and red on slot 1 at opacity 64 (0x82)
/// in 0006.cd's "Server Error" scene.
const OP_FILL: u16 = 0x56;
/// Sets an object's blend (handler 0x414050, field +0x1b4): u8 layer, u32
/// depth, u8 blend (0-6: Alpha, Add, Multi, Screen, SignedAdd, Copy,
/// dissolve). Used once, on the white overlay of 0x56 (dissolve, drawn as
/// a cross-fade here).
const OP_BLEND: u16 = 0x83;
/// Clears the chapter history (handler 0x475af0, `ScriptStd.cpp`: "チャプタ
/// 内ではチャプタ履歴をクリアできません。" inside a chapter); nothing in
/// replay modes 1 and 2. Used at the start of 0004.cd and after the ends of
/// 0021/0022.cd. The conversion has no chapter history yet. It is
/// converted as a clear of the environment effects (0x8c slot 0): the end
/// of 0022.cd (存在消滅) leaves the static at full opacity and jumps back
/// to the start of 0004.cd, and the original starts that week clean
/// (filmed under wine; the code that clears it wasn't found).
const OP_CLEAR_CHAPTER_HISTORY: u16 = 0x39;
/// Copies `sub` bytes of the record (a name) into the game state at +4
/// (handler 0x478650); only 0x3c (0x4786c0, a jump when the name matches)
/// reads it, and no script uses 0x3c. The boot script sets "debug entry"
/// or "release entry". Nothing to emit.
const OP_ENTRY_NAME: u16 = 0x3b;
/// Plays a staff roll (handler 0x4788d0, task 0x454a60; see
/// [`staffroll`]): arg 0 or 1. Roll 1 (with its song, 223 s) opens the
/// epilogue (0003.cd #2), which the end of 0008.cd jumps to; roll 0 (63 s)
/// plays from the boot script's `s0_l8`, which the route ends call. Neither a click nor a key ends it.
const OP_STAFF_ROLL: u16 = 0x91;
/// Returns to the root scene (handler 0x477900, `ScriptText.cpp`): leaves
/// every open link scene (0x467670); with none open it logs "既にルートシー
/// ンにいるので何も行いません。" and does nothing. Routes 0010 and 0011.cd,
/// entered from a link, start with it (#7) and never return to the link
/// page (they end with jumps).
const OP_RETURN_TO_ROOT: u16 = 0x48;
/// Sets the text style of the messages that follow (handler 0x477af0;
/// values decoded like 0x49's by 0x491d80 / 0x491e30): u16 size (0x1000 +
/// n a preset, below 0x1000 percent, 0xf000 + k: 0 none, 1 the default,
/// 2 kept), u32 COLORREF (top byte 0xff: k as for the size), u32 edge
/// colour (stored twice; not converted). Used once in 0011.cd: four lines
/// at 125 % in orange, then back to the defaults.
const OP_DEFAULT_TEXT_STYLE: u16 = 0x4a;
/// Waits until environment slot n's module has finished (handler 0x4155c0,
/// wait task 0x4154b0; object key n + 0x78): u32 slot, u8 (0xff). Once, in
/// 0007.cd, after a 3 s ripple.
const OP_ENV_WAIT: u16 = 0x8d;
/// Yields for one frame (handler 0x473d20 returns 0). Nothing to emit.
/// 0x00 has the same handler (once, at the start of 0006.cd).
const OP_YIELD: u16 = 0x67;
const OP_YIELD_2: u16 = 0x00;
/// Returns to the title (handler 0x478330: stops the sounds (0x46f570),
/// then the title task 0x45b6f0 in mode 0, as the extras do at 0x44f6c1).
/// Once, at the very end of the epilogue (0003.cd #1280).
const OP_RETURN_TO_TITLE: u16 = 0x96;
/// Sets an object's opacity (handler 0x413f10; draw field +0x1b2).
/// Record: u8 layer, u32 depth, u16 opacity (0-256). The object is found
/// by its key: the depth for layers 0 and 1, depth + 10 for layer 2 (a
/// character's bust-up, as 0x90 finds it), + 110 and + 120 for 3 and 4.
/// The game uses layer 2 (34 uses, characters 1-60) and layer 1 depth 2
/// (3, the overlay "bg-2" of 0x55) and depth 1 (1, after 0x56).
const OP_OPACITY: u16 = 0x82;
/// Loads a picture into an object (handler 0x414180): u8 layer, u32 depth
/// (found as in 0x82), u8 kind, u32 number (image id kind × 10,000,000 +
/// number - 1, so kind 2 numbers bust-ups as 0x8e does), u8 ?. The game
/// uses it on layer 2 objects that 0x7a made (6 uses: blog scenes, a
/// hidden object at depth 2000 in group 4, the top band: over the text
/// window), then 0x7d and 0x7c; one scene (0135.cd) steps a meter through
/// five pictures. 0x7b on layer 2 removes the object, also a bust-up 0x8e
/// made (5 uses).
const OP_PICTURE: u16 = 0x84;
const PICTURE_KIND_BUSTUP: u8 = 2;
/// 0x7a's group byte for the top sort band, over the text window (the
/// blog scenes' bust-ups).
const PICTURE_GROUP_TOP: u8 = 4;
/// Sets which point of an object its position names (handler 0x413940):
/// u8 layer, u32 depth, u8 horizontal, u8 vertical, each 0-2 (left/top,
/// centre, right/bottom; flags 0x504028/0x504034 in field +0x180, 0 for a
/// new object).
const OP_ANCHOR: u16 = 0x7d;
/// Shows or hides an object (handler 0x414460, 0x40f1b0): u8 layer, u32
/// depth, u8 visible (0xff in every use).
const OP_SHOW: u16 = 0x7c;
/// Scales an object (handler 0x413cd0; fields +0x1a8, +0x1ac, 100 for a new
/// object): u8 layer, u32 depth (found as in 0x82), i32 x, y percent. Once
/// (0013.cd #10884): character 12 at 80 %, right after 0x8e showed it and
/// 0x82 made it translucent; scaled about its centre here.
const OP_SCALE: u16 = 0x80;
/// Puts an object at a position (handler 0x413bb0): u8 layer, u32 depth
/// (found as in 0x82), i32 x, y, the centre in 0x8e's coordinates.
const OP_POSITION: u16 = 0x7f;
/// Adds a module to an object or removes one (handler 0x413540, then
/// 0x413350): u8 layer, u32 depth (found as in 0x82), u32 slot, u32 kind.
/// Kind 0x80000001 removes the slot's module; any other kind makes a
/// key-frame motion (`AdvObject::CModule_Animation_KeyFrameMotion`, setup
/// 0x416fe0) and is its flags: which fields the key frames have, in this
/// order after the u32 time in ms: 0x10 u8; 0x20 i32, i32, u8; 0x40 the
/// position (i32 x, y; u8); 0x80 i32, i32, u8; 0x100 the rotation (i32,
/// tenths of a degree?; u8); 0x200 the opacity (u16; u8). Each u8 is how
/// the field moves towards that key frame (0x4176f0: 0 kept, 1 a jump
/// halfway, 2 linear, 3 a curve). A u16 key frame count follows the flags;
/// the first key frame (time 0) is replaced by the object's state. The
/// game slides bust-ups (9 uses, linear, one with a rotation of -900) and
/// fades the overlay out once (opacity).
const OP_MODULE: u16 = 0x86;
const MODULE_REMOVE: u32 = 0x8000_0001;
const MOTION_POSITION: u32 = 0x40;
const MOTION_OPACITY: u32 = 0x200;
/// Waits for an object's module to finish (handler 0x413820): u8 layer,
/// u32 depth, u32 slot, u8 (0xff). Each slide is followed by this wait,
/// the removal of the module and a 0x7f to the end point.
const OP_MODULE_WAIT: u16 = 0x87;
/// Effect presets (0x88 defines a module, 0x90 attaches a preset to a
/// bust-up) are kept in variables, so they follow the game's control flow:
/// preset p, slot s is [`PRESET_FIELDS`] local variables from
/// [`PRESET_VAR_BASE`] (the game's own locals are 0-2). Preset 1 is the
/// lighting tint (script 2 redefines it per mood); 2 and 3 are used in a
/// few single scenes.
const PRESET_VAR_BASE: u16 = 30;
const PRESETS: u32 = 3;
const PRESET_SLOTS: u32 = 3;
/// kind, blend, strength, three parameters (as in [`SpriteFilter`]).
const PRESET_FIELDS: u32 = 6;
/// Pictures of static in the noise strip ([`AssetBank::noise`]); the
/// original draws new static every interval, which a short loop stands in
/// for.
const NOISE_FRAMES: u32 = 4;
/// A timed wait (handler 0x475cd0, callback 0x475bf0): arg's low 16 bits
/// are the ms; when its high half isn't 0 a click or skip mode ends it.
const OP_WAIT: u16 = 0x66;
/// 0x76's subs for Trans_Dissolve and Trans_Dot4x4.
const TRANSITION_DISSOLVE: u16 = 26;
const TRANSITION_DOT4X4: u16 = 4;
const TRANSITION_TV_POWER_SWITCH: u16 = 48;
const TRANSITION_SMOOTH_CROSS_FADE: u16 = 36;
/// The `screenfilter` slot of SmoothCrossFade's blur (the game's filters
/// use 1-3).
const SMOOTH_BLUR_SLOT: u32 = 4;

/// Ends an armed transition, and SmoothCrossFade's last third (`tail`):
/// the new picture comes in blurred and sharpens over three quarters of
/// it (filmed: Gaussian σ about 1 to none), then stays.
fn end_transition(ops: &mut Vec<Op>, tail: &mut Option<u32>) {
    ops.push(Op::EndTransition);
    if let Some(ms) = tail.take() {
        let sharpen = ms * 3 / 4;
        ops.push(Op::ScreenFilter { slot: SMOOTH_BLUR_SLOT, filter: None, fade_ms: sharpen });
        ops.push(Op::Wait { ms: ms - sharpen });
    }
}
/// The deepest a bust-up usually sits (0x8e's depth: 10-150, then
/// 1100-5000 for the few drawn over the overlay).
const BUST_UP_DEPTH_MAX: u32 = 1000;
const TV_POWER_SWITCH_MS: u32 = 700;
/// Plays `HD/mvNNNN.bin` (handler 0x478ab0, task 0x408ed0): arg's low 16
/// bits are the number, its high 16 bits whether a click or key ends it
/// (checked at 0x408e5b).
const OP_MOVIE: u16 = 0x97;
/// Fades to black (0x72) or white (0x74; handlers 0x44a360, 0x44a390 →
/// 0x44a2f0 → `CTaskWork_Fade` 0x436000, drawn by 0x435f20): arg's low 16
/// bits are the ms; a high half ≠ 0 fades the other way (from the colour;
/// no script does). Filmed under wine: the colour covers the picture, not
/// the text window and the bars, and stays until the next background.
const OP_FADE_BLACK: u16 = 0x72;
const OP_FADE_WHITE: u16 = 0x74;
const BG_PER_ARCHIVE: usize = 64;
const BU_PER_ARCHIVE: usize = 128;
const SE_FIRST_BANK: usize = 256;
/// Opacity of the text window's base layer, and of the save menu's:
/// measured under wine against the same scene without them, 0.495 on
/// every row of the box's gradient and 0.50 on the save menu's.
const WINDOW_OPACITY_PERCENT: u32 = 50;
/// Assumption: the title screen fades in and out over this long.
const TITLE_FADE_MS: u32 = 1000;
/// A voice id's character is `id / VOICE_CHARACTER`.
const VOICE_CHARACTER: u32 = 100_000;
/// The original's voice switches (the settings dialog's 音声 page, in its
/// order) and the voice characters each covers. 0 and 1 are the minor
/// women and men (フランシス, 紅緒 / 邑西, 宗多, ...); 2 is unused.
const VOICE_TOGGLES: [(&str, &[u32]); 13] = [
    ("紅緒　あずさ", &[3]),
    ("本堂　沙也加", &[4]),
    ("塚本　葉子", &[5]),
    ("伊月　笛子", &[6]),
    ("樋口　章二", &[7]),
    ("塚本　斎", &[8]),
    ("イマ", &[9]),
    ("貴宮　千鳥", &[10]),
    ("南　レイ", &[11]),
    ("シャーリー・磯下", &[12]),
    ("ジーン・ブランシェット", &[13]),
    ("その他の女性陣", &[0]),
    ("その他の男達", &[1]),
];
/// Bust-ups (0x8e) fade in and out over this long, whatever the entry's
/// depth (handler 0x4347b0 starts a 500 ms task, 0x442c70; filmed under
/// wine). Skip mode shows them at once, and so does a record whose
/// entries all have mode bit 1 (0x434530 counts only entries without it as
/// changes; 1,301 records).
const SPRITE_FADE_MS: u32 = 500;

#[derive(Debug, Error)]
pub enum ConvertError {
    #[error("{path}: {source}")]
    Io { path: PathBuf, source: io::Error },
    #[error("{path}: {source}")]
    Gaf { path: PathBuf, source: GafError },
    #[error("{path}: {source}")]
    Xsa { path: PathBuf, source: XsaError },
    #[error("{path} slot {slot}: {source}")]
    Ogg { path: PathBuf, slot: usize, source: crate::ogg::OggError },
    #[error("{path}: {source}")]
    Xarc { path: PathBuf, source: XarcError },
    #[error("{what}: {source}")]
    Image { what: String, source: image::ImageError },
    #[error(transparent)]
    Cd(#[from] CdError),
    #[error("{kind} {number} does not exist")]
    MissingAsset { kind: &'static str, number: usize },
    #[error("converting {path} with ffmpeg: {detail}")]
    Ffmpeg { path: PathBuf, detail: String },
}

/// Reads images and sounds from an installed game and writes the ones that
/// are used into the output directory. Keeps only names and sizes in memory;
/// use [`AssetBank::prefetch`] to read each archive once for a whole game.
pub struct AssetBank {
    game: PathBuf,
    out: PathBuf,
    images: HashMap<(&'static str, usize), Image>,
    written: HashMap<(&'static str, usize), String>,
    used_names: HashSet<String>,
    /// The sound bank read last; voices arrive mostly in bank order.
    last_bank: Option<(PathBuf, Vec<u8>)>,
    /// `HD/fx.arc` once read, and the animations written from it.
    fx_arc: Option<Vec<u8>>,
    animations: HashMap<String, Option<FxAnimation>>,
    /// Write no files (a dry run): the names and sizes only.
    pub dry_run: bool,
}

/// A full-screen animation from `HD/fx.arc`, written as one picture.
#[derive(Debug, Clone)]
pub struct FxAnimation {
    pub image: String,
    pub frame_ms: Vec<u32>,
    pub looped: bool,
}

/// An animation script (`fxNNNN.ap` in `HD/fx.arc`): text statements
/// ending in ";", "#" comments. The name, the offset ("0 0"), loop (1 =
/// start over, else the last frame stays; Aom_Animation_Parts 0x418150),
/// a "default" frame (not used here), the frame count, then one "picture
/// ms flags" line per frame, pictures counted from 1 in the `.004` next
/// to it (the flags are always 0).
struct AnimationScript {
    looped: bool,
    frames: Vec<(usize, u32)>,
}

fn parse_animation_script(text: &str) -> Option<AnimationScript> {
    let mut statements = text
        .lines()
        .map(|l| l.split('#').next().unwrap_or("").trim())
        .filter(|l| !l.is_empty())
        .map(|l| l.trim_end_matches(';').trim());
    let _name = statements.next()?;
    let _offset = statements.next()?;
    let looped = statements.next()? == "1";
    let _default = statements.next()?;
    let count: usize = statements.next()?.parse().ok()?;
    let frames = statements
        .take(count)
        .map(|l| {
            let mut f = l.split_whitespace().map(str::parse::<u32>);
            Some((f.next()?.ok()? as usize, f.next()?.ok()?))
        })
        .collect::<Option<Vec<_>>>()?;
    (frames.len() == count && count > 0 && frames.iter().all(|&(p, _)| p > 0)).then_some(AnimationScript { looped, frames })
}

/// The assets a set of scripts uses (numbers as in the script).
#[derive(Debug, Default)]
pub struct Needs {
    pub backgrounds: BTreeSet<usize>,
    pub sprites: BTreeSet<usize>,
    pub bgm: BTreeSet<usize>,
    pub se: BTreeSet<usize>,
}

impl Needs {
    /// Adds what `cd` refers to. Undecodable records are skipped here; the
    /// lowering reports them.
    pub fn add_script(&mut self, cd: &CdScript) {
        for ins in &cd.code {
            if ins.op == crate::xuse::cd::OP_BACKGROUND && cd.arg(ins) > 0 {
                self.backgrounds.insert(cd.arg(ins) as usize);
                if let Some(frame) = background_frame(cd.sub(ins)) {
                    self.backgrounds.insert(frame);
                }
            }
            if ins.op == OP_OVERLAY {
                let number = pool_u32(cd, cd.arg(ins) as u32 as usize + 6).unwrap_or(0);
                if number > 0 {
                    self.backgrounds.insert(number as usize);
                }
            }
            if let Some(Ok(entries)) = cd.sprites(ins) {
                for e in entries {
                    if e.mode & 1 != 0 {
                        self.sprites.insert(e.sprite as usize);
                    }
                }
            }
            match cd.sound(ins) {
                Some(Ok(Sound::BgmPlay { track, .. } | Sound::BgmCrossfade { to: track, .. })) => {
                    self.bgm.insert(track as usize);
                }
                Some(Ok(Sound::SePlay { sound, .. })) => {
                    self.se.insert(sound as usize);
                }
                _ => {}
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct Image {
    pub path: String,
    pub width: u32,
    pub height: u32,
}

impl AssetBank {
    pub fn new(game: &Path, out: &Path) -> Self {
        Self {
            game: game.to_path_buf(),
            out: out.to_path_buf(),
            images: HashMap::new(),
            written: HashMap::new(),
            used_names: HashSet::new(),
            last_bank: None,
            fx_arc: None,
            animations: HashMap::new(),
            dry_run: false,
        }
    }

    /// Writes `data` to `name` in the output, unless this is a dry run.
    fn write(&self, name: &str, data: &[u8]) -> Result<(), ConvertError> {
        if self.dry_run {
            return Ok(());
        }
        write_file(&self.out.join(name), data)
    }

    /// Background `number` as used by opcode 0x54 (1-based).
    pub fn background(&mut self, number: usize) -> Result<Image, ConvertError> {
        let index = number.checked_sub(1).ok_or(ConvertError::MissingAsset { kind: "background", number })?;
        self.image("bg", "background", "bg", index, BG_PER_ARCHIVE, number)
    }

    /// Sprite `number` as used by opcode 0x8e (1-based: the handler loads
    /// image id 19,999,999 + number, bust-ups being 20,000,000 + index).
    pub fn sprite(&mut self, number: usize) -> Result<Image, ConvertError> {
        let index = number.checked_sub(1).ok_or(ConvertError::MissingAsset { kind: "sprite", number })?;
        self.image("bu", "sprite", "sprite", index, BU_PER_ARCHIVE, number)
    }

    /// Sprite `number` scaled to `sx`, `sy` percent (0x80), as a copy next
    /// to it.
    pub fn sprite_scaled(&mut self, number: usize, sx: u32, sy: u32) -> Result<Image, ConvertError> {
        let base = self.sprite(number)?;
        let (width, height) = ((base.width * sx / 100).max(1), (base.height * sy / 100).max(1));
        let stem = base.path.strip_suffix(".png").unwrap_or(&base.path);
        let name = format!("{stem}_s{sx}x{sy}.png");
        let key = ("scaled sprite", number * 1_000_000 + sx as usize * 1000 + sy as usize);
        if !self.dry_run && !self.written.contains_key(&key) {
            let src = self.out.join(&base.path);
            let img = image::open(&src).map_err(|source| ConvertError::Image { what: base.path.clone(), source })?;
            let scaled = image::imageops::resize(&img.to_rgba8(), width, height, image::imageops::FilterType::Lanczos3);
            self.save_image(&name, &scaled)?;
            self.written.insert(key, name.clone());
        }
        Ok(Image { path: name, width, height })
    }

    /// BGM track (1-based).
    pub fn bgm(&mut self, track: usize) -> Result<String, ConvertError> {
        let slot = track.checked_sub(1).ok_or(ConvertError::MissingAsset { kind: "BGM track", number: track })?;
        self.sound("bm0000.bin", slot, "bgm", format!("bgm{track:02}.ogg"), BGM, track)
    }

    /// Sound effect (1-based, across `k0000.bin` and `k0001.bin`).
    pub fn se(&mut self, number: usize) -> Result<String, ConvertError> {
        let index = number.checked_sub(1).ok_or(ConvertError::MissingAsset { kind: "sound effect", number })?;
        let (bank, slot) = if index < SE_FIRST_BANK {
            ("k0000.bin", index)
        } else {
            ("k0001.bin", index - SE_FIRST_BANK)
        };
        self.sound(bank, slot, "se", format!("se{number:03}.ogg"), "sound effect", number)
    }

    /// Voice clip for a message's voice id. The engine plays clip
    /// `(id - 1) % 100000` (`id / 100000` is the character, for the
    /// per-character voice switch), from `CD/BV{clip >> 8}.xsa`, slot
    /// `clip & 0xff` (handlers 0x41f080 and 0x47f620).
    pub fn voice(&mut self, id: u32) -> Result<String, ConvertError> {
        let number = id as usize;
        let clip = (number.checked_sub(1).ok_or(ConvertError::MissingAsset { kind: "voice", number })?) % 100_000;
        let path = crate::find(&self.game, &format!("CD/bv{:04}.xsa", clip >> 8));
        self.sound_at(path, clip & 0xff, "voice", format!("v{id}.ogg"), "voice", number)
    }

    /// Monotone palette `number` (`pal%04u.pal` in `HD/pal.arc`, a RIFF
    /// PAL of 256 RGB entries for the grey levels) as offsets added to the
    /// grey level (CONscripter's monotone filter). The game's one palette
    /// is sepia, grey + (28, -8, -38) up to rounding; the offsets are the
    /// mean over the grey levels 64-191, where nothing is clamped.
    pub fn palette_offsets(&mut self, number: u32) -> Result<[i32; 3], ConvertError> {
        let missing = || ConvertError::MissingAsset { kind: "palette", number: number as usize };
        let path = crate::find(&self.game, "HD/pal.arc");
        let bytes = fs::read(&path).map_err(|source| ConvertError::Io { path: path.clone(), source })?;
        let arc = Xarc::parse(&bytes).map_err(|source| ConvertError::Xarc { path: path.clone(), source })?;
        let file = format!("pal{number:04}.pal");
        let pal = arc.data(arc.entries.iter().find(|e| e.name == file).ok_or_else(missing)?);
        // "RIFF", size, "PAL data", size, version, count, then r, g, b, flags.
        let entries = pal.get(0x18..0x18 + 256 * 4).filter(|_| pal.starts_with(b"RIFF")).ok_or_else(missing)?;
        let mut sum = [0i32; 3];
        for grey in 64..192 {
            for (k, s) in sum.iter_mut().enumerate() {
                *s += i32::from(entries[grey * 4 + k]) - grey as i32;
            }
        }
        Ok(sum.map(|s| (f64::from(s) / 128.0).round() as i32))
    }

    /// Transition mask `number` from `HD/mask.arc` as a greyscale PNG
    /// (inverted if asked).
    pub fn mask(&mut self, number: usize, inverted: bool) -> Result<String, ConvertError> {
        let kind = if inverted { "inverted mask" } else { "mask" };
        if let Some(p) = self.written.get(&(kind, number)) {
            return Ok(p.clone());
        }
        let path = crate::find(&self.game, "HD/mask.arc");
        let bytes = fs::read(&path).map_err(|source| ConvertError::Io { path: path.clone(), source })?;
        let arc = Xarc::parse(&bytes).map_err(|source| ConvertError::Xarc { path: path.clone(), source })?;
        let file = format!("mask{number:04}.png");
        let entry = arc.entries.iter().find(|e| e.name == file).ok_or(ConvertError::MissingAsset { kind, number })?;
        let mut img = image::load_from_memory_with_format(arc.data(entry), image::ImageFormat::Png)
            .map_err(|source| ConvertError::Image { what: file.clone(), source })?
            .to_luma8();
        if inverted {
            image::imageops::invert(&mut img);
        }
        let name = format!("system/mask{number:04}{}.png", if inverted { "r" } else { "" });
        let mut png = Vec::new();
        img.write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
            .map_err(|source| ConvertError::Image { what: name.clone(), source })?;
        self.write(&name, &png)?;
        self.written.insert((kind, number), name.clone());
        Ok(name)
    }

    /// Static for opcode 0x8c's noise ([`crate::xuse::cd::Noise`]):
    /// [`NOISE_FRAMES`] screens of greyscale noise, one random level per
    /// pixel, side by side in one picture.
    /// A screen filled with one colour (0xRRGGBB), for 0x56.
    pub fn fill(&mut self, rgb: u32) -> Result<String, ConvertError> {
        let name = format!("system/fill_{rgb:06x}.png");
        if self.written.contains_key(&("fill", rgb as usize)) {
            return Ok(name);
        }
        let img = image::RgbImage::from_pixel(800, 600, image::Rgb([(rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8]));
        let mut png = Vec::new();
        img.write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
            .map_err(|source| ConvertError::Image { what: name.clone(), source })?;
        self.write(&name, &png)?;
        self.written.insert(("fill", rgb as usize), name.clone());
        Ok(name)
    }

    /// The rings of a 0x70 Filter-Zigzag as a `screenripple` map: grey =
    /// (d × frequency/100 mod 1024) / 4 for the distance d from the centre,
    /// so that grey/256 is the turn of the kernel's sine (0x43f0b0).
    pub fn zigzag_map(&mut self, cx: i32, cy: i32, frequency: i32) -> Result<String, ConvertError> {
        let name = format!("system/zigzag_{cx}_{cy}_{frequency}.png");
        let key = ("zigzag", (cx as usize & 0xfff) << 24 | (cy as usize & 0xfff) << 12 | (frequency as usize & 0xfff));
        if self.written.contains_key(&key) {
            return Ok(name);
        }
        let img = image::GrayImage::from_fn(800, 600, |x, y| {
            let d = (f64::from(cx - x as i32).powi(2) + f64::from(cy - y as i32).powi(2)).sqrt();
            let turn = (d * f64::from(frequency) / 100.0) as i64;
            image::Luma([(turn.rem_euclid(1024) / 4) as u8])
        });
        let mut png = Vec::new();
        img.write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
            .map_err(|source| ConvertError::Image { what: name.clone(), source })?;
        self.write(&name, &png)?;
        self.written.insert(key, name.clone());
        Ok(name)
    }

    /// A linear gradient mask for a wipe whose edge travels in direction
    /// (`dx`, `dy`): dark where the new picture appears first.
    pub fn wipe_mask(&mut self, dx: i8, dy: i8) -> Result<String, ConvertError> {
        let dir = match (dx, dy) {
            (1, _) => "right",
            (-1, _) => "left",
            (_, 1) => "down",
            _ => "up",
        };
        let name = format!("system/wipe_{dir}.png");
        let key = ("wipe", usize::from(dir.as_bytes()[0]));
        if self.written.contains_key(&key) {
            return Ok(name);
        }
        let img = image::GrayImage::from_fn(800, 600, |x, y| {
            let (pos, len) = if dx != 0 { (x, 800) } else { (y, 600) };
            let along = if dx < 0 || dy < 0 { len - 1 - pos } else { pos };
            image::Luma([(along * 255 / (len - 1)) as u8])
        });
        let mut png = Vec::new();
        img.write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
            .map_err(|source| ConvertError::Image { what: name.clone(), source })?;
        self.write(&name, &png)?;
        self.written.insert(key, name.clone());
        Ok(name)
    }

    /// A 4×4 Bayer matrix tiled over the screen as a mask (16 grey
    /// levels, dark first): an ordered-dither dissolve.
    pub fn dither_mask(&mut self) -> Result<String, ConvertError> {
        const BAYER: [[u8; 4]; 4] = [[0, 8, 2, 10], [12, 4, 14, 6], [3, 11, 1, 9], [15, 7, 13, 5]];
        let name = "system/dither.png".to_string();
        if self.written.contains_key(&("dither", 0)) {
            return Ok(name);
        }
        let img = image::GrayImage::from_fn(800, 600, |x, y| image::Luma([BAYER[y as usize % 4][x as usize % 4] * 16 + 8]));
        let mut png = Vec::new();
        img.write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
            .map_err(|source| ConvertError::Image { what: name.clone(), source })?;
        self.write(&name, &png)?;
        self.written.insert(("dither", 0), name.clone());
        Ok(name)
    }

    pub fn noise(&mut self) -> Result<String, ConvertError> {
        let name = "system/noise.png".to_string();
        if self.written.contains_key(&("noise", 0)) {
            return Ok(name);
        }
        let (w, h) = (800 * NOISE_FRAMES, 600);
        // xorshift32: the same picture on every run.
        let mut state = 0x2545_f491u32;
        let img = image::GrayImage::from_fn(w, h, |_, _| {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            image::Luma([(state >> 24) as u8])
        });
        let mut png = Vec::new();
        img.write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
            .map_err(|source| ConvertError::Image { what: name.clone(), source })?;
        self.write(&name, &png)?;
        self.written.insert(("noise", 0), name.clone());
        Ok(name)
    }

    /// Animation `name` from `HD/fx.arc` (`name.004` pictures played as
    /// `name.ap` says), its frames side by side in playing order, opaque
    /// like the original's (blend 0, Aom_Animation_Parts). `None` if its
    /// script can't be read.
    pub fn animation(&mut self, name: &str) -> Result<Option<FxAnimation>, ConvertError> {
        if let Some(a) = self.animations.get(name) {
            return Ok(a.clone());
        }
        let path = crate::find(&self.game, "HD/fx.arc");
        if self.fx_arc.is_none() {
            self.fx_arc = Some(fs::read(&path).map_err(|source| ConvertError::Io { path: path.clone(), source })?);
        }
        let bytes = self.fx_arc.as_deref().unwrap();
        let arc = Xarc::parse(bytes).map_err(|source| ConvertError::Xarc { path: path.clone(), source })?;
        let script = arc
            .entries
            .iter()
            .find(|e| e.name == format!("{name}.ap"))
            .and_then(|e| parse_animation_script(&encoding_rs::SHIFT_JIS.decode(arc.data(e)).0));
        let animation = match script {
            Some(script) => self.write_animation(&arc, &path, name, &script)?,
            None => None,
        };
        self.animations.insert(name.to_string(), animation.clone());
        Ok(animation)
    }

    fn write_animation(&self, arc: &Xarc, path: &Path, name: &str, script: &AnimationScript) -> Result<Option<FxAnimation>, ConvertError> {
        let file = format!("{name}.004");
        let Some(entry) = arc.entries.iter().find(|e| e.name == file) else { return Ok(None) };
        let gaf_path = path.join(&file);
        let gaf = Gaf::parse(arc.data(entry)).map_err(|source| ConvertError::Gaf { path: gaf_path.clone(), source })?;
        let images = gaf.images().map_err(|source| ConvertError::Gaf { path: gaf_path, source })?;
        let mut pictures = Vec::new();
        for i in &images {
            let img = image::load_from_memory_with_format(i.png, image::ImageFormat::Png)
                .map_err(|source| ConvertError::Image { what: file.clone(), source })?
                .to_rgb8();
            pictures.push(img);
        }
        let Some((w, h)) = pictures.first().map(|p| p.dimensions()) else { return Ok(None) };
        if script.frames.iter().any(|&(p, _)| pictures.get(p - 1).is_none_or(|i| i.dimensions() != (w, h))) {
            return Ok(None);
        }
        let mut strip = image::RgbImage::new(w * script.frames.len() as u32, h);
        for (n, &(p, _)) in script.frames.iter().enumerate() {
            image::imageops::replace(&mut strip, &pictures[p - 1], i64::from(w) * n as i64, 0);
        }
        let image = format!("fx/{name}.png");
        let mut png = Vec::new();
        strip
            .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
            .map_err(|source| ConvertError::Image { what: image.clone(), source })?;
        self.write(&image, &png)?;
        Ok(Some(FxAnimation { image, frame_ms: script.frames.iter().map(|&(_, ms)| ms).collect(), looped: script.looped }))
    }

    /// Text window for a preset: the measured ADV box for style 0, else the
    /// style's `TextWnd{n}.004` graphics at the preset's height, centred.
    ///
    /// Metrics per style are estimates from the window size and the longest
    /// lines / most lines per page the game shows in that style (not yet
    /// checked against screenshots of the original): font size, line gap,
    /// columns, rows.
    pub fn text_window_style(&mut self, preset: WindowPreset) -> Result<TextWindow, ConvertError> {
        if preset.style == 0 {
            return self.text_window();
        }
        // Font size, line gap, columns, rows and the first character's
        // offset from the window: measured under wine, the opening's pages
        // shown in each style (`0002.cd`'s ADV preset patched), against
        // ours on the same lines, to 2-3 px. Characters advance by the font
        // size + 1, so an 18 px pitch is a 17 px font. `None`: estimated
        // (style 6, which the game doesn't use).
        let (font_size, line_gap, columns, rows, offset) = match preset.style {
            1 => (24, 12, 24, 12, Some((21, 24))),
            2 | 7 => (24, 12, 25, 13, Some((48, 35))),
            3 | 4 => (17, 5, 40, 23, Some((41, 26))),
            5 => (17, 11, 36, 16, Some((33, 21))),
            6 => (24, 12, 26, 5, None),
            8 => (20, 13, 36, 15, Some((43, 60))),
            _ => (24, 12, 26, 3, None),
        };
        let file = format!("TextWnd{}.004", preset.style);
        let images = self.wnd_images(&file)?;
        let image = format!("system/textwnd{}.png", preset.style);
        self.layered(&image, &images[0], &images[1])?;
        let (w, _) = png_size(&images[0]);
        let x = (800 - w as i32) / 2;
        // The ADV box draws 3 px below its preset position (measured).
        let y = if preset.style == 6 { preset.y + 3 } else { preset.y };
        let name_plate = if png_size(&images[2]).1 > 1 {
            let plate = format!("system/nameplate{}.png", preset.style);
            self.layered(&plate, &images[2], &images[3])?;
            Some(NamePlate { image: plate, x: x - 5, y: y - 40, text_x: x + 11, text_y: y - 28, font_size: 20 })
        } else {
            None
        };
        let (text_x, text_y) = match offset {
            Some((dx, dy)) => (x + dx, y + dy),
            None => (x + (w as i32 - columns as i32 * (font_size as i32 + 1)) / 2, y + 20),
        };
        Ok(TextWindow { image, x, y, text_x, text_y, columns, rows, font_size, line_gap, vertical: false, name_plate, wait_icon_at: None })
    }

    /// The four PNG layers (message base, frame, name base, frame) of a
    /// `TextWnd*.004` in `HD/wnd.arc`.
    fn wnd_images(&self, file: &str) -> Result<Vec<Vec<u8>>, ConvertError> {
        let images = self.gaf_images("wnd.arc", file, "text window")?;
        if images.len() < 4 {
            return Err(ConvertError::MissingAsset { kind: "text window layer", number: images.len() });
        }
        Ok(images.into_iter().take(4).collect())
    }

    /// The PNGs of GAF4 container `file` inside the XARC archive `HD/{arc}`.
    fn gaf_images(&self, arc: &str, file: &str, kind: &'static str) -> Result<Vec<Vec<u8>>, ConvertError> {
        let path = crate::find(&self.game, &format!("HD/{arc}"));
        let bytes = fs::read(&path).map_err(|source| ConvertError::Io { path: path.clone(), source })?;
        let arc = Xarc::parse(&bytes).map_err(|source| ConvertError::Xarc { path: path.clone(), source })?;
        let entry = arc.entries.iter().find(|e| e.name == file).ok_or(ConvertError::MissingAsset { kind, number: 0 })?;
        let gaf_path = path.join(file);
        let gaf = Gaf::parse(arc.data(entry)).map_err(|source| ConvertError::Gaf { path: gaf_path.clone(), source })?;
        let images = gaf.images().map_err(|source| ConvertError::Gaf { path: gaf_path, source })?;
        Ok(images.iter().map(|i| i.png.to_vec()).collect())
    }

    fn save_image(&self, name: &str, img: &image::RgbaImage) -> Result<(), ConvertError> {
        if self.dry_run {
            return Ok(());
        }
        let dest = self.out.join(name);
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent).map_err(|source| ConvertError::Io { path: parent.to_path_buf(), source })?;
        }
        img.save(&dest).map_err(|source| ConvertError::Image { what: name.to_string(), source })
    }

    /// Builds the ADV text box from the game's own window graphics
    /// (`HD/wnd.arc` → `TextWnd0.004`: message base + frame, name base +
    /// frame). Positions match the original game's 800×600 layout.
    pub fn text_window(&mut self) -> Result<TextWindow, ConvertError> {
        let images = self.wnd_images("TextWnd0.004")?;
        let window = "system/textwnd.png".to_string();
        let plate = "system/nameplate.png".to_string();
        self.layered(&window, &images[0], &images[1])?;
        self.layered(&plate, &images[2], &images[3])?;
        // Positions measured on screenshots of the original game.
        Ok(TextWindow {
            wait_icon_at: None,
            image: window,
            x: 47,
            y: 452,
            text_x: 89,
            text_y: 474,
            columns: 26,
            rows: 3,
            font_size: 24,
            line_gap: 12,
            vertical: false,
            name_plate: Some(NamePlate { image: plate, x: 42, y: 412, text_x: 58, text_y: 424, font_size: 20 }),
        })
    }

    /// Writes `top` composited over `base` (both PNGs of the same size).
    ///
    /// The game alpha-blends the base layer (an opaque grey gradient) over
    /// the scene at 50% opacity ([`WINDOW_OPACITY_PERCENT`]): the box's
    /// light top brightens the scene and its dark middle darkens it.
    fn layered(&self, name: &str, base: &[u8], top: &[u8]) -> Result<(), ConvertError> {
        let load = |png: &[u8]| {
            image::load_from_memory_with_format(png, image::ImageFormat::Png)
                .map(|i| i.to_rgba8())
                .map_err(|source| ConvertError::Image { what: name.to_string(), source })
        };
        self.save_image(name, &blend_layers(load(base)?, &load(top)?))
    }

    /// Reads every archive the `needs` touch once and writes the used
    /// assets. Numbers that don't exist are left for the lowering to report.
    pub fn prefetch(&mut self, needs: &Needs) -> Result<(), ConvertError> {
        let bg: Vec<usize> = needs.backgrounds.iter().filter_map(|n| n.checked_sub(1)).collect();
        self.prefetch_images("bg", "background", "bg", &bg, BG_PER_ARCHIVE, 1)?;
        let bu: Vec<usize> = needs.sprites.iter().filter_map(|n| n.checked_sub(1)).collect();
        self.prefetch_images("bu", "sprite", "sprite", &bu, BU_PER_ARCHIVE, 1)?;
        for &t in &needs.bgm {
            let _ = self.bgm(t);
        }
        for &n in &needs.se {
            let _ = self.se(n);
        }
        Ok(())
    }

    /// `indices` are 0-based; `number_offset` converts back to the script's
    /// numbering (1 for backgrounds and sprites).
    fn prefetch_images(
        &mut self,
        prefix: &str,
        kind: &'static str,
        dir: &str,
        indices: &[usize],
        per_archive: usize,
        number_offset: usize,
    ) -> Result<(), ConvertError> {
        let mut by_archive: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
        for &i in indices {
            by_archive.entry(i / per_archive).or_default().push(i);
        }
        for (archive, wanted) in by_archive {
            let path = crate::find(&self.game, &format!("HD/{prefix}{archive:04}.004"));
            if !path.exists() {
                continue;
            }
            let bytes = fs::read(&path).map_err(|source| ConvertError::Io { path: path.clone(), source })?;
            let gaf = Gaf::parse(&bytes).map_err(|source| ConvertError::Gaf { path: path.clone(), source })?;
            let images = gaf.images().map_err(|source| ConvertError::Gaf { path: path.clone(), source })?;
            for i in wanted {
                if let Some(img) = images.get(i % per_archive) {
                    self.store_image(kind, dir, i + number_offset, img)?;
                }
            }
        }
        Ok(())
    }

    fn store_image(
        &mut self,
        kind: &'static str,
        dir: &str,
        number: usize,
        img: &crate::xuse::gaf::GafImage<'_>,
    ) -> Result<Image, ConvertError> {
        if let Some(done) = self.images.get(&(kind, number)) {
            return Ok(done.clone());
        }
        let (width, height) = png_size(img.png);
        let name = unique(&mut self.used_names, dir, &img.file_name(), number);
        self.write(&name, img.png)?;
        let image = Image { path: name, width, height };
        self.images.insert((kind, number), image.clone());
        Ok(image)
    }

    fn image(
        &mut self,
        prefix: &str,
        kind: &'static str,
        dir: &str,
        index: usize,
        per_archive: usize,
        number: usize,
    ) -> Result<Image, ConvertError> {
        if let Some(done) = self.images.get(&(kind, number)) {
            return Ok(done.clone());
        }
        let path = crate::find(&self.game, &format!("HD/{prefix}{:04}.004", index / per_archive));
        if !path.exists() {
            return Err(ConvertError::MissingAsset { kind, number });
        }
        let bytes = fs::read(&path).map_err(|source| ConvertError::Io { path: path.clone(), source })?;
        let gaf = Gaf::parse(&bytes).map_err(|source| ConvertError::Gaf { path: path.clone(), source })?;
        let images = gaf.images().map_err(|source| ConvertError::Gaf { path: path.clone(), source })?;
        let img = images.get(index % per_archive).ok_or(ConvertError::MissingAsset { kind, number })?;
        self.store_image(kind, dir, number, img)
    }

    fn sound(
        &mut self,
        bank: &str,
        slot: usize,
        dir: &str,
        file_name: String,
        kind: &'static str,
        number: usize,
    ) -> Result<String, ConvertError> {
        let path = crate::find(&self.game, &format!("HD/{bank}"));
        self.sound_at(path, slot, dir, file_name, kind, number)
    }

    fn sound_at(
        &mut self,
        path: PathBuf,
        slot: usize,
        dir: &str,
        file_name: String,
        kind: &'static str,
        number: usize,
    ) -> Result<String, ConvertError> {
        if let Some(p) = self.written.get(&(kind, number)) {
            return Ok(p.clone());
        }
        if self.last_bank.as_ref().is_none_or(|(p, _)| *p != path) {
            let bytes = fs::read(&path).map_err(|source| ConvertError::Io { path: path.clone(), source })?;
            self.last_bank = Some((path.clone(), bytes));
        }
        let bytes = &self.last_bank.as_ref().unwrap().1;
        let xsa = Xsa::parse(bytes).map_err(|source| ConvertError::Xsa { path: path.clone(), source })?;
        let entry = xsa.entries.iter().find(|e| e.slot == slot).ok_or(ConvertError::MissingAsset { kind, number })?;
        let name = format!("{dir}/{file_name}");
        // BGM: param 0 is where the loop starts, in sample frames (the audio
        // before it matches the track's end); the loop ends at the end.
        let loop_start = entry.params[0];
        if kind == BGM && loop_start > 0 {
            let tagged = crate::ogg::set_loop_start(xsa.data(entry), loop_start.into())
                .map_err(|source| ConvertError::Ogg { path: path.clone(), slot, source })?;
            self.write(&name, &tagged)?;
        } else {
            self.write(&name, xsa.data(entry))?;
        }
        self.written.insert((kind, number), name.clone());
        Ok(name)
    }
}

const BGM: &str = "BGM track";
const MOVIE: &str = "movie";
const STAFF_ROLL: &str = "staff roll";
const STAFF_ROLL_FPS: u32 = 30;

impl AssetBank {
    /// Movie `number` (`HD/mvNNNN.bin`, an MPEG-1 program stream) as H.264
    /// and AAC in MP4, which browsers play. Movies stay loose files (the
    /// web build hands the file to a `<video>` element). A movie already
    /// in the output is kept: encoding takes a minute or so each.
    /// Staff roll `number` (0x91's arg) as a movie: every frame drawn from
    /// the exe's tables ([`staffroll`]) at 30 fps, with its song. Kept like
    /// the movies: delete it to draw it again.
    pub fn staff_roll(&mut self, number: usize) -> Result<String, ConvertError> {
        let key = (STAFF_ROLL, number);
        if let Some(p) = self.written.get(&key) {
            return Ok(p.clone());
        }
        let roll = staffroll::ROLLS.get(number).ok_or(ConvertError::MissingAsset { kind: STAFF_ROLL, number })?;
        let song = self.bgm(roll.bgm)?;
        let song = self.out.join(song);
        let name = format!("movie/staffroll{number}.mp4");
        let dest = self.out.join(&name);
        if !self.dry_run && !dest.exists() {
            let pictures = self.gaf_rgba("gd.arc", roll.file, STAFF_ROLL)?;
            let size = format!("{}x{}", staffroll::WIDTH, staffroll::HEIGHT);
            let seconds = format!("{:.3}", f64::from(roll.total_ms) / 1000.0);
            let mut args: Vec<std::ffi::OsString> = ["-f", "rawvideo", "-pix_fmt", "rgb24", "-s", &size, "-r", &STAFF_ROLL_FPS.to_string(), "-i", "-", "-i"].map(Into::into).to_vec();
            args.push(song.into());
            args.extend(["-map", "0:v", "-map", "1:a", "-t", &seconds].map(Into::into));
            args.extend(media::mp4_options(20, "slow").into_iter().map(Into::into));
            let mut frame = image::RgbImage::new(staffroll::WIDTH, staffroll::HEIGHT);
            let frames = u64::from(roll.total_ms) * u64::from(STAFF_ROLL_FPS) / 1000;
            media::ffmpeg(&dest, args, |stdin| {
                (0..frames).try_for_each(|i| {
                    staffroll::render(roll, &pictures, i as f64 * 1000.0 / f64::from(STAFF_ROLL_FPS), &mut frame);
                    stdin.write_all(frame.as_raw())
                })
            })
            .map_err(|detail| ConvertError::Ffmpeg { path: PathBuf::from(roll.file), detail })?;
        }
        self.written.insert(key, name.clone());
        Ok(name)
    }

    pub fn movie(&mut self, number: u16) -> Result<String, ConvertError> {
        let key = (MOVIE, usize::from(number));
        if let Some(p) = self.written.get(&key) {
            return Ok(p.clone());
        }
        let src = crate::find(&self.game, &format!("HD/mv{number:04}.bin"));
        if !src.exists() {
            return Err(ConvertError::MissingAsset { kind: MOVIE, number: key.1 });
        }
        let name = format!("movie/mv{number:04}.mp4");
        let dest = self.out.join(&name);
        if !self.dry_run && !dest.exists() {
            let mut args: Vec<std::ffi::OsString> = vec!["-i".into(), src.clone().into()];
            args.extend(media::mp4_options(22, "slow").into_iter().map(Into::into));
            media::ffmpeg(&dest, args, |_| Ok(())).map_err(|detail| ConvertError::Ffmpeg { path: src.clone(), detail })?;
        }
        self.written.insert(key, name.clone());
        Ok(name)
    }
}

/// Label names from `0.cd`, by (script id, code offset in bytes).
#[derive(Debug, Default)]
pub struct LabelNames(HashMap<(u32, u32), String>);

impl LabelNames {
    pub fn new(table: &SymbolTable) -> Self {
        let mut names = HashMap::new();
        for (script, symbols) in table.scripts.iter().enumerate() {
            for s in symbols {
                names.insert((script as u32, s.code_offset), s.name.clone());
            }
        }
        Self(names)
    }

    /// Name of `cd`'s label-table entry `index` (index 0 is implicit).
    fn table_entry(&self, cd: &CdScript, index: i64) -> Option<&str> {
        let l = cd.labels.get(usize::try_from(index).ok()?.checked_sub(1)?)?;
        self.0.get(&(l.script, l.code_offset)).map(String::as_str)
    }

    /// Name of label number `label` of `cd` itself.
    fn own(&self, cd: &CdScript, label: u32) -> Option<&str> {
        let l = cd.labels.iter().find(|l| l.script == cd.script_id && l.label == label)?;
        self.0.get(&(l.script, l.code_offset)).map(String::as_str)
    }

    /// The converted label of the label named `name` in the script whose
    /// source is `file` (e.g. the scene replay's `rm…` labels).
    pub fn find(&self, scripts: &[(String, CdScript)], file: &str, name: &str) -> Option<String> {
        let (_, cd) = scripts.iter().find(|(_, cd)| cd.name == file)?;
        let l = cd.labels.iter().find(|l| l.script == cd.script_id && self.0.get(&(l.script, l.code_offset)).is_some_and(|n| n == name))?;
        Some(format!("s{}_l{}", l.script, l.label))
    }
}

/// "chapter<route>]?<title>" → title.
fn chapter_title(name: &str) -> Option<&str> {
    // `chapter<route>]?<title>`; two chapters of 0010.cd have a part
    // number before the `?` (`chapterさやか]１?ビジョン`).
    name.strip_prefix("chapter")?.split_once(']')?.1.split_once('?').map(|(_, title)| title)
}

/// "blog?<title>" → title: the scenes links open.
fn scene_title(name: &str) -> Option<&str> {
    name.strip_prefix("blog?")
}

/// How jumps are converted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Flow {
    /// Convert unconditional jumps; conditional ones are left out.
    Jumps,
    /// Convert no jumps at all: every script plays straight through. A
    /// stopgap until conditions are decoded, since a jump without its
    /// condition can loop forever.
    Linear,
}

/// A text window preset subroutine (see [`OP_WINDOW_MODE`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WindowPreset {
    pub full_screen: bool,
    pub style: u32,
    pub x: i32,
    pub y: i32,
    /// Shows messages whole ([`OP_TEXT_DISPLAY`] 1).
    pub instant: bool,
}

/// Subroutines that set up shared state, keyed by IR label name.
#[derive(Debug, Default)]
pub struct Presets {
    /// Text window presets: labels whose code sets a window position.
    pub window: HashMap<String, WindowPreset>,
    /// The window a label is entered with, for labels the code before
    /// can't fall into (after a return or a jump), when every jump, gosub
    /// and link to it with a known window has the same one. The window is
    /// otherwise followed in file order, which carries a footnote scene's
    /// full-screen window (`s10_l97`, `s13_l1`) into the next scene.
    pub entry_window: HashMap<String, WindowPreset>,
    /// The window a subroutine leaves set through the presets it calls
    /// (`s19_l1` calls `s2_l18`), for subroutines that aren't presets.
    pub sub_window: HashMap<String, WindowPreset>,
}

/// Finds the preset subroutines (see [`Presets`]).
pub fn presets(scripts: &[(String, CdScript)]) -> Result<Presets, ConvertError> {
    let mut presets = Presets::default();
    for (_, cd) in scripts {
        let own: HashMap<usize, u32> = cd
            .labels
            .iter()
            .filter(|l| l.script == cd.script_id)
            .map(|l| (l.code_offset as usize / 12 + 1, l.label))
            .collect();
        let (mut label, mut mode, mut style) = (None, 0, 0);
        for (index, ins) in cd.code.iter().enumerate() {
            if let Some(l) = own.get(&index) {
                label = Some(*l);
            }
            let Some(l) = label else { continue };
            let name = format!("s{}_l{l}", cd.script_id);
            match ins.op {
                OP_WINDOW_MODE => mode = cd.arg(ins),
                OP_WINDOW_STYLE => style = cd.arg(ins),
                OP_WINDOW_POS => {
                    let preset = WindowPreset {
                        full_screen: mode == 1,
                        style: style as u32,
                        x: cd.arg(ins) & 0xffff,
                        y: cd.arg(ins) >> 16,
                        instant: false,
                    };
                    presets.window.insert(name, preset);
                }
                OP_TEXT_DISPLAY => {
                    if let Some(p) = presets.window.get_mut(&name) {
                        p.instant = cd.arg(ins) == 1;
                    }
                }
                _ => {}
            }
        }
    }
    presets.sub_window = sub_windows(scripts, &presets.window);
    presets.entry_window = entry_windows(scripts, &presets.window, &presets.sub_window)?;
    Ok(presets)
}

/// The window each label that can't be fallen into is entered with (see
/// [`Presets::entry_window`]). The window at a jump depends on the entry
/// windows found so far, so this repeats until nothing changes.
fn entry_windows(
    scripts: &[(String, CdScript)],
    window: &HashMap<String, WindowPreset>,
    sub_window: &HashMap<String, WindowPreset>,
) -> Result<HashMap<String, WindowPreset>, ConvertError> {
    let mut entry: HashMap<String, WindowPreset> = HashMap::new();
    for _ in 0..10 {
        // The windows of each label's callers; callers whose window isn't
        // known (before a script's first preset) don't count.
        let mut callers: HashMap<String, Vec<WindowPreset>> = HashMap::new();
        for (_, cd) in scripts {
            let own: HashMap<usize, u32> = cd
                .labels
                .iter()
                .filter(|l| l.script == cd.script_id)
                .map(|l| (l.code_offset as usize / 12 + 1, l.label))
                .collect();
            let mut current: Option<WindowPreset> = None;
            for (index, ins) in cd.code.iter().enumerate() {
                if let Some(l) = own.get(&index) {
                    let name = format!("s{}_l{l}", cd.script_id);
                    if !falls_into(cd, index) {
                        current = entry.get(&name).copied().or(current);
                    }
                }
                let target = match ins.op {
                    OP_GOTO | OP_GOTO_2 if own_label_at(cd, cd.arg(ins)).is_some_and(|i| i != index) => {
                        Some(format!("s{}_l{}", cd.script_id, cd.arg(ins)))
                    }
                    OP_GOTO_SUB | OP_IF_EQUAL..=OP_IF_GE => table_label(cd, cd.sub(ins).into()),
                    OP_GOSUB => {
                        let t = table_label(cd, cd.sub(ins).into());
                        if let Some(p) = t.as_ref().and_then(|t| window.get(t)) {
                            current = Some(*p);
                            None
                        } else {
                            if let Some(p) = t.as_ref().and_then(|t| sub_window.get(t)) {
                                current = Some(*p);
                            }
                            t
                        }
                    }
                    _ => match cd.link(ins) {
                        Some(link) => table_label(cd, link?.target.into()),
                        None => None,
                    },
                };
                if let (Some(t), Some(p)) = (target, current) {
                    callers.entry(t).or_default().push(p);
                }
            }
        }
        let found: HashMap<String, WindowPreset> = callers
            .into_iter()
            .filter(|(_, ws)| ws.windows(2).all(|w| w[0] == w[1]))
            .map(|(t, ws)| (t, ws[0]))
            .collect();
        if found == entry {
            break;
        }
        entry = found;
    }
    Ok(entry)
}

/// The window each subroutine leaves set (see [`Presets::sub_window`]):
/// the last preset its code calls, directly or through another
/// subroutine, before its first return.
fn sub_windows(scripts: &[(String, CdScript)], window: &HashMap<String, WindowPreset>) -> HashMap<String, WindowPreset> {
    let mut subs: HashMap<String, WindowPreset> = HashMap::new();
    for _ in 0..10 {
        let mut found = HashMap::new();
        for (_, cd) in scripts {
            for l in cd.labels.iter().filter(|l| l.script == cd.script_id) {
                let name = format!("s{}_l{}", cd.script_id, l.label);
                if window.contains_key(&name) {
                    continue;
                }
                let mut current = None;
                for ins in cd.code.iter().skip(l.code_offset as usize / 12 + 1) {
                    match ins.op {
                        OP_GOSUB => {
                            let t = table_label(cd, cd.sub(ins).into());
                            if let Some(p) = t.as_ref().and_then(|t| window.get(t).or_else(|| subs.get(t))) {
                                current = Some(*p);
                            }
                        }
                        OP_RETURN => break,
                        // A jump: where the code goes on isn't followed.
                        OP_GOTO | OP_GOTO_2 | OP_GOTO_SUB | OP_IF_EQUAL..=OP_IF_GE | OP_LINK_RETURN => {
                            current = None;
                            break;
                        }
                        _ => {}
                    }
                }
                if let Some(p) = current {
                    found.insert(name, p);
                }
            }
        }
        if found == subs {
            break;
        }
        subs = found;
    }
    subs
}

/// Sets the text window of a preset.
fn set_window(ops: &mut Vec<Op>, assets: &mut AssetBank, p: WindowPreset) -> Result<(), ConvertError> {
    ops.push(Op::SetWindow(assets.text_window_style(p)?));
    // setwindow resets the text speed the preset set.
    if p.instant {
        ops.push(Op::InstantText(true));
    }
    Ok(())
}

/// Whether the instruction before the label placed at `index` (after its
/// marker, see [`lower_script`]) can go on into it. Other labels' markers
/// right before it are passed over: when they start the script, nothing
/// falls into it (a script's first labels, `sN_l0`, are entered only by
/// jumps, and only `s4_l0` is ever jumped to).
fn falls_into(cd: &CdScript, index: usize) -> bool {
    let mut at = index;
    loop {
        let Some(before) = at.checked_sub(2).and_then(|i| cd.code.get(i)) else { return false };
        match before.op {
            OP_GOTO | OP_GOTO_2 if own_label_at(cd, cd.arg(before)).is_some_and(|i| i == at - 2) => at -= 1,
            OP_GOTO | OP_GOTO_2 | OP_GOTO_SUB | OP_RETURN | OP_LINK_RETURN | OP_RETURN_TO_ROOT | OP_RETURN_TO_TITLE => return false,
            _ => return true,
        }
    }
}

/// The variable holding `field` of a preset slot (1-based preset and slot).
fn preset_var(preset: u32, slot: u32, field: u32) -> Var {
    let n = ((preset - 1) * PRESET_SLOTS + slot - 1) * PRESET_FIELDS + field;
    Var { global: false, index: PRESET_VAR_BASE + n as u16 }
}

/// A 0x88/0x70 module as CONscripter filter fields (kind, blend,
/// strength, three parameters), or what can't be expressed. `monotone`
/// is a monotone's palette as offsets.
fn module_filter(kind: u32, blend: u16, opacity: u16, data: ModuleData, monotone: Option<[i32; 3]>) -> Result<[i32; 6], String> {
    let (kind, params) = match (kind, data, monotone) {
        (PRESET_KIND_COLOUR, ModuleData::Colour([r, g, b]), _) => (FILTER_COLOUR, [r.into(), g.into(), b.into()]),
        (PRESET_KIND_GREYSCALE, ..) => (FILTER_GREYSCALE, [0; 3]),
        (PRESET_KIND_NEGATIVE, ..) => (FILTER_NEGATIVE, [0; 3]),
        // The amount is on a 0-255 scale: the game goes down to -250.
        (PRESET_KIND_BRIGHTNESS, ModuleData::Brightness(n), _) => (FILTER_BRIGHTNESS, [n.into(), 0, 0]),
        (PRESET_KIND_MONOTONE, _, Some(offsets)) => (FILTER_MONOTONE, offsets),
        // The exe clamps the ranges to 6.
        (PRESET_KIND_SMOOTH, ModuleData::Smooth(x, y), _) => (FILTER_BLUR, [x.min(6).into(), y.min(6).into(), 0]),
        (PRESET_KIND_HUE_SATURATION, ModuleData::HueSaturation(hue, saturation), _) => {
            (FILTER_HUE_SATURATION, [hue.into(), saturation.into(), 0])
        }
        (PRESET_KIND_BRIGHTNESS_INVERT, ..) => (FILTER_LIGHTNESS_INVERT, [0; 3]),
        _ => return Err(format!("filter kind {kind}")),
    };
    let blend = match blend {
        PRESET_BLEND_ALPHA | PRESET_BLEND_COPY => FILTER_BLEND_COPY,
        PRESET_BLEND_MULTI => FILTER_BLEND_MULTIPLY,
        PRESET_BLEND_ADD => FILTER_BLEND_ADD,
        PRESET_BLEND_SCREEN => FILTER_BLEND_SCREEN,
        _ => return Err(format!("blend {blend}")),
    };
    Ok([kind, blend, i32::from(opacity.min(256)), params[0], params[1], params[2]])
}

/// Slots of CONscripter's `screenfilter` (the game uses 1-3).
const SCREEN_FILTER_SLOTS: u32 = 4;

/// The `screenripple` slot of a 0x70 Filter-Replace in filter slot n
/// (0x8c, 0x6f and 0x6d use 1-4).
const REPLACE_SLOT_BASE: u32 = 4;

/// The interval of a still `screenripple` (it never changes; each step
/// redraws the screen).
const REPLACE_INTERVAL_MS: u32 = 1000;

/// Opcode 0x70: sets one slot of the screen filter. A Filter-Replace
/// becomes a still `screenripple` in slot 4 + n; `replaces` holds the
/// filter slots that hold one, in file order, so the next module in the
/// slot clears it. A module the converter can't express is reported and
/// leaves the slot as it was.
fn lower_screen_module(
    m: &ScreenModule,
    replaces: &mut BTreeSet<u32>,
    assets: &mut AssetBank,
    ops: &mut Vec<Op>,
    at: impl Fn() -> SourceLoc,
) -> Result<(), ConvertError> {
    if m.slot > SCREEN_FILTER_SLOTS {
        ops.push(Op::Unsupported { what: format!("opcode 0x70 slot {}", m.slot), at: at() });
        return Ok(());
    }
    let clear_replace = |slot: u32, replaces: &mut BTreeSet<u32>, ops: &mut Vec<Op>| {
        if replaces.remove(&slot) {
            ops.push(Op::ScreenEffect { slot: REPLACE_SLOT_BASE + slot, effect: None });
        }
    };
    if m.slot == 0 {
        for slot in std::mem::take(replaces) {
            ops.push(Op::ScreenEffect { slot: REPLACE_SLOT_BASE + slot, effect: None });
        }
    } else if m.kind == 0 || !matches!(m.kind, PRESET_KIND_REPLACE | PRESET_KIND_ZIGZAG) {
        clear_replace(m.slot, replaces, ops);
    }
    if m.slot == 0 || m.kind == 0 {
        ops.push(Op::ScreenFilter { slot: m.slot, filter: None, fade_ms: 0 });
        return Ok(());
    }
    // The exe's sine index (1024 a turn) starts at (v << 16) / 100 >> 6, in
    // 1/1000 turn for screenripple.
    let phase = |v: i32| (((((i64::from(v) << 16) / 100) >> 6 & 0x3ff) * 1000 + 512) / 1024) as i32;
    let displacement = match m.data {
        // Starts at the begin angle (1/100 turn, 0x41d390).
        ModuleData::Replace(r) => Some((assets.mask(r.map as usize, false)?, r.distance, r.direction, phase(r.begin_angle))),
        // The zigzag's rings as a map (grey = the kernel's table index / 4);
        // its Multi blend multiplies the picture by the shifted copy
        // (screenripple's blend 1). The exe (0x41dab0) passes the frequency
        // as the phase too and never reads the begin angle or the reduction.
        ModuleData::Zigzag(z) => {
            Some((assets.zigzag_map(z.centre_x, z.centre_y, z.frequency)?, z.distance, z.direction, phase(z.frequency)))
        }
        _ => None,
    };
    if let Some((map, distance, direction, phase)) = displacement {
        let multi_zigzag = matches!(m.data, ModuleData::Zigzag(_)) && m.blend == PRESET_BLEND_MULTI;
        if !matches!(m.blend, PRESET_BLEND_ALPHA | PRESET_BLEND_COPY) && !multi_zigzag {
            ops.push(Op::Unsupported { what: format!("opcode 0x70 kind {} blend {}", m.kind, m.blend), at: at() });
            return Ok(());
        }
        let opacity = i32::from(m.opacity.min(256));
        let effect = ScreenEffect::Ripple {
            map,
            time_ms: 0,
            interval_ms: REPLACE_INTERVAL_MS,
            opacity: (opacity, opacity),
            amplitude: (distance, distance),
            angle: direction,
            angle_step: 0,
            phase_step: 0,
            multiply: multi_zigzag,
            phase,
        };
        if !replaces.contains(&m.slot) {
            ops.push(Op::ScreenFilter { slot: m.slot, filter: None, fade_ms: 0 });
        }
        replaces.insert(m.slot);
        ops.push(Op::ScreenEffect { slot: REPLACE_SLOT_BASE + m.slot, effect: Some(effect) });
        return Ok(());
    }
    let monotone = match m.data {
        ModuleData::Palette(p) => match assets.palette_offsets(p) {
            Ok(offsets) => Some(offsets),
            Err(ConvertError::MissingAsset { .. }) => None,
            Err(e) => return Err(e),
        },
        _ => None,
    };
    match module_filter(m.kind, m.blend, m.opacity, m.data, monotone) {
        Ok([kind, blend, strength, p0, p1, p2]) => {
            let filter = SpriteFilter {
                kind: Operand::Const(kind),
                blend: Operand::Const(blend),
                strength: Operand::Const(strength),
                params: [p0, p1, p2].map(Operand::Const),
            };
            ops.push(Op::ScreenFilter { slot: m.slot, filter: Some(filter), fade_ms: 0 });
        }
        Err(what) => ops.push(Op::Unsupported { what: format!("opcode 0x70 {what}"), at: at() }),
    }
    Ok(())
}

/// Opcode 0x8c: an effect over the whole screen: static (kind 2) as a
/// sprite, a ripple (4) and shaking (5) as CONscripter's `screenripple`
/// and `screenshake`. `slots` holds the kind each slot was last set to in
/// file order, so clearing a slot removes the right thing.
fn lower_environment(
    m: &EnvModule,
    slots: &mut BTreeMap<u32, u32>,
    assets: &mut AssetBank,
    ops: &mut Vec<Op>,
    at: impl Fn() -> SourceLoc,
) -> Result<(), ConvertError> {
    let clear = |slot: u32, kind: u32, ops: &mut Vec<Op>| match kind {
        ENV_KIND_NOISE => ops.push(Op::Noise(None)),
        _ => ops.push(Op::ScreenEffect { slot, effect: None }),
    };
    if m.slot == 0 {
        let cleared = std::mem::take(slots);
        if cleared.values().any(|&k| k == ENV_KIND_NOISE) {
            clear(0, ENV_KIND_NOISE, ops);
        }
        if cleared.values().any(|&k| k != ENV_KIND_NOISE) {
            clear(0, 0, ops);
        }
        return Ok(());
    }
    // Setting a slot replaces what it held.
    if let Some(kind) = slots.remove(&m.slot)
        && (m.kind == 0 || kind != m.kind)
    {
        clear(m.slot, kind, ops);
    }
    let set = match (m.noise, m.ripple, m.shake) {
        // The game always uses alpha blending (0) and greyscale (mode 1),
        // and its opacity always rises from 0, over up to 2 s.
        (Some(n), ..) if n.blend == 0 && n.mode & 1 == 1 => {
            let image = assets.noise()?;
            let frame_ms = n.interval_ms.max(1);
            if n.time_ms > 0 && n.begin_opacity < n.end_opacity {
                ops.push(Op::NoiseFadeIn { ms: n.time_ms });
            }
            ops.push(Op::Noise(Some(Noise { image, frames: NOISE_FRAMES, frame_ms, opacity: u32::from(n.end_opacity.min(256)) })));
            true
        }
        // Option 0 (the only one used): a fixed direction, not a centre.
        (_, Some(r), _) if r.option == 0 && r.blend == 0 => {
            let map = assets.mask(r.map as usize, false)?;
            let effect = ScreenEffect::Ripple {
                map,
                time_ms: r.time_ms,
                interval_ms: r.interval_ms,
                opacity: (i32::from(r.begin_opacity), i32::from(r.end_opacity)),
                amplitude: (r.begin_amplitude, r.end_amplitude),
                angle: r.angle,
                angle_step: r.angle_step,
                phase_step: r.phase_step,
                multiply: false,
                phase: 0,
            };
            ops.push(Op::ScreenEffect { slot: m.slot, effect: Some(effect) });
            true
        }
        (.., Some(s)) => {
            let effect = ScreenEffect::Shake {
                time_ms: s.time_ms,
                interval_ms: s.interval_ms,
                path: s.option & 1 == 1,
                speed: s.speed,
                range: [s.begin_range, s.end_range],
                end_ms: 0,
            };
            ops.push(Op::ScreenEffect { slot: m.slot, effect: Some(effect) });
            true
        }
        _ if m.kind == 0 => false,
        _ => {
            ops.push(Op::Unsupported { what: format!("opcode 0x8c kind {}", m.kind), at: at() });
            false
        }
    };
    if set {
        slots.insert(m.slot, m.kind);
    }
    Ok(())
}

/// The `screenshake` slot for opcode 0x6d's quakes (0x8c uses 1 and 2).
const QUAKE_SLOT: u32 = 4;

/// Opcode 0x6d: a quake, as `screenshake` along the 7-point path, waited
/// for with a `delay` (a click ends both, as in the game) and cleared.
/// Assumption: the game holds the full range for 4 points, then shrinks
/// it; here it shrinks from the start, over the same total time, and both
/// axes reach 0 together.
fn lower_quake(q: &Quake, ops: &mut Vec<Op>) {
    let (x, y) = (q.range.0.min(1000) as i32, q.range.1.min(1000) as i32);
    if q.speed == 0 || (x == 0 && y == 0) {
        return;
    }
    let shrink = (q.speed / 4).max(50); // pixels a second
    // Filmed under wine (speed 80): the picture jumps to the next point
    // every frame (30 a second), at the full range for the first points,
    // then the range shrinks: two shakes, the second shrinking to 0.
    let points = q.speed.min(QUAKE_POINTS_PER_S);
    let hold_ms = 4000 / points;
    let shrink_ms = x.max(y) as u32 * 1000 / shrink;
    let speed = (i64::from(points) * 65536 / 1000) as i32;
    for (time_ms, to) in [(hold_ms, (x, y)), (shrink_ms, (0, 0))] {
        let effect = ScreenEffect::Shake { time_ms, interval_ms: 10, path: true, speed: (speed, 0), range: [(x, y), to], end_ms: 0 };
        ops.push(Op::ScreenEffect { slot: QUAKE_SLOT, effect: Some(effect) });
        ops.push(Op::Wait { ms: time_ms });
    }
    ops.push(Op::ScreenEffect { slot: QUAKE_SLOT, effect: None });
}

/// A quake's points a second at most: one a frame at the original's 30.
const QUAKE_POINTS_PER_S: u32 = 30;

/// The `screenpulse` slot for opcode 0x6f's pulses.
const PULSE_SLOT: u32 = 3;

/// Opcode 0x6f: a pulse, as `screenpulse` waited for with a `delay` (a
/// click ends both) and cleared. The tints: blend 2 (Multi) is `tint` 1;
/// blend 6 (white, presets 6-8) is assumed to lay the colour over the copy.
fn lower_surprise(s: &Surprise, ops: &mut Vec<Op>) {
    let p = SURPRISE_PRESETS[s.preset];
    let zoom = |v: i32| ((i64::from(v) * 1000 + 32768) >> 16) as i32;
    let c = p.tint;
    let colour = (c & 0xff) << 16 | (c & 0xff00) | (c >> 16 & 0xff);
    let (tint, tint_opacity) = match p.tint_blend {
        _ if p.tint_opacity == 0 => (0, 0),
        2 => (1, p.tint_opacity),
        _ => (2, p.tint_opacity),
    };
    let effect = ScreenEffect::Pulse {
        time_ms: s.time_ms,
        interval_ms: 10,
        zoom: (zoom(p.zoom.0), zoom(p.zoom.1)),
        opacity: p.opacity,
        colour,
        tint_opacity,
        tint,
    };
    ops.push(Op::ScreenEffect { slot: PULSE_SLOT, effect: Some(effect) });
    ops.push(Op::Wait { ms: s.time_ms });
    ops.push(Op::ScreenEffect { slot: PULSE_SLOT, effect: None });
}

/// Opcode 0x88: sets one slot of a preset (slot 0 clears every slot). A
/// module the converter can't express clears its slot and is reported.
fn lower_preset_module(m: &PresetModule, ops: &mut Vec<Op>, at: impl Fn() -> SourceLoc) {
    let unsupported = |what: String| Op::Unsupported { what, at: at() };
    if !(1..=PRESETS).contains(&m.preset) || m.slot > PRESET_SLOTS {
        ops.push(unsupported(format!("opcode 0x88 preset {} slot {}", m.preset, m.slot)));
        return;
    }
    let set = |slot: u32, field: u32, n: i32| Op::Arith {
        var: preset_var(m.preset, slot, field),
        op: ArithOp::Set,
        value: Operand::Const(n),
    };
    if m.slot == 0 {
        ops.extend((1..=PRESET_SLOTS).map(|slot| set(slot, 0, 0)));
        return;
    }
    if m.kind == 0 {
        ops.push(set(m.slot, 0, 0));
        return;
    }
    match module_filter(m.kind, m.blend, m.opacity, m.data, None) {
        Ok(fields) => ops.extend(fields.into_iter().enumerate().map(|(field, n)| set(m.slot, field as u32, n))),
        Err(what) => {
            ops.push(unsupported(format!("opcode 0x88 {what}")));
            ops.push(set(m.slot, 0, 0));
        }
    }
}

/// Opcode 0x90: the filters of a preset (0 = none), read from its
/// variables when the script runs.
fn preset_filters(preset: u32) -> Option<Vec<SpriteFilter>> {
    match preset {
        0 => Some(Vec::new()),
        1..=PRESETS => Some(
            (1..=PRESET_SLOTS)
                .map(|slot| {
                    let v = |field| Operand::Var(preset_var(preset, slot, field));
                    SpriteFilter { kind: v(0), blend: v(1), strength: v(2), params: [v(3), v(4), v(5)] }
                })
                .collect(),
        ),
        _ => None,
    }
}

/// An opcode 0x86 record ([`OP_MODULE`]).
#[derive(Debug)]
struct Motion {
    layer: u8,
    depth: u32,
    flags: u32,
    /// The second key frame of a two-frame key-frame motion whose position
    /// moves linearly; `None` for a removal and for other motions.
    end: Option<MotionEnd>,
}

#[derive(Debug)]
struct MotionEnd {
    ms: u32,
    /// The position moves linearly (else `x`, `y` mean nothing).
    position: bool,
    x: i32,
    y: i32,
    rotation: i32,
    /// The opacity at the end, if the motion has it.
    opacity: Option<u16>,
}

fn motion(cd: &CdScript, o: usize) -> Option<Motion> {
    let u32_at = |at: usize| pool_u32(cd, at);
    let byte = |at: usize| cd.bytes(at, 1).map(|b| b[0]);
    let layer = byte(o)?;
    let depth = u32_at(o + 1)?;
    let flags = u32_at(o + 9)?;
    let mut motion = Motion { layer, depth, flags, end: None };
    if flags & 0x8000_0000 != 0 {
        return Some(motion);
    }
    let count = u16::from_le_bytes(cd.bytes(o + 13, 2)?.try_into().unwrap());
    // Each field group's size in bytes, by flag (see OP_MODULE).
    let groups = [(0x10, 1), (0x20, 9), (0x40, 9), (0x80, 9), (0x100, 5), (0x200, 3)];
    let frame_size = 4 + groups.iter().filter(|(f, _)| flags & f != 0).map(|(_, n)| n).sum::<usize>();
    if count != 2 || flags & (MOTION_POSITION | MOTION_OPACITY) == 0 {
        return Some(motion);
    }
    let mut at = o + 15 + frame_size;
    let ms = u32_at(at)?;
    at += 4;
    let mut end = MotionEnd { ms, position: false, x: 0, y: 0, rotation: 0, opacity: None };
    for (flag, size) in groups.into_iter().filter(|(f, _)| flags & f != 0) {
        match flag {
            MOTION_POSITION => {
                (end.x, end.y) = (u32_at(at)? as i32, u32_at(at + 4)? as i32);
                end.position = byte(at + 8)? == 2;
            }
            // Only a linear turn (u8 2, as all the game's) is drawn turning.
            0x100 if byte(at + 4)? == 2 => end.rotation = u32_at(at)? as i32,
            MOTION_OPACITY if byte(at + 2)? == 2 => end.opacity = Some(u16::from_le_bytes(cd.bytes(at, 2)?.try_into().unwrap())),
            _ => {}
        }
        at += size;
    }
    if end.position || end.opacity.is_some() {
        motion.end = Some(end);
    }
    Some(motion)
}

/// A layer 2 object made by 0x7a ([`OP_PICTURE`]), in file order.
#[derive(Debug, Default)]
struct Picture {
    x: i32,
    y: i32,
    /// 0x7d's horizontal and vertical anchor (0-2).
    anchor: (u8, u8),
    /// Bust-up number (as in 0x8e).
    sprite: Option<u32>,
    visible: bool,
}

impl Picture {
    /// The sprite change that shows the object as it now is.
    fn change(&self, assets: &mut AssetBank, layer: u32) -> Result<Option<SpriteChange>, ConvertError> {
        let Some(number) = self.sprite.filter(|_| self.visible) else { return Ok(None) };
        let img = assets.sprite(number as usize)?;
        let sprite = SpritePlacement {
            image: img.path,
            x: self.x - img.width as i32 * i32::from(self.anchor.0) / 2,
            y: self.y - img.height as i32 * i32::from(self.anchor.1) / 2,
        };
        Ok(Some(SpriteChange { layer, sprite: Some(sprite), keep_filters: false }))
    }
}

/// Whether bust-up `number`, turned by the motion's end rotation (tenths of
/// a degree) about its end centre, lies wholly off the 800×600 screen.
fn turned_off_screen(assets: &mut AssetBank, number: u16, end: &MotionEnd) -> bool {
    let Ok(img) = assets.sprite(number.into()) else { return false };
    let a = f64::from(end.rotation).to_radians() / 10.0;
    let (w, h) = (f64::from(img.width), f64::from(img.height));
    let hw = (w * a.cos().abs() + h * a.sin().abs()) / 2.0;
    let hh = (w * a.sin().abs() + h * a.cos().abs()) / 2.0;
    let (x, y) = (f64::from(end.x), f64::from(end.y));
    x - hw >= 800.0 || x + hw <= 0.0 || y - hh >= 600.0 || y + hh <= 0.0
}

/// Where a bust-up entry's image goes on screen.
fn placement(assets: &mut AssetBank, e: &SpriteEntry) -> Result<SpritePlacement, ConvertError> {
    let img = assets.sprite(e.sprite as usize)?;
    // Assumption: (x, y) is the sprite's centre on the 800×600 screen.
    Ok(SpritePlacement {
        image: img.path,
        x: i32::from(e.x) - img.width as i32 / 2,
        y: i32::from(e.y) - img.height as i32 / 2,
    })
}

/// Lowers one script. Instructions that can't be expressed yet become
/// [`Op::Unsupported`]; assets are written through `assets`.
pub fn lower_script(
    cd: &CdScript,
    file: &str,
    flow: Flow,
    assets: &mut AssetBank,
    presets: &Presets,
    names: &LabelNames,
) -> Result<Vec<Op>, ConvertError> {
    // A label's offset points at a marker instruction (a 0x10 that targets
    // itself); a jump lands just after it, so the label goes after that
    // instruction.
    let own_labels: HashMap<usize, u32> = cd
        .labels
        .iter()
        .filter(|l| l.script == cd.script_id)
        .map(|l| (l.code_offset as usize / 12 + 1, l.label))
        .collect();

    let mut ops = Vec::new();
    // Links, ruby, styles and indent records apply to the next message.
    let mut pending = MessageRecords::default();
    // A transition armed by 0x76 and not yet waited on (file order).
    let mut transition_armed = false;
    // The text window (file order): its rows and columns and whether it is
    // full-screen (where messages join onto a page while they fit); the
    // index of the last text op and the rows on its page, and whether a
    // "p" tag followed it.
    let mut window_size: Option<(usize, usize, bool)> = None;
    // The last window preset set (file order).
    let mut window_preset: Option<WindowPreset> = None;
    let mut last_text: Option<(usize, usize)> = None;
    let mut page_break = false;
    // Opcode 0x8c slots and the kind each holds (file order).
    let mut env_slots = BTreeMap::new();
    let mut replace_slots = BTreeSet::new();
    // The time of the module each 0x8c slot holds, for 0x8d.
    let mut env_times: BTreeMap<u32, u32> = BTreeMap::new();
    // Each character's bust-up (number, centre) and 0x82 opacity (file
    // order), for 0x80.
    let mut placed: HashMap<u32, (u16, i32, i32)> = HashMap::new();
    let mut opacities: HashMap<u32, u32> = HashMap::new();
    // The fx object: its opacity once 0x7a made it, and the animation
    // 0x8b loaded into it (file order).
    let mut fx_opacity: Option<u32> = None;
    let mut fx_frames: Option<(Vec<u32>, bool)> = None;
    // Layer 2 objects 0x7a made, by depth (= character layer).
    let mut pictures: HashMap<u32, Picture> = HashMap::new();
    // 0x56 fills (file order): the background's colour, whether the
    // overlay is a fill, and the time of a fade that the next wait covers.
    let mut background_fill: Option<u32> = None;
    // A slot-1 fill on layer 3 or 4 (0x20/0x30 bands, over the bust-ups and
    // the static): its colour, drawn as a screen filter.
    let mut top_fill: Option<u32> = None;
    let mut overlay_fill = false;
    // The depth of the overlay over the sprites, while one is up, and the
    // bust-up layers drawn in front of it (Op::SpriteInFront).
    let mut overlay_depth: Option<u32> = None;
    let mut in_front: HashSet<u32> = HashSet::new();
    // SmoothCrossFade's last third: a wait after the transition.
    let mut smooth_tail: Option<u32> = None;
    // Where the armed 0x76 transition starts in `ops` when its key (the
    // third u32: layer byte, depth << 8) is below the top band: it covers
    // only the objects sorting under it.
    let mut keyed_start: Option<usize> = None;
    let mut wait_credit_ms = 0;
    // 0x4a's style for whole messages (file order): colour, size.
    let mut default_style: (Option<u32>, Option<u16>) = (None, None);
    // Record runs that jumps enter part-way, by first record: the message
    // index and one variant per path (see [`unfold_records`]).
    let mut unfolded = if flow == Flow::Jumps { unfold_records(cd, &own_labels, assets, names)? } else { HashMap::new() };
    let mut variants: Option<(usize, usize, Vec<RecordPath>)> = None;
    let mut unfold_id = 0;
    for (index, ins) in cd.code.iter().enumerate() {
        if let Some((message, paths)) = unfolded.remove(&index) {
            variants = Some((index, message, paths));
        }
        let in_run = variants.as_ref().is_some_and(|(start, message, _)| (*start..=*message).contains(&index));
        // Labels inside a run are emitted with its paths.
        let inside_run = variants.as_ref().is_some_and(|(start, message, _)| (*start + 1..=*message).contains(&index));
        if let Some(label) = own_labels.get(&index).filter(|_| !inside_run) {
            let name = format!("s{}_l{label}", cd.script_id);
            let entered = presets.entry_window.get(&name).copied().filter(|_| !falls_into(cd, index));
            ops.push(Op::Label(name));
            if let Some(p) = entered.filter(|p| window_preset != Some(*p)) {
                set_window(&mut ops, assets, p)?;
                window_preset = Some(p);
                window_size = Some(window_size_of(assets, p)?);
                last_text = None;
            }
        }
        if in_run && cd.message(ins).is_none() {
            continue; // emitted with the message, path by path
        }
        if add_record(cd, ins, &mut pending)? {
            continue;
        }
        let at = || SourceLoc { file: file.to_string(), index };
        if matches!(ins.op, OP_GOTO | OP_GOTO_2) && own_label_at(cd, cd.arg(ins)).is_some_and(|i| i == index) {
            continue; // label marker: a jump to the label sitting on this instruction
        }
        if flow == Flow::Linear && matches!(ins.op, OP_GOTO | OP_GOTO_2 | OP_GOTO_SUB | OP_IF_EQUAL..=OP_IF_GE | OP_GOSUB | OP_RETURN | OP_LINK_RETURN) {
            ops.push(Op::Unsupported { what: format!("opcode {:#04x} (linear mode)", ins.op), at: at() });
            continue;
        }
        if ins.op == OP_MASK_TRANSITION {
            let o = cd.arg(ins) as u32 as usize;
            match (pool_u32(cd, o), pool_u32(cd, o + 4), pool_u32(cd, o + 8)) {
                (Some(number), Some(soft), Some(fade_ms)) => {
                    // Assumption: negative softness runs the mask bright-first.
                    let mask = match assets.mask(number as usize, (soft as i32) < 0) {
                        Ok(m) => Some(m),
                        Err(ConvertError::MissingAsset { .. }) => None,
                        Err(e) => return Err(e),
                    };
                    transition_armed = true;
                    keyed_start = None;
                    let effect = format!("MaskBlend2 mask {number} softness {}", soft as i32);
                    ops.push(Op::BeginTransition { effect, fade_ms, mask, style: TransitionStyle::Fade });
                }
                _ => ops.push(Op::Unsupported { what: format!("opcode {:#04x} (bad record)", ins.op), at: at() }),
            }
            continue;
        }
        if ins.op == OP_OVERLAY {
            let o = cd.arg(ins) as u32 as usize;
            let record = cd.bytes(o, 2).zip(pool_u32(cd, o + 6));
            match record {
                Some((&[2, layer @ 0..=2], number)) => {
                    let image = match number {
                        0 => None,
                        n => Some(assets.background(n as usize)?.path),
                    };
                    let over = overlay_over(layer, pool_u32(cd, o + 2).unwrap_or(0));
                    overlay_depth = over.filter(|_| image.is_some());
                    ops.push(Op::Overlay { image, over_sprites: over.is_some() });
                    if cd.bytes(o + 10, 1).is_some_and(|mark| mark[0] != 0) && let Some(number) = gallery_cg(number as i32) {
                        ops.push(Op::CgSeen { number });
                    }
                }
                _ => ops.push(Op::Unsupported { what: format!("opcode {:#04x} record {record:?}", ins.op), at: at() }),
            }
            continue;
        }
        if ins.op == OP_FILL {
            let o = cd.arg(ins) as u32 as usize;
            let record = cd.bytes(o, 2).zip(pool_u32(cd, o + 6));
            let rgb = |c: u32| (c & 0xff) << 16 | (c & 0xff00) | (c >> 16 & 0xff);
            match record {
                // Replaces the background (slot 1) and lies over everything
                // else. Screen filter slot 4: no 0x70 filter is up at the
                // one use (0006.cd).
                Some((&[1, 3 | 4], colour)) => {
                    top_fill = Some(rgb(colour));
                    ops.push(Op::Background { image: assets.fill(0)?, fade_ms: 0 });
                    ops.push(Op::ScreenFilter { slot: TOP_FILL_FILTER_SLOT, filter: Some(colour_filter(rgb(colour), 256)), fade_ms: 0 });
                }
                Some((&[1, _], colour)) => {
                    background_fill = Some(rgb(colour));
                    ops.push(Op::Background { image: assets.fill(rgb(colour))?, fade_ms: 0 });
                }
                Some((&[2, layer @ 0..=2], colour)) => {
                    overlay_fill = true;
                    let over = overlay_over(layer, pool_u32(cd, o + 2).unwrap_or(0));
                    overlay_depth = over;
                    ops.push(Op::Overlay { image: Some(assets.fill(rgb(colour))?), over_sprites: over.is_some() });
                }
                _ => ops.push(Op::Unsupported { what: format!("opcode {:#04x} record {record:?}", ins.op), at: at() }),
            }
            continue;
        }
        if ins.op == OP_BLEND && overlay_fill && cd.bytes(cd.arg(ins) as u32 as usize, 5) == Some(&[1, 2, 0, 0, 0]) {
            continue; // dissolve: the fade-out is drawn as a cross-fade
        }
        if ins.op == OP_RETURN_TO_TITLE {
            ops.push(Op::ReturnToTitle);
            continue;
        }
        if ins.op == OP_CLEAR_CHAPTER_HISTORY {
            env_slots.clear();
            ops.push(Op::Noise(None));
            ops.push(Op::ScreenEffect { slot: 0, effect: None });
            ops.push(Op::ChapterHistoryClear);
            continue;
        }
        if matches!(ins.op, OP_YIELD | OP_YIELD_2 | OP_ENTRY_NAME) {
            continue;
        }
        // sound_request leaves out a sound that doesn't loop while skipping.
        if let Some(Ok(Sound::SePlay { looped: false, .. })) = cd.sound(ins) {
            ops.push(Op::SeNotSkipped);
        }
        // op_5a passes the record's first u32 to the sound request as its
        // fade-in (0 while skipping).
        if let Some(Ok(Sound::SePlay { fade_in_ms, .. })) = cd.sound(ins)
            && fade_in_ms > 0
        {
            ops.push(Op::SeFadeIn { ms: fade_in_ms });
        }
        if ins.op == crate::xuse::cd::OP_BACKGROUND && cd.arg(ins) > 0 {
            // op_54 first removes background slots 1-10 (0x4132d0), the
            // overlay (0x55, slot 2) with them.
            overlay_depth = None;
            ops.push(Op::Overlay { image: None, over_sprites: false });
            // It starts no fade: the picture shows at once, or with the
            // transition armed around it.
            ops.push(Op::Background { image: assets.background(cd.arg(ins) as usize)?.path, fade_ms: 0 });
            if let Some(frame) = background_frame(cd.sub(ins)) {
                ops.push(Op::Overlay { image: Some(assets.background(frame)?.path), over_sprites: false });
            }
        }
        if ins.op == crate::xuse::cd::OP_BACKGROUND {
            background_fill = None;
            if top_fill.take().is_some() {
                ops.push(Op::ScreenFilter { slot: TOP_FILL_FILTER_SLOT, filter: None, fade_ms: 0 });
            }
            // 0x54 marks its picture as seen (0x4222c0's last argument).
            if let Some(number) = gallery_cg(cd.arg(ins)) {
                ops.push(Op::CgSeen { number });
            }
            continue;
        }
        if ins.op == OP_OPACITY && let Some(fill) = top_fill {
            let o = cd.arg(ins) as u32 as usize;
            if cd.bytes(o, 5) == Some(&[1, 1, 0, 0, 0]) && let Some(opacity) = cd.bytes(o + 5, 2) {
                let opacity = i32::from(u16::from_le_bytes([opacity[0], opacity[1]])).min(256);
                ops.push(Op::ScreenFilter { slot: TOP_FILL_FILTER_SLOT, filter: Some(colour_filter(fill, opacity)), fade_ms: 0 });
                continue;
            }
        }
        if ins.op == OP_OPACITY && let Some(fill) = background_fill {
            // The filled background (slot 1) over nothing: darker.
            let o = cd.arg(ins) as u32 as usize;
            if cd.bytes(o, 5) == Some(&[1, 1, 0, 0, 0]) && let Some(opacity) = cd.bytes(o + 5, 2) {
                let opacity = u32::from(u16::from_le_bytes([opacity[0], opacity[1]])).min(256);
                let scaled = [16, 8, 0].iter().fold(0, |c, s| c | ((fill >> s & 0xff) * opacity / 256) << s);
                ops.push(Op::Background { image: assets.fill(scaled)?, fade_ms: 0 });
                continue;
            }
        }
        if ins.op == OP_OPACITY {
            let o = cd.arg(ins) as u32 as usize;
            let record = cd.bytes(o, 7).map(|r| (r[0], u32::from_le_bytes([r[1], r[2], r[3], r[4]]), u16::from_le_bytes([r[5], r[6]])));
            match record {
                Some((2, character, opacity)) if opacity <= 256 => {
                    opacities.insert(character, opacity.into());
                    ops.push(Op::SpriteOpacity { layer: Some(character), opacity: opacity.into() })
                }
                Some((0 | 1, 2, opacity)) if opacity <= 256 => ops.push(Op::SpriteOpacity { layer: None, opacity: opacity.into() }),
                _ => ops.push(Op::Unsupported { what: format!("opcode {:#04x} record {record:?}", ins.op), at: at() }),
            }
            continue;
        }
        if ins.op == OP_SCALE {
            let o = cd.arg(ins) as u32 as usize;
            let record = cd.bytes(o, 5).map(|r| (r[0], u32::from_le_bytes([r[1], r[2], r[3], r[4]])));
            let scale = pool_u32(cd, o + 5).zip(pool_u32(cd, o + 9));
            match (record, scale) {
                (Some((2, layer)), Some((sx @ 1..=1000, sy @ 1..=1000))) if placed.contains_key(&layer) => {
                    let (number, cx, cy) = placed[&layer];
                    let img = assets.sprite_scaled(number.into(), sx, sy)?;
                    let sprite = SpritePlacement { image: img.path, x: cx - img.width as i32 / 2, y: cy - img.height as i32 / 2 };
                    ops.push(Op::Sprites { changes: vec![SpriteChange { layer, sprite: Some(sprite), keep_filters: true }], fade_ms: 0 });
                    // The new picture is opaque again.
                    if let Some(&opacity) = opacities.get(&layer) {
                        ops.push(Op::SpriteOpacity { layer: Some(layer), opacity });
                    }
                }
                _ => ops.push(Op::Unsupported { what: format!("opcode {:#04x} record {record:?} {scale:?}", ins.op), at: at() }),
            }
            continue;
        }
        if ins.op == OP_POSITION {
            let o = cd.arg(ins) as u32 as usize;
            let i = |at: usize| cd.bytes(at, 4).map(|b| i32::from_le_bytes(b.try_into().unwrap()));
            let record = cd.bytes(o, 5).map(|r| (r[0], u32::from_le_bytes([r[1], r[2], r[3], r[4]])));
            match (record, i(o + 5), i(o + 9)) {
                (Some((2, character)), Some(x), Some(y)) => {
                    if let Some(p) = placed.get_mut(&character) {
                        (p.1, p.2) = (x, y);
                    }
                    ops.push(Op::SpriteMove { layer: character, x, y, ms: 0 })
                }
                _ => ops.push(Op::Unsupported { what: format!("opcode {:#04x} record {record:?}", ins.op), at: at() }),
            }
            continue;
        }
        if ins.op == OP_MODULE_WAIT {
            continue; // the slides run to their end at 0x86
        }
        if ins.op == OP_MODULE {
            let o = cd.arg(ins) as u32 as usize;
            match motion(cd, o) {
                Some(m) if m.flags == MODULE_REMOVE => {}
                // The fill's fade-out (0x56): a timed removal; the wait
                // after it already covers the fade.
                Some(Motion { layer: 1, depth: 2, flags: MOTION_OPACITY, end: Some(MotionEnd { ms, opacity: Some(0), .. }) }) if overlay_fill => {
                    ops.push(Op::BeginTransition { effect: "fill fade-out".into(), fade_ms: ms, mask: None, style: TransitionStyle::Fade });
                    overlay_depth = None;
                    ops.push(Op::Overlay { image: None, over_sprites: false });
                    ops.push(Op::EndTransition);
                    overlay_fill = false;
                    wait_credit_ms = ms;
                }
                // A slide with a turn that ends off screen (0043.cd: a
                // character falls over, -90°): drawn turning, then gone.
                Some(Motion { layer: 2, depth, flags, end: Some(end) })
                    if flags & !0x100 == MOTION_POSITION
                        && end.position
                        && end.rotation % 3600 != 0
                        && placed.get(&depth).is_some_and(|&(n, ..)| turned_off_screen(assets, n, &end)) =>
                {
                    if std::mem::take(&mut transition_armed) {
                        end_transition(&mut ops, &mut smooth_tail);
                    }
                    let (number, cx, cy) = placed.remove(&depth).unwrap();
                    let image = assets.sprite(number.into())?.path;
                    ops.push(Op::SpriteTurn { layer: depth, image, from: (cx, cy), to: (end.x, end.y), degrees: end.rotation / 10, ms: end.ms });
                }
                Some(Motion { layer: 2, depth, flags, end: Some(end) }) if flags & !0x100 == MOTION_POSITION && end.position => {
                    if let Some(p) = placed.get_mut(&depth) {
                        (p.1, p.2) = (end.x, end.y);
                    }
                    // The original slides while the transition runs; here
                    // the transition shows the start, then the slide runs.
                    if std::mem::take(&mut transition_armed) {
                        end_transition(&mut ops, &mut smooth_tail);
                    }
                    ops.push(Op::SpriteMove { layer: depth, x: end.x, y: end.y, ms: end.ms });
                    if flags & 0x100 != 0 {
                        ops.push(Op::Unsupported { what: format!("opcode {:#04x} rotation {}", ins.op, end.rotation), at: at() });
                    }
                }
                m => ops.push(Op::Unsupported { what: format!("opcode {:#04x} module {m:?}", ins.op), at: at() }),
            }
            continue;
        }
        if matches!(ins.op, OP_FX_CREATE | OP_FX_REMOVE | OP_PICTURE | OP_ANCHOR | OP_SHOW)
            && cd.bytes(cd.arg(ins) as u32 as usize, 1) == Some(&[2])
        {
            let o = cd.arg(ins) as u32 as usize;
            let record = cd.pool.get(o..).unwrap_or(&[]);
            let unsupported = || Op::Unsupported { what: format!("opcode {:#04x} record {:02x?}", ins.op, &record[..record.len().min(24)]), at: at() };
            let byte = |i: usize| cd.bytes(o + i, 1).map(|b| b[0]);
            let u16_at = |i: usize| cd.bytes(o + i, 2).map(|b| u16::from_le_bytes([b[0], b[1]]));
            let u32_at = |i: usize| cd.bytes(o + i, 4).map(|b| u32::from_le_bytes(b.try_into().unwrap()));
            let Some(layer) = u32_at(1) else {
                ops.push(unsupported());
                continue;
            };
            let picture = pictures.get_mut(&layer);
            let shown = match (ins.op, picture) {
                (OP_FX_CREATE, _) => {
                    // Blend 0 and full opacity in every use; the size is the picture's.
                    match (u16_at(10), u16_at(12), byte(18), u16_at(19), byte(21)) {
                        (Some(x), Some(y), Some(0), Some(256), Some(visible)) => {
                            let picture = Picture { x: x.into(), y: y.into(), visible: visible != 0, ..Default::default() };
                            pictures.insert(layer, picture);
                            // The new object replaces the character's bust-up.
                            ops.push(Op::Sprites { changes: vec![SpriteChange { layer, sprite: None, keep_filters: false }], fade_ms: 0 });
                            // Group 4 is the top band (op_7a 0x414590, table 0x503e68, key 0x4126c0): over the text window.
                            if byte(5) == Some(PICTURE_GROUP_TOP) {
                                ops.push(Op::SpriteOverText { layer });
                            }
                        }
                        _ => ops.push(unsupported()),
                    }
                    continue;
                }
                (OP_FX_REMOVE, _) => {
                    pictures.remove(&layer);
                    ops.push(Op::Sprites { changes: vec![SpriteChange { layer, sprite: None, keep_filters: false }], fade_ms: 0 });
                    continue;
                }
                (OP_PICTURE, Some(p)) if byte(5) == Some(PICTURE_KIND_BUSTUP) && u32_at(6).is_some_and(|n| n > 0) => {
                    p.sprite = u32_at(6);
                    p.change(assets, layer)?
                }
                (OP_ANCHOR, Some(p)) if matches!(cd.bytes(o + 5, 2), Some(&[0..=2, 0..=2])) => {
                    p.anchor = (record[5], record[6]);
                    p.change(assets, layer)?
                }
                (OP_SHOW, Some(p)) if byte(5).is_some() => {
                    p.visible = record[5] != 0;
                    match p.change(assets, layer)? {
                        None => Some(SpriteChange { layer, sprite: None, keep_filters: false }),
                        shown => shown,
                    }
                }
                _ => {
                    ops.push(unsupported());
                    continue;
                }
            };
            if let Some(change) = shown {
                ops.push(Op::Sprites { changes: vec![change], fade_ms: 0 });
            }
            continue;
        }
        if matches!(ins.op, OP_FX_CREATE | OP_FX_LOAD | OP_FX_WAIT | OP_FX_REMOVE) {
            let o = cd.arg(ins) as u32 as usize;
            let record = cd.pool.get(o..).unwrap_or(&[]);
            let unsupported = || Op::Unsupported { what: format!("opcode {:#04x} record {:02x?}", ins.op, &record[..record.len().min(24)]), at: at() };
            let byte = |i: usize| cd.bytes(o + i, 1).map(|b| b[0]);
            let u16_at = |i: usize| cd.bytes(o + i, 2).map(|b| u16::from_le_bytes([b[0], b[1]]));
            if byte(0) != Some(0) {
                ops.push(unsupported());
                continue;
            }
            match ins.op {
                OP_FX_CREATE => {
                    let full_screen = [u16_at(10), u16_at(12), u16_at(14), u16_at(16)] == [Some(0), Some(0), Some(800), Some(600)];
                    match (full_screen, byte(18), u16_at(19)) {
                        (true, Some(0), Some(opacity)) => fx_opacity = Some(u32::from(opacity).min(256)),
                        _ => ops.push(unsupported()),
                    }
                }
                OP_FX_LOAD => {
                    let name = cd.bytes(o + 13, 32).map(|n| n.split(|&b| b == 0).next().unwrap_or(&[]));
                    let name = name.and_then(|n| std::str::from_utf8(n).ok()).filter(|n| n.starts_with("fx"));
                    let animation = match name {
                        Some(name) => assets.animation(name)?,
                        None => None,
                    };
                    match (animation, fx_opacity) {
                        (Some(a), Some(opacity)) => {
                            fx_frames = Some((a.frame_ms.clone(), a.looped));
                            ops.push(Op::Animation(Some(Animation { image: a.image, frame_ms: a.frame_ms, looped: a.looped, opacity })));
                        }
                        _ => ops.push(unsupported()),
                    }
                }
                OP_FX_WAIT => match &fx_frames {
                    // Until the last frame shows; a click ends `delay` too.
                    Some((frames, false)) => ops.push(Op::Wait { ms: frames[..frames.len() - 1].iter().sum() }),
                    Some((_, true)) => {}
                    None => ops.push(unsupported()),
                },
                _ => {
                    // Also when this script didn't make it (0012.cd #916):
                    // removing a missing object does nothing.
                    fx_opacity = None;
                    fx_frames = None;
                    ops.push(Op::Animation(None));
                }
            }
            continue;
        }
        if ins.op == OP_UNLOCK_SCENE {
            match cd.arg(ins) {
                scene @ 0..REPLAY_SCENES => ops.push(Op::UnlockScene { scene: scene as u32 }),
                _ => ops.push(Op::Unsupported { what: format!("opcode {:#04x} arg {}", ins.op, cd.arg(ins)), at: at() }),
            }
            continue;
        }
        if ins.op == OP_DEFAULT_TEXT_STYLE {
            let o = cd.arg(ins) as u32 as usize;
            let size = cd.bytes(o, 2).map(|b| u16::from_le_bytes([b[0], b[1]]));
            match (size, pool_u32(cd, o + 2)) {
                (Some(size), Some(colour)) => {
                    default_style.1 = match size {
                        0xf002 => default_style.1,
                        n if n & 0xf000 == 0x1000 => Some(STYLE_PRESET_PERCENT[usize::from(n & 0xfff).clamp(1, 7)]),
                        n if n < 0x1000 => Some(n),
                        _ => None,
                    };
                    default_style.0 = match colour {
                        0xff00_0002 => default_style.0,
                        c if c >> 24 == 0 => Some(rgb_of(c)),
                        _ => None,
                    };
                }
                _ => ops.push(Op::Unsupported { what: format!("opcode {:#04x} (bad record)", ins.op), at: at() }),
            }
            continue;
        }
        if ins.op == OP_RETURN_TO_ROOT {
            ops.push(Op::ReturnToRoot);
            continue;
        }
        if ins.op == OP_STAFF_ROLL {
            match assets.staff_roll(cd.arg(ins) as usize) {
                Ok(file) => ops.push(Op::Movie { file, skippable: false }),
                Err(ConvertError::MissingAsset { .. }) => {
                    ops.push(Op::Unsupported { what: format!("opcode {:#04x} arg {}", ins.op, cd.arg(ins)), at: at() })
                }
                Err(e) => return Err(e),
            }
            continue;
        }
        if ins.op == OP_PROGRESS {
            match cd.sub(ins) {
                bit @ 0..32 => ops.push(Op::Progress { bit: bit.into() }),
                _ => ops.push(Op::Unsupported { what: format!("opcode {:#04x} sub {:#x}", ins.op, cd.sub(ins)), at: at() }),
            }
            continue;
        }
        if ins.op == OP_CG_SEEN {
            match cd.arg(ins) {
                number @ 1.. if (number as usize) <= 9 * BG_PER_ARCHIVE => ops.push(Op::CgSeen { number: number as u32 }),
                _ => ops.push(Op::Unsupported { what: format!("opcode {:#04x} arg {}", ins.op, cd.arg(ins)), at: at() }),
            }
            continue;
        }
        if ins.op == OP_SCENE_END {
            ops.push(Op::SceneEnd);
            continue;
        }
        if ins.op == OP_CHAPTER_END {
            let target = pool_u32(cd, cd.arg(ins) as u32 as usize + 4)
                .and_then(|i| cd.labels.get((i as usize).checked_sub(1)?));
            let next = cd.code.get(index + 1).map(|i| i.op);
            match target {
                Some(l) if l.script == cd.script_id && l.code_offset as usize / 12 == index + 1 && next == Some(OP_CHAPTER_CLEAR) => {}
                _ => ops.push(Op::Unsupported { what: format!("opcode {:#04x} (not followed by its 0x38)", ins.op), at: at() }),
            }
            continue;
        }
        if ins.op == OP_CHAPTER_CLEAR {
            ops.push(Op::Chapter { title: String::new() });
            continue;
        }
        if ins.op == OP_CHAPTER {
            let title = pool_u32(cd, cd.arg(ins) as u32 as usize)
                .and_then(|label| names.own(cd, label))
                .and_then(chapter_title);
            match title {
                Some(title) => ops.push(Op::Chapter { title: title.to_string() }),
                None => ops.push(Op::Unsupported { what: format!("opcode {:#04x} (no chapter name)", ins.op), at: at() }),
            }
            continue;
        }
        if ins.op == OP_TRANSITION {
            match pool_u32(cd, cd.arg(ins) as u32 as usize) {
                Some(fade_ms) => {
                    transition_armed = true;
                    let key = pool_u32(cd, cd.arg(ins) as u32 as usize + 8);
                    keyed_start = key.filter(|k| matches!(k & 0xff, 1..=3)).map(|_| ops.len());
                    // TvPowerSwitch runs its own 700 ms key frames (setup
                    // 0x4485d7: 260 ... 700 ms) whatever the record says.
                    let mut fade_ms = if cd.sub(ins) == TRANSITION_TV_POWER_SWITCH { TV_POWER_SWITCH_MS } else { fade_ms };
                    // SmoothCrossFade (filmed under wine, 3 s): the old
                    // picture blurs at once (about a Gaussian σ 1: a 3×3
                    // mean) and stays for a third of the time, cross-fades
                    // in the second third and the new one sharpens in the
                    // last ([`end_transition`]).
                    if cd.sub(ins) == TRANSITION_SMOOTH_CROSS_FADE {
                        let blur = SpriteFilter {
                            kind: Operand::Const(FILTER_BLUR),
                            blend: Operand::Const(FILTER_BLEND_COPY),
                            strength: Operand::Const(256),
                            params: [1, 1, 0].map(Operand::Const),
                        };
                        ops.push(Op::ScreenFilter { slot: SMOOTH_BLUR_SLOT, filter: Some(blur), fade_ms: 0 });
                        ops.push(Op::Wait { ms: fade_ms / 3 });
                        smooth_tail = Some(fade_ms / 3);
                        fade_ms /= 3;
                    }
                    // Dot4x4 (kernel 0x4430a0) copies the new picture
                    // through a 4×4 pattern: the hard mask transition over
                    // a tiled Bayer mask reveals the same dots.
                    let mask = match cd.sub(ins) {
                        TRANSITION_DOT4X4 => Some(assets.dither_mask()?),
                        // Trans_SoftEdgeWipe (setup 0x4481e0 with sub - 5;
                        // directions seen under wine): 5 up from the bottom,
                        // 6 from the left, 7 down from the top, 8 from the
                        // right, with a soft edge of about 64 px. Drawn as a
                        // hard wipe through a gradient (ONScripter's soft
                        // mask fade can't keep a narrow edge across the
                        // screen).
                        5..=8 => {
                            let (dx, dy) = [(0, -1), (1, 0), (0, 1), (-1, 0)][usize::from(cd.sub(ins) - 5)];
                            Some(assets.wipe_mask(dx, dy)?)
                        }
                        _ => None,
                    };
                    ops.push(Op::BeginTransition {
                        effect: transition_name(cd.sub(ins)).to_string(),
                        fade_ms,
                        mask,
                        style: transition_style(cd.sub(ins)),
                    });
                }
                None => ops.push(Op::Unsupported { what: format!("opcode {:#04x} (bad record)", ins.op), at: at() }),
            }
            continue;
        }
        if matches!(ins.op, OP_TEXT_BRN | OP_WINDOW_MODE | OP_WINDOW_STYLE | OP_WINDOW_POS) {
            continue; // brn = the default in full-screen modes; presets: see window_presets
        }
        if ins.op == OP_TEXT_PAGE {
            page_break = true;
            continue;
        }
        if let Some(module) = cd.preset_module(ins) {
            lower_preset_module(&module?, &mut ops, at);
            continue;
        }
        if let Some(module) = cd.screen_module(ins) {
            let module = module?;
            match keyed_start.as_mut().filter(|_| transition_armed && module.layer == 0) {
                // A layer-0 filter sorts over everything (it darkens the
                // bust-ups), above such a transition's key: it changes at
                // once, before the transition (filmed: 0011.cd #193 clears
                // one inside a 0.5 s cross-fade keyed layer 2 depth 100).
                Some(start) => {
                    let mut now = Vec::new();
                    lower_screen_module(&module, &mut replace_slots, assets, &mut now, at)?;
                    let n = now.len();
                    ops.splice(*start..*start, now);
                    *start += n;
                }
                None => lower_screen_module(&module, &mut replace_slots, assets, &mut ops, at)?,
            }
            continue;
        }
        if let Some(module) = cd.environment(ins) {
            let module = module?;
            let time = module.noise.map(|n| n.time_ms).or(module.ripple.map(|r| r.time_ms)).or(module.shake.map(|s| s.time_ms));
            match time {
                Some(ms) => env_times.insert(module.slot, ms),
                None => env_times.remove(&module.slot),
            };
            lower_environment(&module, &mut env_slots, assets, &mut ops, at)?;
            continue;
        }
        if ins.op == OP_ENV_WAIT {
            // Waits the module's whole time: the one use follows the 0x8c
            // that set the slot at once.
            match pool_u32(cd, cd.arg(ins) as u32 as usize).and_then(|slot| env_times.get(&slot)) {
                Some(&ms) => ops.push(Op::Wait { ms }),
                None => ops.push(Op::Unsupported { what: format!("opcode {:#04x} (no module time)", ins.op), at: at() }),
            }
            continue;
        }
        // While skipping (skip_state_a/_b) the original draws none of these;
        // with 画面エフェクトを使用 off (setting_effects bit 0) none but the
        // flash. A script fade left out draws no cover at all.
        let fade = matches!(ins.op, OP_FADE_BLACK | OP_FADE_WHITE) && cd.arg(ins) >> 16 == 0;
        if fade || matches!(ins.op, OP_FLASH | OP_QUAKE | OP_SURPRISE) {
            let gate = format!("s{}_fx{index}", cd.script_id);
            ops.push(Op::IfSkipping { label: gate.clone() });
            if ins.op != OP_FLASH {
                ops.push(Op::IfSwitch { switch: Switch::Effects, on: false, label: gate.clone() });
            }
            if ins.op == OP_FLASH {
                let c = cd.arg(ins) as u32;
                let colour = (c & 0xff) << 16 | (c & 0xff00) | (c >> 16 & 0xff);
                // At sort key 0xffffffff: over the text window too.
                ops.push(Op::ScreenFlash { colour, opacity: FLASH_OPACITY, ms: FLASH_MS });
            } else if let Some(quake) = cd.quake(ins) {
                lower_quake(&quake?, &mut ops);
            } else if let Some(surprise) = cd.surprise(ins) {
                lower_surprise(&surprise?, &mut ops);
            } else {
                let colour = if ins.op == OP_FADE_WHITE { 0xffffff } else { 0 };
                ops.push(Op::ScreenFade { colour, ms: u32::from(cd.arg(ins) as u16) });
            }
            ops.push(Op::Label(gate));
            continue;
        }
        if let Some(apply) = cd.apply_preset(ins) {
            let apply = apply?;
            // Without an object for the character the handler does nothing;
            // neither does spfilter on a layer without a sprite.
            match preset_filters(apply.preset) {
                Some(filters) => ops.push(Op::SpriteFilters { layer: u32::from(apply.character), filters }),
                None => ops.push(Op::Unsupported { what: format!("opcode {:#04x} preset {}", ins.op, apply.preset), at: at() }),
            }
            continue;
        }
        if ins.op == OP_TRANSITION_WAIT {
            if std::mem::take(&mut transition_armed) {
                end_transition(&mut ops, &mut smooth_tail);
            }
            continue;
        }
        if let Some(Ok(entries)) = cd.sprites(ins) {
            for e in entries {
                let layer = u32::from(e.character);
                opacities.remove(&layer);
                if e.mode & 1 == 1 {
                    placed.insert(layer, (e.sprite, i32::from(e.x), i32::from(e.y)));
                    // A bust-up deeper than the overlay (1100-5000; filmed
                    // under wine) is drawn over it, under the static and
                    // the animations.
                    // Every time: a csp -1 in a subroutine (s2_l17) puts
                    // it back unseen.
                    let front = u32::from(e.depth) > BUST_UP_DEPTH_MAX && overlay_depth.is_none_or(|d| u32::from(e.depth) > d);
                    if front || in_front.contains(&layer) {
                        ops.push(Op::SpriteInFront { layer, front });
                    }
                    if front {
                        in_front.insert(layer);
                    } else {
                        in_front.remove(&layer);
                    }
                } else {
                    placed.remove(&layer);
                    in_front.remove(&layer);
                }
            }
        }
        if matches!(ins.op, OP_CLEAR_SPRITES | crate::xuse::cd::OP_BACKGROUND) {
            placed.clear();
            opacities.clear();
            in_front.clear();
        }
        let paths = match variants.take().filter(|_| in_run) {
            Some((_, _, paths)) => paths,
            None => {
                let records = if cd.message(ins).is_some() { std::mem::take(&mut pending) } else { MessageRecords::default() };
                vec![RecordPath { label: None, prefix: Vec::new(), records: Some(records) }]
            }
        };
        let after = (paths.len() > 1).then(|| {
            unfold_id += 1;
            format!("s{}_records{unfold_id}", cd.script_id)
        });
        for path in paths {
            if let Some(label) = path.label {
                ops.push(Op::Label(label));
            }
            ops.extend(path.prefix);
            let Some(records) = path.records else { continue };
            let unsupported = || format!("opcode {:#04x} sub {:#x} arg {}", ins.op, cd.sub(ins), cd.arg(ins));
            match lower_instr(cd, ins, &records, assets, names, window_size.is_none_or(|w| !w.2)) {
                Ok(Some(Op::Text { speaker, text, voice, mut links, ruby, mut styles, hang, continues })) if flow == Flow::Linear => {
                    whole_text_style(&mut styles, &text, default_style);
                    // No jumps in linear mode, so links are colour only.
                    links.iter_mut().for_each(|l| l.target = None);
                    ops.push(Op::Text { speaker, text, voice, links, ruby, styles, hang, continues });
                }
                Ok(Some(Op::Text { speaker, text, voice, links, ruby, mut styles, hang, .. })) => {
                    whole_text_style(&mut styles, &text, default_style);
                    let rows_of = |text: &str, indent: usize| {
                        window_size.map_or(0, |(_, columns, _)| text.split('\n').map(|l| line_rows(l, columns, indent)).sum::<usize>())
                    };
                    let indent = hang_cells(&text, hang);
                    // A message taller than the window goes on as many pages
                    // as it needs (the original's proportional font fits
                    // more on a row).
                    let parts = match window_size {
                        Some((rows, columns, _)) if rows_of(&text, indent) > rows => split_rows(&text, &links, &ruby, &styles, rows, columns, indent),
                        _ => vec![PagePart { text, links, ruby, styles }],
                    };
                    let (mut speaker, mut voice, mut hang) = (speaker, voice, hang);
                    let full_screen_rows = window_size.filter(|w| w.2).map(|(rows, columns, _)| (rows, columns));
                    for part in parts {
                        let lines = rows_of(&part.text, if hang.is_some() { indent } else { 0 });
                        if let (Some((rows, _)), Some((i, used))) = (full_screen_rows, last_text) {
                            let joins = !page_break && used + lines <= rows;
                            if joins && let Some(Op::Text { continues, .. }) = ops.get_mut(i) {
                                *continues = true;
                            }
                            let used = if joins { used + lines } else { lines };
                            last_text = Some((ops.len(), used));
                        } else if full_screen_rows.is_some() {
                            last_text = Some((ops.len(), lines));
                        }
                        page_break = false;
                        let PagePart { text, links, ruby, styles } = part;
                        ops.push(Op::Text { speaker: speaker.take(), text, voice: voice.take(), links, ruby, styles, hang: hang.take(), continues: false });
                    }
                }
                Ok(Some(Op::Gosub(target))) => {
                    let preset = presets.window.get(&target).copied();
                    // A subroutine that sets a window through a preset: the
                    // window is set again here, so the text after it is laid
                    // out for that window.
                    let sub = presets.sub_window.get(&target).copied().filter(|p| window_preset != Some(*p));
                    ops.push(Op::Gosub(target));
                    if let Some(p) = preset.or(sub) {
                        window_size = Some(window_size_of(assets, p)?);
                        last_text = None;
                        window_preset = Some(p);
                        set_window(&mut ops, assets, p)?;
                    }
                }
                Ok(Some(op @ (Op::Wait { .. } | Op::Pause { .. }))) if wait_credit_ms > 0 => {
                    let credit = std::mem::take(&mut wait_credit_ms);
                    match op {
                        Op::Wait { ms } if ms > credit => ops.push(Op::Wait { ms: ms - credit }),
                        Op::Pause { ms } if ms > credit => ops.push(Op::Pause { ms: ms - credit }),
                        _ => {}
                    }
                }
                Ok(Some(op)) => ops.push(op),
                Ok(None) => ops.push(Op::Unsupported { what: unsupported(), at: at() }),
                Err(ConvertError::MissingAsset { kind, number }) => {
                    ops.push(Op::Unsupported { what: format!("{} (missing {kind} {number})", unsupported()), at: at() })
                }
                Err(e) => return Err(e),
            }
            if let Some(after) = &after {
                ops.push(Op::Jump(after.clone()));
            }
        }
        if let Some(after) = after {
            ops.push(Op::Label(after));
        }
    }
    Ok(ops)
}

/// Converts a link's (row, column, length) into a character range of the
/// page text. Rows count message segments from 1, including the speaker's
/// name segment; links that don't land inside the text are dropped.
fn link_range(segments: &[String], has_speaker: bool, link: &Link) -> Option<(usize, usize)> {
    let row = usize::from(link.row).checked_sub(1)?;
    let col = usize::from(link.column).checked_sub(1)?;
    let first_body = usize::from(has_speaker);
    if row < first_body || row >= segments.len() {
        return None;
    }
    let len = usize::from(link.length);
    if col + len > segments[row].chars().count() {
        return None;
    }
    // Body segments are joined with '\n', which counts as one character.
    let before: usize = segments[first_body..row].iter().map(|s| s.chars().count() + 1).sum();
    Some((before + col, len))
}

/// Character range of `ruby` in the text lines `body` joined with '\n'.
fn ruby_range(body: &[String], ruby: &RubyRecord) -> Option<(usize, usize)> {
    let line = usize::from(ruby.line).checked_sub(1)?;
    let col = usize::from(ruby.column).checked_sub(1)?;
    let len = usize::from(ruby.length);
    if len == 0 || col + len > body.get(line)?.chars().count() {
        return None;
    }
    let before: usize = body[..line].iter().map(|s| s.chars().count() + 1).sum();
    Some((before + col, len))
}

/// Adds `ins` to `records` if it is a record for the next message.
fn add_record(cd: &CdScript, ins: &Instr, records: &mut MessageRecords) -> Result<bool, ConvertError> {
    if let Some(link) = cd.link(ins) {
        records.links.push(link?);
    } else if let Some(ruby) = cd.ruby(ins) {
        records.ruby.push(ruby?);
    } else if let Some(style) = cd.text_style(ins) {
        records.styles.push(style?);
    } else if ins.op == OP_TEXT_INDENT {
        records.indent = Some((cd.arg(ins) as u32 >> 16) as u16);
    } else {
        return Ok(false);
    }
    Ok(true)
}

/// One way into a run of records unfolded by [`unfold_records`]: the label
/// it starts at (`None` for the fall-through), the jumps it takes, and the
/// records its message gets (`None` if it jumps to another path instead).
struct RecordPath {
    label: Option<String>,
    prefix: Vec<Op>,
    records: Option<MessageRecords>,
}

/// Records collect at run time until a message uses them, so a jump to a
/// label between the records and their message skips the ones before it.
/// 最果てのイマ gates its route links that way (`0004.cd`: `if` a route is
/// taken, jump past the 0x46 to the same message without the link) and
/// picks one of two links with an if/else (`0009.cd`, `0011.cd`). For each
/// run of records, label markers, gotos and ifs that ends in a message and
/// has a label inside it, this lists one [`RecordPath`] per entry: the
/// fall-through, each label, and each jump from inside the run with the
/// records collected so far. Keyed by the run's first record; the value
/// holds the message's index.
fn unfold_records(
    cd: &CdScript,
    own_labels: &HashMap<usize, u32>,
    assets: &mut AssetBank,
    names: &LabelNames,
) -> Result<HashMap<usize, (usize, Vec<RecordPath>)>, ConvertError> {
    let is_record = |ins: &Instr| cd.link(ins).is_some() || cd.ruby(ins).is_some() || cd.text_style(ins).is_some() || ins.op == OP_TEXT_INDENT;
    let label_name = |label: u32| format!("s{}_l{label}", cd.script_id);
    let mut runs = HashMap::new();
    let mut index = 0;
    while index < cd.code.len() {
        if !is_record(&cd.code[index]) {
            index += 1;
            continue;
        }
        let start = index;
        let message = cd.code[start..].iter().position(|ins| !(is_record(ins) || matches!(ins.op, OP_GOTO | OP_GOTO_SUB | OP_IF_EQUAL..=OP_IF_GE)));
        let message = message.map(|n| start + n).filter(|&m| cd.message(&cd.code[m]).is_some());
        index = message.map_or(start + 1, |m| m + 1);
        let Some(message) = message else { continue };
        let inside: BTreeMap<usize, String> = (start + 1..=message).filter_map(|i| own_labels.get(&i).map(|&l| (i, label_name(l)))).collect();
        if inside.is_empty() {
            continue;
        }
        // Path entries by (index, records collected before it): name, done.
        let mut entries: Vec<(usize, Vec<usize>, Option<String>)> = vec![(start, Vec::new(), None)];
        entries.extend(inside.iter().map(|(&i, name)| (i, Vec::new(), Some(name.clone()))));
        let mut paths = Vec::new();
        let mut next = 0;
        let mut unsupported = false;
        while next < entries.len() {
            let (entry, mut carried, label) = entries[next].clone();
            next += 1;
            let mut prefix = Vec::new();
            let mut ends_in_message = true;
            let mut i = entry;
            while i < message {
                let ins = &cd.code[i];
                let target_of = |name: &str, carried: &[usize], entries: &mut Vec<(usize, Vec<usize>, Option<String>)>| {
                    let at = *inside.iter().find(|(_, n)| n.as_str() == name)?.0;
                    if let Some((_, _, l)) = entries.iter().find(|(e, c, _)| *e == at && c == carried) {
                        return l.clone();
                    }
                    let name = format!("{name}_r{}", entries.len());
                    entries.push((at, carried.to_vec(), Some(name.clone())));
                    Some(name)
                };
                if is_record(ins) {
                    carried.push(i);
                } else if ins.op == OP_GOTO_SUB {
                    let Some(target) = table_label(cd, cd.sub(ins).into()) else { unsupported = true; break };
                    match target_of(&target, &carried, &mut entries) {
                        Some(name) => prefix.push(Op::Jump(name)),
                        None => prefix.push(Op::Jump(target)),
                    }
                    ends_in_message = false;
                    break;
                } else if ins.op != OP_GOTO {
                    match lower_instr(cd, ins, &MessageRecords::default(), assets, names, true)? {
                        Some(Op::If { var, cmp, value, label }) => {
                            let label = target_of(&label, &carried, &mut entries).unwrap_or(label);
                            prefix.push(Op::If { var, cmp, value, label });
                        }
                        _ => {
                            unsupported = true;
                            break;
                        }
                    }
                } else if own_label_at(cd, cd.arg(ins)) != Some(i) {
                    unsupported = true; // a goto by label number, not a marker
                    break;
                }
                i += 1;
            }
            if unsupported {
                break;
            }
            let records = if ends_in_message {
                let mut records = MessageRecords::default();
                for &r in &carried {
                    add_record(cd, &cd.code[r], &mut records)?;
                }
                Some(records)
            } else {
                None
            };
            paths.push(RecordPath { label, prefix, records });
        }
        if !unsupported {
            runs.insert(start, (message, paths));
        }
    }
    Ok(runs)
}

/// Records that apply to the next message: links (0x46), ruby (0x44),
/// text styles (0x49) and the hanging indent (0x4e, half-width units).
#[derive(Default)]
struct MessageRecords {
    links: Vec<Link>,
    ruby: Vec<RubyRecord>,
    styles: Vec<StyleRecord>,
    indent: Option<u16>,
}

/// The character of `line` that `units` half-width units (0x4e) reach:
/// ASCII, half-width katakana and Greek letters are one unit in the game's
/// layout, other characters two.
fn hang_char(line: &str, units: u16) -> Option<usize> {
    let mut width = 0;
    for (i, c) in line.chars().enumerate() {
        if width >= usize::from(units) {
            return (i > 0).then_some(i);
        }
        let half = c.is_ascii() || ('\u{FF61}'..='\u{FF9F}').contains(&c) || ('\u{0391}'..='\u{03C9}').contains(&c);
        width += if half { 1 } else { 2 };
    }
    None
}

/// Size [`StyleRecord::size`] that keeps the window's size.
const STYLE_SIZE_KEEP: u16 = 0xf000;
/// Colour [`StyleRecord::colour`] that keeps the text colour.
const STYLE_COLOUR_KEEP: u32 = 0xff00_0000;

/// A 0x49 record as a range of the text lines `body` joined with '\n'.
/// Length 30000 (or any past the end) runs to the end of the message: the
/// game wraps the lines of enlarged messages short enough to fit.
/// Sizes are percent; 0x1000 + n is a preset (table at 0x4fa850, looked
/// up at 0x491d00).
const STYLE_PRESET_PERCENT: [u16; 8] = [0, 25, 50, 75, 100, 125, 150, 175];

/// COLORREF 0x00BBGGRR to 0xRRGGBB.
fn rgb_of(c: u32) -> u32 {
    let [r, g, b, _] = c.to_le_bytes();
    u32::from_be_bytes([0, r, g, b])
}

/// Puts 0x4a's style (colour, size) on the whole message, under its own
/// 0x49 styles.
fn whole_text_style(styles: &mut Vec<TextStyle>, text: &str, (colour, size_percent): (Option<u32>, Option<u16>)) {
    let len = text.chars().count();
    if len > 0 && (colour.is_some() || size_percent.is_some()) {
        styles.insert(0, TextStyle { start: 0, len, colour, size_percent, ..Default::default() });
    }
}

fn text_style(body: &[String], s: &StyleRecord) -> Option<TextStyle> {
    let line = usize::from(s.line).checked_sub(1)?;
    let col = usize::from(s.column).checked_sub(1)?;
    if col >= body.get(line)?.chars().count() {
        return None;
    }
    let before: usize = body[..line].iter().map(|s| s.chars().count() + 1).sum();
    let total = body.iter().map(|s| s.chars().count() + 1).sum::<usize>() - 1;
    let len = usize::from(s.length).min(total - before - col);
    let colour = (s.colour != STYLE_COLOUR_KEEP).then(|| rgb_of(s.colour));
    let size = match s.size {
        STYLE_SIZE_KEEP => None,
        n if n & 0xf000 == 0x1000 => Some(STYLE_PRESET_PERCENT[usize::from(n & 0xfff).clamp(1, 7)]),
        n if n < 0x1000 => Some(n),
        _ => None,
    };
    if len == 0 || (colour.is_none() && size.is_none()) {
        return None;
    }
    Some(TextStyle { start: before + col, len, colour, size_percent: size, ..Default::default() })
}

fn lower_instr(
    cd: &CdScript,
    ins: &Instr,
    records: &MessageRecords,
    assets: &mut AssetBank,
    names: &LabelNames,
    names_shown: bool,
) -> Result<Option<Op>, ConvertError> {
    if let Some(msg) = cd.message(ins) {
        let msg = msg?;
        let segs = &msg.segments;
        // `sub` bit 0: the first segment is the speaker's name. In a text
        // mode that can't show names (`names_shown` false: the full-screen
        // windows, mode 1), show_message (0x475f40) clears the bit through
        // text_mode_allows_name (0x4679d0) and the name is a line of text.
        let has_speaker = cd.sub(ins) & 1 != 0 && names_shown && segs.len() >= 2;
        let (speaker, body) = if has_speaker { (Some(segs[0].clone()), &segs[1..]) } else { (None, &segs[..]) };
        let links = records
            .links
            .iter()
            .filter_map(|l| {
                let (start, len) = link_range(segs, has_speaker, l)?;
                let scene = names.table_entry(cd, l.target.into()).and_then(scene_title).map(String::from);
                Some(TextLink { start, len, target: table_label(cd, l.target.into()), scene })
            })
            .collect();
        // Segments are the game's own display lines.
        // A missing clip only drops the voice, not the text.
        let voice = match msg.voice_id.map(|id| assets.voice(id).map(|file| Voice { file, character: id / VOICE_CHARACTER })) {
            Some(Err(ConvertError::MissingAsset { .. })) | None => None,
            Some(r) => Some(r?),
        };
        let ruby = records
            .ruby
            .iter()
            .filter_map(|r| {
                let (start, len) = ruby_range(body, r)?;
                Some(Ruby { start, len, text: r.text.clone() })
            })
            .collect();
        let styles = records.styles.iter().filter_map(|s| text_style(body, s)).collect();
        let hang = records.indent.and_then(|units| hang_char(body.first()?, units));
        return Ok(Some(Op::Text { speaker, text: body.join("\n"), voice, links, ruby, styles, hang, continues: false }));
    }
    if let Some(sound) = cd.sound(ins) {
        return Ok(Some(match sound? {
            Sound::BgmPlay { track, fade_in_ms } => {
                Op::BgmPlay { file: assets.bgm(track as usize)?, fade_in_ms: fade_in_ms.into() }
            }
            Sound::BgmStop { fade_out_ms, .. } => Op::BgmStop { fade_out_ms: fade_out_ms.into() },
            // The old track fades out while the new one fades in, over the
            // crossfade's time (CONscripter's bgm cross-fade, #98).
            Sound::BgmCrossfade { to, fade_ms, .. } => Op::BgmCrossfade { file: assets.bgm(to as usize)?, ms: fade_ms },
            Sound::SePlay { sound, looped, .. } => Op::SePlay { file: assets.se(sound as usize)?, looped },
            // op_5b passes the fade to the sound request (0x47aab0).
            Sound::SeStop { sound, fade_out_ms } => {
                let file = if sound == 0 { None } else { Some(assets.se(sound as usize)?) };
                match fade_out_ms {
                    0 => Op::SeStop { file },
                    ms => Op::SeFadeOut { file, ms: ms.into() },
                }
            }
        }));
    }
    if let Some(entries) = cd.sprites(ins) {
        let entries = entries?;
        let mut changes = Vec::new();
        for e in &entries {
            // Each character (1-100) has its own bust-up object, and an entry
            // only touches that one. Mode bit 0 clear removes it (handler
            // 0x434130 deletes the "bustup-%u" object).
            let layer = u32::from(e.character);
            if e.mode & 1 == 0 {
                changes.push(SpriteChange { layer, sprite: None, keep_filters: false });
                continue;
            }
            // Mode bit 2 keeps the character's object, and with it the
            // attached filter; otherwise the handler (0x434130) deletes it
            // and creates a new one.
            let sprite = placement(assets, e)?;
            changes.push(SpriteChange { layer, sprite: Some(sprite), keep_filters: e.mode & 4 != 0 });
        }
        let fade_ms = if entries.iter().all(|e| e.mode & 2 != 0) { 0 } else { SPRITE_FADE_MS };
        return Ok(Some(Op::Sprites { changes, fade_ms }));
    }
    // Label-table index -> label name (index 0 is the engine's implicit
    // entry, not stored in the file; its meaning isn't known).
    let label = |n: i64| table_label(cd, n);
    let var = |v: u32| Var { global: v & 0x8000 != 0, index: (v & 0x7fff) as u16 };
    match ins.op {
        OP_GOTO | OP_GOTO_2 => {
            return Ok(own_label_at(cd, cd.arg(ins)).map(|_| Op::Jump(format!("s{}_l{}", cd.script_id, cd.arg(ins)))));
        }
        OP_GOTO_SUB => return Ok(label(cd.sub(ins).into()).map(Op::Jump)),
        OP_GOSUB => return Ok(label(cd.sub(ins).into()).map(Op::Gosub)),
        OP_RETURN | OP_LINK_RETURN => return Ok(Some(Op::Return)),
        OP_CLEAR_SPRITES => return Ok(Some(Op::ClearSprites)),
        OP_HIDE_TEXT => return Ok(Some(Op::HideTextWindow)),
        OP_TEXT_DISPLAY => return Ok(Some(Op::InstantText(cd.arg(ins) == 1))),
        OP_CLICK_WAIT => return Ok(Some(Op::ClickWait { skippable: cd.arg(ins) != 0 })),
        OP_CLICK_WAIT_SKIPPABLE => return Ok(Some(Op::ClickWait { skippable: true })),
        OP_MOVIE => {
            let file = assets.movie(cd.arg(ins) as u16)?;
            return Ok(Some(Op::Movie { file, skippable: cd.arg(ins) >> 16 != 0 }));
        }
        // The wait callback (0x475bf0) lets a click or skip end it only
        // when arg's high half isn't 0.
        OP_WAIT if cd.arg(ins) >> 16 == 0 => return Ok(Some(Op::Pause { ms: u32::from(cd.arg(ins) as u16) })),
        OP_WAIT => return Ok(Some(Op::Wait { ms: u32::from(cd.arg(ins) as u16) })),
        OP_IF_EQUAL..=OP_IF_GE => {
            // Handlers 0x474400 (sete), 0x474460 (setne), 0x4745e0 (setl),
            // 0x474520 (setg), 0x474580 (setle), 0x4744c0 (setge); the
            // jump is taken when the comparison holds (0x474340).
            let cmp = [Compare::Eq, Compare::Ne, Compare::Lt, Compare::Gt, Compare::Le, Compare::Ge][usize::from(ins.op - OP_IF_EQUAL)];
            let Some(target) = label(cd.sub(ins).into()) else { return Ok(None) };
            let o = cd.arg(ins) as u32 as usize;
            let (Some(a), Some(b)) = (pool_u32(cd, o), pool_u32(cd, o + 4)) else { return Ok(None) };
            return Ok(Some(Op::If { var: var(a), cmp, value: Operand::Const(b as i32), label: target }));
        }
        // Arithmetic on variables (sub = target variable). 0x03 shares
        // the "-= variable" handler in the engine's dispatch table.
        0x01..=0x0e => {
            let (op, by_var) = match ins.op {
                0x01 => (ArithOp::Set, false),
                0x02 => (ArithOp::Add, false),
                0x03 => (ArithOp::Sub, true),
                0x04 => (ArithOp::Mul, false),
                0x05 => (ArithOp::Div, false),
                0x06 => (ArithOp::BitAnd, false),
                0x07 => (ArithOp::BitOr, false),
                0x08 => (ArithOp::Set, true),
                0x09 => (ArithOp::Add, true),
                0x0a => (ArithOp::Sub, true),
                0x0b => (ArithOp::Mul, true),
                0x0c => (ArithOp::Div, true),
                0x0d => (ArithOp::BitAnd, true),
                _ => (ArithOp::BitOr, true),
            };
            let value = if by_var { Operand::Var(var(cd.arg(ins) as u32 & 0xffff)) } else { Operand::Const(cd.arg(ins)) };
            return Ok(Some(Op::Arith { var: var(cd.sub(ins).into()), op, value }));
        }
        _ => {}
    }
    Ok(None)
}

/// The cells a text line takes: the engine draws a full-width character
/// in a cell and a run of half-width ones two to a cell (a run of n takes
/// n / 2 rounded up). Half-width spaces count: the game's blog headers,
/// which fit its proportional font, take two rows here. Counted as the
/// script writer writes the line: the characters it makes full-width
/// (`escape_text` in `conv-cons`) count as such, and a line that starts
/// with ASCII gets a full-width space before it.
fn line_cells(line: &str) -> usize {
    const ESCAPED: &str = "@\\/#%$`!_\";*()<>[]?";
    let (mut cells, mut half) = (usize::from(line.starts_with(|c: char| c.is_ascii())), 0usize);
    for c in line.chars() {
        if (c.is_ascii() && !ESCAPED.contains(c)) || ('\u{ff61}'..='\u{ff9f}').contains(&c) {
            half += 1;
        } else {
            cells += half.div_ceil(2) + 1;
            half = 0;
        }
    }
    cells + half.div_ceil(2)
}

/// The rows a text line takes in a window `columns` cells wide, where
/// the engine wraps at the edge and starts each wrapped row `indent`
/// cells in (`indent`, a message's hanging indent).
fn line_rows(line: &str, columns: usize, indent: usize) -> usize {
    let (cells, columns) = (line_cells(line), columns.max(1));
    if cells <= columns {
        return 1;
    }
    1 + (cells - columns).div_ceil(columns.saturating_sub(indent).max(1))
}

/// The cells of a message's hanging indent: its first `hang` characters.
fn hang_cells(text: &str, hang: Option<usize>) -> usize {
    hang.map_or(0, |n| line_cells(&text.split('\n').next().unwrap_or_default().chars().take(n).collect::<String>()))
}

/// The `screenfilter` slot of a 0x56 fill over everything ([`colour_filter`]).
const TOP_FILL_FILTER_SLOT: u32 = 4;

/// 0xRRGGBB laid over the picture at `opacity`/256.
fn colour_filter(rgb: u32, opacity: i32) -> SpriteFilter {
    SpriteFilter {
        kind: Operand::Const(FILTER_COLOUR),
        blend: Operand::Const(FILTER_BLEND_COPY),
        strength: Operand::Const(opacity),
        params: [16, 8, 0].map(|s| Operand::Const((rgb >> s & 0xff) as i32)),
    }
}

/// The depth of an overlay (0x55, 0x58) drawn over the bust-ups, or `None`
/// for one under them: objects sort by layer, then depth (filmed under
/// wine): layer 0 and 1 are under the bust-ups, and layer 2 shares their
/// band, where most bust-ups sit at depth 10-150 and the overlays at 1000
/// (a few at 0-100, under them).
fn overlay_over(layer: u8, depth: u32) -> Option<u32> {
    (layer == 2 && depth > 150).then_some(depth)
}

/// The rows and columns of a window preset, and whether it is full-screen.
fn window_size_of(assets: &mut AssetBank, p: WindowPreset) -> Result<(usize, usize, bool), ConvertError> {
    let w = assets.text_window_style(p)?;
    Ok((w.rows as usize, w.columns as usize, p.full_screen))
}

/// One page of a message split by [`split_rows`]: its text and the links,
/// readings and styles that fall in it, moved to its own character range.
struct PagePart {
    text: String,
    links: Vec<TextLink>,
    ruby: Vec<Ruby>,
    styles: Vec<TextStyle>,
}

/// Splits a message taller than a full-screen window (`rows` by `columns`)
/// into pages at line ends. A page that would end past a separator line
/// (`━`, the blog's posts) ends before the last one instead, so a post
/// isn't cut in the middle when it can be helped. Links and styles that
/// cross a split are cut in two; a reading that crosses it is dropped.
fn split_rows(text: &str, links: &[TextLink], ruby: &[Ruby], styles: &[TextStyle], rows: usize, columns: usize, indent: usize) -> Vec<PagePart> {
    let lines: Vec<&str> = text.split('\n').collect();
    // Line ranges [first, end) of each page.
    let mut pages = Vec::new();
    let mut first = 0;
    while first < lines.len() {
        let (mut end, mut used) = (first, 0);
        while end < lines.len() && (end == first || used + line_rows(lines[end], columns, indent) <= rows) {
            used += line_rows(lines[end], columns, indent);
            end += 1;
        }
        if end < lines.len()
            && let Some(sep) = (first + 1..end).rev().find(|&i| lines[i].starts_with('━'))
            && sep > first + (end - first) / 2
        {
            end = sep;
        }
        pages.push((first, end));
        first = end;
    }
    // Character offset of each line's start.
    let mut starts = vec![0];
    for l in &lines {
        starts.push(starts.last().unwrap() + l.chars().count() + 1);
    }
    let clip = |s: usize, len: usize, a: usize, b: usize| {
        let (from, to) = (s.max(a), (s + len).min(b));
        (from < to).then(|| (from - a, to - from))
    };
    pages
        .into_iter()
        .map(|(first, end)| {
            let (a, b) = (starts[first], starts[end] - 1);
            PagePart {
                text: lines[first..end].join("\n"),
                links: links.iter().filter_map(|l| clip(l.start, l.len, a, b).map(|(start, len)| TextLink { start, len, ..l.clone() })).collect(),
                ruby: ruby.iter().filter(|r| r.start >= a && r.start + r.len <= b).map(|r| Ruby { start: r.start - a, ..r.clone() }).collect(),
                styles: styles.iter().filter_map(|s| clip(s.start, s.len, a, b).map(|(start, len)| TextStyle { start, len, ..*s })).collect(),
            }
        })
        .collect()
}

/// 0x54's `sub` 1-3 also puts background picture `sub + 5` (`frame01`-
/// `frame03`) on background slot 2, the overlay's (op_54, layer 1 depth 1:
/// under the bust-ups).
fn background_frame(sub: u16) -> Option<usize> {
    (1..=3).contains(&sub).then_some(usize::from(sub) + 5)
}

/// The instruction index a label number of this script is placed at, if
/// the script defines it.
fn own_label_at(cd: &CdScript, label: i32) -> Option<usize> {
    cd.labels
        .iter()
        .find(|l| l.script == cd.script_id && i64::from(l.label) == i64::from(label))
        .map(|l| l.code_offset as usize / 12)
}

/// Transition effect for 0x76's `sub`, named after the engine's
/// `Trans_*.cpp` source files (the handler at `0x449df0` picks the class
/// with a jump table on `sub`).
fn transition_name(sub: u16) -> &'static str {
    match sub {
        0..=3 => "Wave",
        4 => "Dot4x4",
        5..=8 => "SoftEdgeWipe",
        9 => "CrossFade",
        10..=17 => "Slide in",
        18..=25 => "Slide out",
        26 => "Dissolve",
        27 => "Mosaic",
        28..=31 => "BoxRolling",
        32..=35 => "RollFlip",
        36 => "SmoothCrossFade",
        37..=40 => "Zoom",
        41..=44 => "Scroll",
        45..=47 => "MeltDown",
        48 => "TvPowerSwitch",
        _ => "unknown",
    }
}

/// How 0x76's `sub` moves, for the families CONscripter can draw (the
/// rest are cross-fades). From the exe:
/// - Slide (`0x446f10`): `sub - 10` (in) or `sub - 18` (out) indexes the
///   direction tables at `0x5006d0`/`0x5006d8` (x, y). Sliding in, the new
///   picture starts one screen length that way and moves back into place;
///   sliding out, the old picture moves that way. Linear in time.
/// - Scroll (`0x446be0`): both pictures move together, `sub - 41` picks
///   one of four draw routines (table `0x518040`): the new picture enters
///   from below, the right, above, the left.
/// - Mosaic (`0x446160`): blocks up to 100 px (cubic), old then new.
/// - Zoom (`0x449800`): mode `sub - 37`, magnification `1 + f * t^5` with
///   f = 50 (f = 1.2 for 40), 16.16 fixed point about the centre; the
///   draw routines at `0x449160`, `0x449220`, `0x449270`, `0x449330`.
fn transition_style(sub: u16) -> TransitionStyle {
    const DIRECTIONS: [(i8, i8); 8] = [(0, 1), (-1, 1), (-1, 0), (-1, -1), (0, -1), (1, -1), (1, 0), (1, 1)];
    let zoom = |style, max_percent| TransitionStyle::Zoom { style, max_percent, curve: 5 };
    match sub {
        10..=17 => {
            let (dx, dy) = DIRECTIONS[usize::from(sub - 10)];
            TransitionStyle::Slide { moving: SlideLayer::New, dx: -dx, dy: -dy }
        }
        18..=25 => {
            let (dx, dy) = DIRECTIONS[usize::from(sub - 18)];
            TransitionStyle::Slide { moving: SlideLayer::Old, dx, dy }
        }
        41..=44 => {
            let (dx, dy) = [(0, -1), (-1, 0), (0, 1), (1, 0)][usize::from(sub - 41)];
            TransitionStyle::Slide { moving: SlideLayer::Both, dx, dy }
        }
        // Trans_Wave (setup 0x449020, draw functions from 0x51801c): the
        // new picture behind an edge whose last 16 px bands are revealed
        // in shrinking strips, a curtain. 0 top down, 1 right to left,
        // 2 bottom up, 3 left to right.
        0..=3 => {
            let (dx, dy) = [(0, 1), (-1, 0), (0, -1), (1, 0)][usize::from(sub)];
            TransitionStyle::Curtain { dx, dy }
        }
        // Trans_BoxRolling (setup 0x4427a0, step 0x442420, draw 0x442190;
        // filmed under wine, 29 and 31 slowed to 3 s): two faces of a cube
        // turning a quarter turn, seen straight on: the faces' outer edge
        // is at -(cos + sin) / 2 of the width (the step's sum), the shared
        // edge at (sin - cos) / 2, each picture squeezed to its face. 28
        // from the bottom, 29 from the left, 30 from the top, 31 from the
        // right.
        28..=31 => {
            let (dx, dy) = [(0, -1), (1, 0), (0, 1), (-1, 0)][usize::from(sub - 28)];
            TransitionStyle::Cube { dx, dy }
        }
        // Trans_RollFlip (setup 0x446660, draw 0x446390; size factor sin or
        // cos of progress × 90°, 0x446460): flag +0x90 the axis, +0x91
        // whether the new picture grows (filmed under wine: 32 grows
        // horizontally, 34 shrinks the old one horizontally).
        32..=35 => {
            let n = sub - 32;
            TransitionStyle::Flip { horizontal: n.is_multiple_of(2), grow: n < 2 }
        }
        // TvPowerSwitch (mostly cutting to black): a television switching
        // off, the old picture collapsing into a line (one frame of it
        // filmed under wine); drawn as a vertical flip.
        TRANSITION_TV_POWER_SWITCH => TransitionStyle::Flip { horizontal: false, grow: false },
        // Trans_MeltDown (setup 0x445e50, draw 0x4458e0; filmed 45 under
        // wine): a Doom-style melt. The old picture's columns fall away
        // behind a smooth random edge (a spline through 11 random points,
        // 0x445310, clipped at 0: flat stretches between bumps); the
        // filmed edge fits h * ((p - d) / (1 - d))^2 with d 0.2-0.3. The
        // byte 45 0, 46 128, 47 255 (not a colour) sets the per-row
        // acceleration (a/255)^2 * 0.45 + 0.03 (0x4451e0) that stretches
        // the falling columns.
        45..=47 => {
            let a = [0.0f64, 128.0, 255.0][usize::from(sub - 45)] / 255.0;
            TransitionStyle::Melt { stretch_percent: ((a * a * 0.45 + 0.03) * 500.0).round() as u32 }
        }
        27 => TransitionStyle::Mosaic,
        // Dot4x4: a dotted cross-fade; Dissolve passes through white (both
        // seen in the original under wine, 0004.cd #13 slowed to 8 s).
        TRANSITION_DOT4X4 | 5..=8 => TransitionStyle::Threshold,
        TRANSITION_DISSOLVE => TransitionStyle::ThroughWhite,
        37 => zoom(ZoomStyle::NewIn, 5100),
        38 => zoom(ZoomStyle::OldOut, 5100),
        39 => zoom(ZoomStyle::Through, 5100),
        40 => zoom(ZoomStyle::Cross, 220),
        _ => TransitionStyle::Fade,
    }
}

/// Resolves a label-table index (as used by 0x15/0x16/0x22 and links) to
/// the IR label name. Index 0 is the engine's implicit entry.
pub fn table_label(cd: &CdScript, index: i64) -> Option<String> {
    let stored = usize::try_from(index).ok()?.checked_sub(1)?;
    cd.labels.get(stored).map(|l| format!("s{}_l{}", l.script, l.label))
}

/// Opcodes whose `arg` is the offset of a pool record (the records lie end
/// to end in instruction order and fill the pool).
const POOL_RECORDS: [u16; 44] = [
    0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b, 0x36, 0x37, 0x38, 0x3b, 0x40, 0x41, 0x44, 0x46, 0x49, 0x4a, 0x55, 0x56, 0x59, 0x5a, 0x6d, 0x6f,
    0x70, 0x76, 0x77, 0x7a, 0x7b, 0x7c, 0x7d, 0x7f, 0x80, 0x81, 0x82, 0x83, 0x84, 0x86, 0x87, 0x88, 0x8a, 0x8b, 0x8c, 0x8d, 0x8e, 0x90,
];

/// Opcodes given `sub` 0xffff in every use, which their handler (`op_XX`
/// in `research/xuse/names.tsv`) never reads (no read of the
/// instruction's +2, there or in its callees two deep): the compiler's
/// filler for an operand the opcode has no use for. 0xffff there is not
/// given; any other value is noted.
const NO_SUB: &[u16] = &[
    0x00, 0x10, 0x23, 0x36, 0x37, 0x38, 0x39, 0x43, 0x44, 0x46, 0x47, 0x48, 0x49, 0x4a, 0x4c, 0x4d, 0x4e, 0x4f, 0x50, 0x55, 0x56, 0x59,
    0x5a, 0x66, 0x67, 0x68, 0x69, 0x6d, 0x6e, 0x6f, 0x70, 0x71, 0x72, 0x74, 0x77, 0x7a, 0x7b, 0x7c, 0x7d, 0x7f, 0x80, 0x82, 0x83, 0x84,
    0x86, 0x87, 0x88, 0x8a, 0x8b, 0x8c, 0x8d, 0x8f, 0x90, 0x91, 0x93, 0x94, 0x96, 0x97, 0x9d,
];

/// The same for `arg` -1 (no read of the instruction's +4).
const NO_ARG: &[u16] = &[0x00, 0x15, 0x22, 0x23, 0x39, 0x47, 0x48, 0x67, 0x69, 0x71, 0x8f, 0x94, 0x96, 0x99];

/// Operands the converter doesn't read because they don't matter:
/// (opcode, operand as [`CdScript::ignored`] names it, the values covered,
/// why). Values: `sub`/`arg` in decimal, record bytes in hex; `|`
/// separates values, `?` is any one digit, a trailing `*` any rest, `zeros`
/// any run of zero bytes. Another value is noted.
const UNNEEDED_OPERANDS: &[(u16, &str, &str, &str)] = &[
    // Message records (op_40_41 0x4771c0).
    (0x40, "+8", "*", "any: the source script's line number (`cd messages` prints it)"),
    (0x41, "+8", "*", "any: the source script's line number (`cd messages` prints it)"),
    (0x40, "-8", "0400000000000000", "u32 4, u32 0 after the text of every story message"),
    (0x41, "-8", "0400000000000000", "u32 4, u32 0 after the text of every message"),
    // Chapter labels (op_36 0x475460, op_37 0x475780, op_38 0x475970).
    (0x36, "-4", "*", "any: the label-table index of the chapter's |chpe label, where its 0x37 sits"),
    (0x37, "+0", "*", "any: its own label number: the label table places it"),
    (0x38, "+0", "*", "any: its own label number: the label table places it"),
    // op_3b 0x478650: only 0x3c reads the entry name, and no script uses 0x3c.
    (0x3b, "sub", "11|13", "the entry name's length"),
    (0x3b, "+0", "646562756720656e747279|72656c6561736520656e747279", "the entry name, \"debug entry\" or \"release entry\""),
    // op_43 0x4772a0 starts the window's fade-out and returns 2 for arg 0 (go on in the same frame), 0 otherwise (next frame).
    (0x43, "arg", "0|1", "neither waits for the fade-out: one frame apart, not converted (textoff goes on at once)"),
    // op_52 0x476cb0, op_53 0x476cd0 (both FUN_00476a80): sub picks the text window.
    (0x52, "sub", "0", "text window 0, the only one"),
    (0x53, "sub", "0", "text window 0, the only one"),
    // arg: the wait's read-flag number (kept at the window's +0x10; the wait 0x476750 marks it read with set_read_flag 0x4bb450).
    (0x52, "arg", "*", "any: the click wait's read-flag number; CONscripter's kidoku keys the wait by its place in 0.txt"),
    (0x53, "arg", "*", "any: the click wait's read-flag number; CONscripter's kidoku keys the wait by its place in 0.txt"),
    (0x6d, "-12", "zeros", "op_6d 0x442000 reads only speed, x and y range (y in EAX to quake_object)"),
    (0x76, "+4", "00000000", "flags 0 (op_76 0x449df0: bit 0 hides the windows, bits 0-1 remove the bust-ups): neither, as ours"),
    (
        0x77,
        "-8",
        "0000000000102700|0000000000e80300|0000000003000000|0000000003640000|0000000000000000",
        "flags 0 and the sort key (op_77 0x444090): the 9 records with a key below the top band change only what it covers (History, item 15)",
    ),
    // The full-screen animation, one object (sprite 59): op_7a 0x414590, op_7b 0x414820, op_8a 0x416200, op_8b 0x416310.
    (0x7a, "+1", "e8030000030a000000", "its depth, 1000"),
    (0x7a, "-1", "ff", "visible"),
    (0x7a, "+6", "d0070000", "a blog bust-up's depth in the top band, 2000: over everything else there"),
    // op_7a sets them as the object's size (+0x198, +0x19c, FUN_0040f060); 0x40f380 replaces it by the picture's once 0x84 loads one.
    (0x7a, "+14", "00000000", "the empty object's width and height, 0 x 0, until 0x84's picture sets its own"),
    // op_46 0x477460 puts it at +0 of the link entry; only the save (0x493290) and load read it back, not the drawing (0x493140) or the link callback (0x492bb0).
    (0x46, "-4", "00000000", "a field of the link entry nothing but the save reads"),
    (0x7b, "-4", "e8030000", "its depth, 1000"),
    (0x8a, "-9", "e803000001000000ff", "depth 1000, slot 1, 0xff"),
    (0x8b, "+1", "e80300000100000000000000", "its depth, 1000"),
    (0x83, "-1", "06", "the blend, dissolve (op_83 0x414050), which the converter draws as one"),
    (0x84, "-1", "00", "op_84 0x414180 never reads the record's last byte"),
    // Key-frame motions: op_86 0x413540, setup 0x416fe0; op_87 0x413820; op_8d 0x4155c0.
    (0x86, "+5", "01000000", "the module slot, 1: one motion per object"),
    (0x86, "+15", "zeros", "the first key frame, which the setup replaces by the object's state"),
    // 0x413350 hands the module the record from +9; its load (0x416fe0) reads the flags, the count and the key frames only.
    (0x86, "-2", "0000", "two zero bytes after the key frames, which the motion's load doesn't read"),
    (0x87, "+0", "02????????01000000ff", "layer 2, any depth, slot 1: waits for the motion its object's 0x86 just started, which ours runs to its end at 0x86"),
    (0x8d, "-1", "ff", "0xff, as in every 0x87 and 0x8a wait record"),
    // op_88 0x415b50 clears a slot-0 module without its blend and opacity.
    (0x88, "-4", "00000000", "a cleared slot's blend and opacity"),
    // op_70 0x415a30 likewise clears a kind-0 or slot-0 filter without them.
    (0x70, "-4", "00000000", "a cleared slot's blend and opacity"),
    // Filter-Zigzag's draw (0x41dab0) passes the frequency as the phase and never reads these.
    (0x70, "-8", "0a00000000000000", "Zigzag's begin angle (10) and reduction (0)"),
    // op_56 0x4224a0 passes it to object_create as the sort depth in the layer's band.
    (0x56, "+2", "00000000|d0070000", "a fill's depth (0, 2000) in a band where nothing else is"),
    // op_88 sets the module's +0xc, the key FUN_0040e2a0 keeps a preset's modules in (ascending, a tie after the earlier ones).
    (
        0x88,
        "+8",
        "00000000|02000000|e8030000|d0070000",
        "the module's place in its preset (0 in 41; 2, 1000 twice, 2000 in slot 2 or a lone slot 1): the modules set together are in slot order, as ours draws them",
    ),
    // op_90 0x434960 sets the new preset module's +0xc, the key FUN_0040e2a0 sorts the object's modules by (list at +0x68).
    (
        0x90,
        "+2",
        "000001000000|000000000000|000002000000",
        "the preset module's place among the bust-up's modules (1; 0 and 2 once each): it is the only one there (0x8e's module names are empty, no 0x86 motion on that character)",
    ),
    // op_8e 0x4347b0: the module name, empty (byte 0 is 0), leftover memory after it.
    (0x8e, "+12", "00*", "an empty module name"),
    (0x8e, "+56", "00*", "an empty module name"),
    (0x8e, "+100", "00*", "an empty module name"),
    (0x8e, "+144", "00*", "an empty module name"),
    (0x8e, "+188", "00*", "an empty module name"),
    (0x8e, "-32", "00*", "an empty module name"),
];

/// The operands given but never read ([`CdScript::ignored`]) and not in
/// [`UNNEEDED_OPERANDS`]: (instruction index, "<opcode> operand <which> ignored").
pub fn ignored_operands(cd: &CdScript) -> Vec<(usize, String)> {
    cd.ignored(&POOL_RECORDS)
        .into_iter()
        .filter(|u| {
            let filler = match u.which.as_str() {
                "sub" => NO_SUB.contains(&u.op) && u.value == "65535",
                "arg" => NO_ARG.contains(&u.op) && u.value == "-1",
                _ => false,
            };
            !filler && !UNNEEDED_OPERANDS.iter().any(|&(o, w, v, _)| o == u.op && w == u.which && value_matches(v, &u.value))
        })
        .map(|u| (u.index, format!("{:#04x} operand {} ignored", u.op, u.which)))
        .collect()
}

/// Whether `value` is one of `pattern`'s ([`UNNEEDED_OPERANDS`]).
fn value_matches(pattern: &str, value: &str) -> bool {
    pattern.split('|').any(|p| match p {
        "zeros" => !value.is_empty() && value.bytes().all(|b| b == b'0'),
        _ => {
            let (p, rest) = p.strip_suffix('*').map_or((p, false), |p| (p, true));
            (value.len() == p.len() || rest && value.len() > p.len())
                && p.bytes().zip(value.bytes()).all(|(p, v)| p == b'?' || p == v)
        }
    })
}

fn pool_u32(cd: &CdScript, offset: usize) -> Option<u32> {
    cd.bytes(offset, 4).map(|b| u32::from_le_bytes(b.try_into().unwrap()))
}

/// The image at half brightness. The log base is a white-to-black gradient;
/// white text over its top half was hard to read.
/// `number` (a background, 1-based) if the CG gallery shows it.
fn gallery_cg(number: i32) -> Option<u32> {
    let bit = u16::try_from(number.checked_sub(1)?).ok()?;
    gallery::TABS.iter().flat_map(|t| t.iter()).flat_map(|p| p.iter()).any(|s| s.contains(&bit)).then_some(number as u32)
}

/// Cells `which` of a column of `count` cells, side by side.
fn column_cells(img: &image::RgbaImage, count: u32, which: &[u32]) -> image::RgbaImage {
    let (w, h) = (img.width(), img.height() / count);
    let mut out = image::RgbaImage::new(w * which.len() as u32, h);
    for (i, &c) in which.iter().enumerate() {
        let cell = image::imageops::crop_imm(img, 0, c * h, w, h).to_image();
        image::imageops::replace(&mut out, &cell, i64::from(w) * i as i64, 0);
    }
    out
}

/// A gallery thumbnail as two cells: alone, and under the pointer with
/// `frame` (black where nothing is added: the game adds it to the
/// screen) 4 px up and left of it. Over the transparent margin the frame
/// becomes its own colour with opacity = its brightest channel.
fn thumbnail_cells(thumb: &image::RgbaImage, frame: &image::RgbaImage) -> image::RgbaImage {
    const OFFSET: u32 = 4;
    let w = (thumb.width() + OFFSET).max(frame.width());
    let h = (thumb.height() + OFFSET).max(frame.height());
    let mut out = image::RgbaImage::new(w * 2, h);
    image::imageops::replace(&mut out, thumb, i64::from(OFFSET), i64::from(OFFSET));
    image::imageops::replace(&mut out, thumb, i64::from(w + OFFSET), i64::from(OFFSET));
    for (x, y, f) in frame.enumerate_pixels() {
        let px = out.get_pixel_mut(w + x, y);
        let a = f32::from(px.0[3]) / 255.0;
        let add = f.0[..3].iter().copied().max().unwrap_or(0);
        if add == 0 {
            continue;
        }
        let alpha = a.max(f32::from(add) / 255.0);
        for c in 0..3 {
            let premultiplied = f32::from(px.0[c]) * a + f32::from(f.0[c]);
            px.0[c] = (premultiplied / alpha).min(255.0) as u8;
        }
        px.0[3] = (alpha * 255.0).round() as u8;
    }
    out
}

/// `img` with its opacity scaled to `percent`.
fn with_opacity(mut img: image::RgbaImage, percent: u32) -> image::RgbaImage {
    for px in img.pixels_mut() {
        px.0[3] = (u32::from(px.0[3]) * percent / 100) as u8;
    }
    img
}

fn darker(mut img: image::RgbaImage) -> image::RgbaImage {
    for px in img.pixels_mut() {
        for c in &mut px.0[..3] {
            *c /= 2;
        }
    }
    img
}

/// `top` over `base`, the base made translucent like the text window's.
fn blend_layers(mut base: image::RgbaImage, top: &image::RgbaImage) -> image::RgbaImage {
    for px in base.pixels_mut() {
        px.0[3] = (u32::from(px.0[3]) * WINDOW_OPACITY_PERCENT / 100) as u8;
    }
    image::imageops::overlay(&mut base, top, 0, 0);
    base
}

fn png_size(png: &[u8]) -> (u32, u32) {
    let be = |o: usize| png.get(o..o + 4).map_or(0, |b| u32::from_be_bytes(b.try_into().unwrap()));
    (be(16), be(20))
}

/// `dir/name`, made unique if another asset already used that name.
fn unique(used: &mut HashSet<String>, dir: &str, name: &str, number: usize) -> String {
    let safe: String = name
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-') { c } else { '_' })
        .collect();
    let mut candidate = format!("{dir}/{safe}");
    if !used.insert(candidate.clone()) {
        candidate = format!("{dir}/{number}_{safe}");
        used.insert(candidate.clone());
    }
    candidate
}

fn write_file(path: &Path, data: &[u8]) -> Result<(), ConvertError> {
    let io = |source| ConvertError::Io { path: path.to_path_buf(), source };
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(io)?;
    }
    fs::write(path, data).map_err(io)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transition_styles_follow_the_exe() {
        let slide = |moving, dx, dy| TransitionStyle::Slide { moving, dx, dy };
        // Sub 10 (direction 0, +y): the new picture starts below and moves up.
        assert_eq!(transition_style(10), slide(SlideLayer::New, 0, -1));
        assert_eq!(transition_style(12), slide(SlideLayer::New, 1, 0));
        assert_eq!(transition_style(24), slide(SlideLayer::Old, 1, 0));
        assert_eq!(transition_style(41), slide(SlideLayer::Both, 0, -1));
        assert_eq!(transition_style(44), slide(SlideLayer::Both, 1, 0));
        assert_eq!(transition_style(40), TransitionStyle::Zoom { style: ZoomStyle::Cross, max_percent: 220, curve: 5 });
        assert_eq!(transition_style(9), TransitionStyle::Fade);
        // Wave: 0 top down, 1 right to left.
        assert_eq!(transition_style(0), TransitionStyle::Curtain { dx: 0, dy: 1 });
        assert_eq!(transition_style(1), TransitionStyle::Curtain { dx: -1, dy: 0 });
        assert_eq!(transition_style(26), TransitionStyle::ThroughWhite);
        assert_eq!(transition_style(4), TransitionStyle::Threshold);
        assert_eq!(transition_style(7), TransitionStyle::Threshold);
        assert_eq!(transition_style(29), TransitionStyle::Cube { dx: 1, dy: 0 });
        assert_eq!(transition_style(32), TransitionStyle::Flip { horizontal: true, grow: true });
        assert_eq!(transition_style(35), TransitionStyle::Flip { horizontal: false, grow: false });
        assert_eq!(transition_style(45), TransitionStyle::Melt { stretch_percent: 15 });
        assert_eq!(transition_style(47), TransitionStyle::Melt { stretch_percent: 240 });
        assert_eq!(transition_name(20), "Slide out");
    }

    #[test]
    fn windows_follow_the_callers() {
        let ins = |op, sub, arg| Instr { op, sub, arg, source_pos: 0 };
        // Labels 0-4 of script 7 (the one `build` writes) (table indices 1-5): the start, a
        // full-screen and a message box preset, a footnote that sets the
        // full-screen window, and a scene after it that a jump enters with
        // the message box.
        let labels: Vec<crate::xuse::cd::Label> =
            [0, 4, 8, 12, 15].iter().enumerate().map(|(l, &i)| crate::xuse::cd::Label { script: 7, label: l as u32, code_offset: i * 12 }).collect();
        let code = [
            ins(OP_GOTO, 0, 0),
            ins(OP_GOSUB, 3, 0),
            ins(OP_TEXT_DISPLAY, 0, 0),
            ins(OP_GOTO, 0, 4),
            ins(OP_GOTO, 0, 1),
            ins(OP_WINDOW_MODE, 0, 1),
            ins(OP_WINDOW_POS, 0, 0),
            ins(OP_RETURN, 0, 0),
            ins(OP_GOTO, 0, 2),
            ins(OP_WINDOW_MODE, 0, 0),
            ins(OP_WINDOW_POS, 0, 89 | 462 << 16),
            ins(OP_RETURN, 0, 0),
            ins(OP_GOTO, 0, 3),
            ins(OP_GOSUB, 2, 0),
            ins(OP_RETURN, 0, 0),
            ins(OP_GOTO, 0, 4),
            ins(OP_RETURN, 0, 0),
        ];
        let bytes = crate::xuse::cd::tests::build("w.rio", &labels, &code, &[]);
        let scripts = vec![("w.cd".to_string(), CdScript::parse(&bytes).unwrap())];
        let p = presets(&scripts).unwrap();
        let full = p.window["s7_l1"];
        let message = p.window["s7_l2"];
        assert!(full.full_screen && !message.full_screen);
        assert_eq!(p.sub_window.get("s7_l3"), Some(&full));
        // In file order s1_l4 would get the footnote's window.
        assert!(!falls_into(&scripts[0].1, 16));
        assert_eq!(p.entry_window.get("s7_l4"), Some(&message));
    }

    #[test]
    fn script_starts_are_not_fallen_into() {
        let ins = |op, sub, arg| Instr { op, sub, arg, source_pos: 0 };
        // Labels 0 and 1 start the script; 2 and 3 follow a gosub.
        let labels: Vec<crate::xuse::cd::Label> =
            [0, 1, 3, 4].iter().enumerate().map(|(l, &i)| crate::xuse::cd::Label { script: 7, label: l as u32, code_offset: i * 12 }).collect();
        let code = [ins(OP_GOTO, 0, 0), ins(OP_GOTO, 0, 1), ins(OP_GOSUB, 1, 0), ins(OP_GOTO, 0, 2), ins(OP_GOTO, 0, 3), ins(OP_RETURN, 0, 0)];
        let bytes = crate::xuse::cd::tests::build("w.rio", &labels, &code, &[]);
        let cd = CdScript::parse(&bytes).unwrap();
        assert!(!falls_into(&cd, 1));
        assert!(!falls_into(&cd, 2));
        assert!(falls_into(&cd, 4));
        assert!(falls_into(&cd, 5));
    }

    #[test]
    fn chapter_titles() {
        assert_eq!(chapter_title("chapterさやか]?空白の机"), Some("空白の机"));
        assert_eq!(chapter_title("chapterさやか]１?ビジョン"), Some("ビジョン"));
        assert_eq!(chapter_title("blog?あの場所"), None);
    }

    #[test]
    fn unique_names() {
        let mut used = HashSet::new();
        assert_eq!(unique(&mut used, "bg", "a b.png", 3), "bg/a_b.png");
        assert_eq!(unique(&mut used, "bg", "a b.png", 9), "bg/9_a_b.png");
        assert_eq!(unique(&mut used, "sprite", "背景.png", 1), "sprite/__.png");
    }

    #[test]
    fn presets_live_in_variables() {
        let at = || SourceLoc { file: "t".into(), index: 0 };
        let set = |var: Var, n: i32| Op::Arith { var, op: ArithOp::Set, value: Operand::Const(n) };
        let tint = PresetModule {
            preset: 2,
            slot: 1,
            kind: PRESET_KIND_COLOUR,
            blend: PRESET_BLEND_MULTI,
            opacity: 200,
            data: ModuleData::Colour([0x64, 0, 0]),
        };
        let mut ops = Vec::new();
        lower_preset_module(&tint, &mut ops, at);
        let v = |field| preset_var(2, 1, field);
        let expected =
            [(0, FILTER_COLOUR), (1, FILTER_BLEND_MULTIPLY), (2, 200), (3, 0x64), (4, 0), (5, 0)].map(|(f, n)| set(v(f), n));
        assert_eq!(ops, expected);
        // Preset 2 slot 1 comes after preset 1's three slots.
        assert_eq!(v(0).index, PRESET_VAR_BASE + 18);

        ops.clear();
        lower_preset_module(&PresetModule { slot: 0, ..tint }, &mut ops, at);
        assert_eq!(ops, (1..=3).map(|s| set(preset_var(2, s, 0), 0)).collect::<Vec<_>>());

        ops.clear();
        lower_preset_module(&PresetModule { kind: 12, data: ModuleData::None, ..tint }, &mut ops, at);
        assert!(matches!(ops[..], [Op::Unsupported { .. }, Op::Arith { .. }]));

        assert_eq!(preset_filters(0), Some(vec![]));
        assert_eq!(preset_filters(3).unwrap().len(), 3);
        assert_eq!(preset_filters(4), None);
    }

    #[test]
    fn unneeded_values() {
        assert!(value_matches("11|13", "13") && !value_matches("11|13", "1"));
        assert!(value_matches("02??01", "02ab01") && !value_matches("02??01", "02ab0100"));
        assert!(value_matches("00*", "006f6e") && !value_matches("00*", "01"));
        assert!(value_matches("zeros", "0000") && !value_matches("zeros", "0010") && !value_matches("zeros", ""));
    }

    #[test]
    fn link_ranges() {
        let segs: Vec<String> = ["名前", "「一行目です", "二行目」"].iter().map(|s| s.to_string()).collect();
        let link = |row, column, length| Link { row, column, length, style: 0x8007, target: 0 };
        // Row 2 is the first text line (row 1 is the name).
        assert_eq!(link_range(&segs, true, &link(2, 2, 3)), Some((1, 3)));
        assert_eq!(link_range(&segs, true, &link(3, 1, 2)), Some((7, 2)));
        assert_eq!(link_range(&segs, true, &link(1, 1, 2)), None);
        assert_eq!(link_range(&segs, true, &link(3, 3, 5)), None);
        assert_eq!(link_range(&segs[1..], false, &link(1, 2, 3)), Some((1, 3)));
    }

    #[test]
    fn rows_count_half_width_runs_two_to_a_cell() {
        assert_eq!(line_rows("　", 40, 0), 1);
        assert_eq!(line_rows(&"━".repeat(40), 40, 0), 1);
        // 26 full-width characters and 31 half-width ones (16 cells).
        assert_eq!(line_rows("ほっとした  投稿者:ToolSmith004 ／どらいばあ（米）  投稿日: 滅亡元年／06／28 22:32 ", 40, 0), 2);
        assert_eq!(line_rows("あab", 2, 0), 1);
        assert_eq!(line_rows("あabc", 2, 0), 2);
        // As the converter has it, before '/' and '(' become full-width.
        assert_eq!(line_rows("ほっとした  投稿者:ToolSmith004/どらいばあ(米)  投稿日:滅亡元年/06/28 22:32", 40, 0), 2);
        assert_eq!(line_rows("あa/", 2, 0), 2);
        // Wrapped rows start at the hanging indent: 8 cells in rows of 4,
        // then 2 after an indent of 2.
        assert_eq!(line_rows("あいうえおかきく", 4, 0), 2);
        assert_eq!(line_rows("あいうえおかきく", 4, 2), 3);
        assert_eq!(hang_cells("ab　あい\nx", Some(3)), 3);
        // 最果てのイマ's chat log (conv_link35_1): written 「　NIN-NIN 　…」.
        assert_eq!(line_cells("NIN-NIN　数としては、してるけど……その、実際には生存している人とかがいるかなと"), 41);
    }

    #[test]
    fn overlays_sort_by_layer_then_depth() {
        assert_eq!(overlay_over(2, 1000), Some(1000));
        assert_eq!(overlay_over(2, 1), None);
        assert_eq!(overlay_over(0, 1000), None);
    }

    #[test]
    fn tall_messages_split_at_posts() {
        // 2 rows of 4 columns: "ab" "━━" "cdef" on the first page would be
        // 3 rows; the page ends before the separator instead of after it.
        let text = "ab\nxy\n━━\ncdefgh";
        let link = TextLink { start: 1, len: 7, target: Some("t".into()), scene: None };
        let ruby = [Ruby { start: 0, len: 2, text: "r".into() }, Ruby { start: 4, len: 3, text: "s".into() }];
        let style = TextStyle { start: 9, len: 4, colour: Some(1), ..Default::default() };
        let parts = split_rows(text, std::slice::from_ref(&link), &ruby, &[style], 3, 4, 0);
        let texts: Vec<&str> = parts.iter().map(|p| p.text.as_str()).collect();
        assert_eq!(texts, ["ab\nxy", "━━\ncdefgh"]);
        assert_eq!((parts[0].links[0].start, parts[0].links[0].len), (1, 4));
        assert_eq!((parts[1].links[0].start, parts[1].links[0].len), (0, 2));
        assert_eq!(parts[0].ruby.len(), 1);
        assert!(parts[1].ruby.is_empty());
        assert_eq!((parts[1].styles[0].start, parts[1].styles[0].len), (3, 4));
    }

    #[test]
    fn ruby_ranges() {
        let body: Vec<String> = ["　吹く。", "　戦ぐ。"].iter().map(|s| s.to_string()).collect();
        let ruby = |line, column, length| RubyRecord { line, column, length, text: "そよ".into() };
        // Lines count text lines only; '\n' counts as one character.
        assert_eq!(ruby_range(&body, &ruby(2, 2, 1)), Some((6, 1)));
        assert_eq!(ruby_range(&body, &ruby(1, 2, 3)), Some((1, 3)));
        assert_eq!(ruby_range(&body, &ruby(1, 3, 3)), None);
        assert_eq!(ruby_range(&body, &ruby(3, 1, 1)), None);
        assert_eq!(ruby_range(&body, &ruby(0, 1, 1)), None);
    }

    #[test]
    fn text_style_ranges() {
        let body: Vec<String> = ["「牛めし", "　……カルビ"].iter().map(|s| s.to_string()).collect();
        let style = |line, column, length, size, colour| StyleRecord { line, column, length, size, colour };
        // 30000 runs to the end of the message; sizes below 0x1000 are percent.
        assert_eq!(
            text_style(&body, &style(2, 1, 30000, 94, STYLE_COLOUR_KEEP)),
            Some(TextStyle { start: 5, len: 6, size_percent: Some(94), ..Default::default() })
        );
        assert_eq!(text_style(&body, &style(1, 2, 30000, 94, STYLE_COLOUR_KEEP)).map(|s| (s.start, s.len)), Some((1, 10)));
        // COLORREF 0x00BBGGRR; 0xf000 keeps the size.
        assert_eq!(
            text_style(&body, &style(1, 2, 2, STYLE_SIZE_KEEP, 0x00d3_55ba)),
            Some(TextStyle { start: 1, len: 2, colour: Some(0xba55d3), ..Default::default() })
        );
        assert_eq!(
            text_style(&body, &style(1, 1, 1, 0x1006, STYLE_COLOUR_KEEP)).and_then(|s| s.size_percent),
            Some(150)
        );
        assert_eq!(text_style(&body, &style(1, 1, 30000, STYLE_SIZE_KEEP, STYLE_COLOUR_KEEP)), None);
        assert_eq!(text_style(&body, &style(3, 1, 1, 90, STYLE_COLOUR_KEEP)), None);
        assert_eq!(text_style(&body, &style(1, 6, 1, 90, STYLE_COLOUR_KEEP)), None);
    }

    #[test]
    fn module_filters() {
        use ModuleData as D;
        assert_eq!(module_filter(PRESET_KIND_BRIGHTNESS, 5, 256, D::Brightness(-200), None), Ok([4, 0, 256, -200, 0, 0]));
        assert_eq!(
            module_filter(PRESET_KIND_MONOTONE, 5, 300, D::Palette(1), Some([28, -8, -38])),
            Ok([5, 0, 256, 28, -8, -38])
        );
        assert_eq!(module_filter(PRESET_KIND_COLOUR, 2, 128, D::Colour([1, 2, 3]), None), Ok([1, 1, 128, 1, 2, 3]));
        assert_eq!(module_filter(PRESET_KIND_SMOOTH, 5, 256, D::Smooth(3, 9), None), Ok([6, 0, 256, 3, 6, 0]));
        assert_eq!(
            module_filter(PRESET_KIND_HUE_SATURATION, 5, 256, D::HueSaturation(0, -128), None),
            Ok([7, 0, 256, 0, -128, 0])
        );
        assert_eq!(module_filter(PRESET_KIND_BRIGHTNESS_INVERT, 5, 256, D::None, None), Ok([8, 0, 256, 0, 0, 0]));
        assert!(module_filter(12, 2, 100, D::None, None).is_err()); // Zigzag
        assert!(module_filter(PRESET_KIND_GREYSCALE, 4, 256, D::None, None).is_err());
    }

    #[test]
    fn quakes_shake_and_wait() {
        let mut ops = Vec::new();
        lower_quake(&Quake { speed: 80, range: (10, 20) }, &mut ops);
        // 4 points at 30 a second at the full range, then shrinking 20 px
        // at 50 px a second.
        let shake = |time_ms, to| ScreenEffect::Shake { time_ms, interval_ms: 10, path: true, speed: (1966, 0), range: [(10, 20), to], end_ms: 0 };
        assert_eq!(
            ops,
            [
                Op::ScreenEffect { slot: QUAKE_SLOT, effect: Some(shake(133, (10, 20))) },
                Op::Wait { ms: 133 },
                Op::ScreenEffect { slot: QUAKE_SLOT, effect: Some(shake(400, (0, 0))) },
                Op::Wait { ms: 400 },
                Op::ScreenEffect { slot: QUAKE_SLOT, effect: None },
            ]
        );
        ops.clear();
        lower_quake(&Quake { speed: 80, range: (0, 0) }, &mut ops);
        assert!(ops.is_empty());
    }

    #[test]
    fn animation_scripts() {
        let ap = "fx0401;\r\n0 0;\r\n0;\r\n5;\t\t\t\t# default\r\n3;\r\n1  33 0000;\r\n2  50 0000;\r\n1  33 0000;\r\n\r\n";
        let s = parse_animation_script(ap).unwrap();
        assert!(!s.looped);
        assert_eq!(s.frames, [(1, 33), (2, 50), (1, 33)]);
        assert!(parse_animation_script(&ap.replace("3;", "4;")).is_none());
        assert!(parse_animation_script("fx;\n0 0;\n1;\n1;\n1;\n0 33 0;\n").is_none());
    }

    #[test]
    fn surprises_pulse_and_wait() {
        let mut ops = Vec::new();
        lower_surprise(&Surprise { time_ms: 300, preset: 4 }, &mut ops);
        let pulse = ScreenEffect::Pulse {
            time_ms: 300,
            interval_ms: 10,
            zoom: (150, 150),
            opacity: 150,
            colour: 0xff8080,
            tint_opacity: 256,
            tint: 1,
        };
        assert_eq!(
            ops,
            [
                Op::ScreenEffect { slot: PULSE_SLOT, effect: Some(pulse) },
                Op::Wait { ms: 300 },
                Op::ScreenEffect { slot: PULSE_SLOT, effect: None },
            ]
        );
        ops.clear();
        lower_surprise(&Surprise { time_ms: 300, preset: 0 }, &mut ops);
        assert!(matches!(ops[0], Op::ScreenEffect { effect: Some(ScreenEffect::Pulse { zoom: (100, 100), tint: 0, tint_opacity: 0, .. }), .. }));
    }

    #[test]
    fn hanging_indent() {
        assert_eq!(hang_char("Ghost　忍、いるか！？", 7), Some(6));
        assert_eq!(hang_char("θ　すべて", 3), Some(2));
        assert_eq!(hang_char("Ghost　", 7), None);
        assert_eq!(hang_char("Ghost", 0), None);
    }

    #[test]
    fn reads_png_size() {
        let mut png = vec![0u8; 24];
        png[16..20].copy_from_slice(&348u32.to_be_bytes());
        png[20..24].copy_from_slice(&700u32.to_be_bytes());
        assert_eq!(png_size(&png), (348, 700));
        assert_eq!(png_size(&[]), (0, 0));
    }
}
