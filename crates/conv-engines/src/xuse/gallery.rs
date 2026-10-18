//! The extras behind the title's option button (`Title.cpp`, `CGMode.cpp`;
//! pictures in `HD/gd.arc`), seen under wine with a cleared save.
//!
//! Option opens a menu over `SubModeCommon.004` #19 whatever the progress:
//! cg, bgm, chapter, wss and exit (`Title.004` #4, 5, 8, 13, 3) under an
//! "option" heading (#12, not a button). cg and wss are disabled until
//! something unlocks them (cg: a CG of the gallery seen). The original
//! has no settings on the title.
//!
//! The CG gallery: tabs (one per heroine) of pages of up to 5 × 4
//! thumbnails. [`super::exe::Tables::tabs`] is the exe's table (0x500118: per tab, pages of
//! (count, slots); a slot is 64 bytes: thumbnail index, then up to 15 CG
//! bits, 0-ended). A CG bit is the background number of 0x54 minus 1; a
//! slot shows its thumbnail once any of its CGs is seen (0x44cd30) and
//! the viewer steps through the seen ones. Thumbnails come from
//! `SubModeThumb-<tab>-<page>.png`, a grid of 105 × 81 cells in slot order.
//! The viewer has a voice button (`HD/cgv.bin`, not converted) and a
//! check box of unknown use.

/// Tab order on screen and the pictures of each: (tab index in the exe's tabs,
/// label `SubModeCommon.004` #, heading #, viewer name `CgViewer.004` #).
// src: shot (wine, the tab order)
pub const TAB_PICTURES: [(usize, usize, usize, usize); 5] =
    [(0, 0, 10, 0), (1, 1, 11, 3), (2, 4, 14, 4), (3, 3, 13, 1), (6, 2, 12, 2)];

/// Size of a cell of the thumbnail sheets.
// src: the sheets (5 cells a row)
pub const THUMB_W: u32 = 105;
pub const THUMB_H: u32 = 81;
