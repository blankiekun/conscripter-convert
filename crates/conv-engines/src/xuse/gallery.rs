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
//! thumbnails. [`TABS`] is the exe's table (0x500118: per tab, pages of
//! (count, slots); a slot is 64 bytes: thumbnail index, then up to 15 CG
//! bits, 0-ended). A CG bit is the background number of 0x54 minus 1; a
//! slot shows its thumbnail once any of its CGs is seen (0x44cd30) and
//! the viewer steps through the seen ones. Thumbnails come from
//! `SubModeThumb-<tab>-<page>.png`, a grid of 105 × 81 cells in slot order.
//! The viewer has a voice button (`HD/cgv.bin`, not converted) and a
//! check box of unknown use.

/// Per tab (1-based in the file names), pages of slots of CG bits.
pub const TABS: [&[&[&[u16]]]; 7] = [
    // Tab 1: あずさ
    &[
        &[
            &[212],
            &[213, 214, 215, 216, 217, 218, 219, 220, 221, 222, 223, 224, 225, 226, 227],
            &[228, 229, 230],
            &[231, 232],
            &[233, 234, 235],
            &[236, 237],
            &[238],
            &[239],
            &[240],
            &[241],
            &[242, 243],
            &[454],
            &[244, 245, 246, 247],
        ],
    ],
    // Tab 2: 沙也加
    &[
        &[
            &[248, 249, 250, 251],
            &[252, 253, 518],
            &[254],
            &[255, 256],
            &[257],
            &[258],
            &[259, 260],
            &[261, 262],
            &[263],
            &[264, 265, 266],
            &[267, 268, 269, 270],
            &[271, 272, 273],
            &[274],
            &[275, 276],
            &[277],
            &[453],
            &[291, 292, 293, 294],
            &[283, 284],
            &[285, 286],
            &[278, 519, 279, 520],
        ],
        &[
            &[280, 281, 282],
            &[287, 288, 289, 290],
        ],
    ],
    // Tab 3: 葉子
    &[
        &[
            &[295],
            &[296, 297, 298],
            &[299],
            &[300, 301],
            &[302],
            &[303, 304, 305, 306],
            &[307, 308, 309, 310, 311],
            &[312, 313, 314],
            &[315, 316],
            &[317, 318],
            &[319, 320],
            &[321, 322],
            &[451, 452],
            &[323, 324, 325],
            &[326, 327, 328, 329],
        ],
    ],
    // Tab 4: 笛子
    &[
        &[
            &[330, 331],
            &[332, 333],
            &[334, 335],
            &[336],
            &[337, 338, 339, 340, 341],
            &[342],
            &[343],
            &[344],
            &[345],
            &[346, 347, 348],
            &[349],
            &[350, 351, 352, 353],
            &[354],
            &[355, 521],
            &[356, 357],
            &[358, 359],
            &[360, 361],
            &[443, 444, 445, 446, 447, 448],
            &[449, 450],
            &[362, 363, 364, 365],
        ],
        &[
            &[366, 367, 368, 369],
        ],
    ],
    // Tab 5: no pages
    &[
    ],
    // Tab 6: no pages
    &[
    ],
    // Tab 7: その他
    &[
        &[
            &[416],
            &[417],
            &[418, 419],
            &[420, 421],
            &[422],
            &[423, 424, 425, 426],
            &[427, 428, 429, 430, 431, 432, 433, 434, 435, 436, 437],
            &[438],
            &[439, 440],
            &[441, 442],
            &[455],
            &[456, 457, 458, 459, 460],
            &[461, 462],
            &[463, 464, 465],
            &[466, 522, 467, 468, 523],
            &[469, 470, 471, 472, 473, 474],
            &[475],
            &[476, 477],
            &[478, 479, 480],
            &[481, 482, 483, 484],
        ],
        &[
            &[485],
            &[486, 487, 488],
            &[489],
            &[490],
            &[491, 492, 493, 494, 495],
            &[496, 497, 498, 499],
            &[500, 501, 502, 503],
            &[504, 505],
            &[511, 512],
            &[513],
            &[514, 515, 516, 517],
            &[370, 371, 372, 373, 374],
            &[375, 376, 377, 378, 379],
            &[384, 385, 386, 387, 388, 389, 390],
            &[391, 392, 393, 394, 395, 396],
            &[401, 402, 403, 404, 405],
            &[406, 407, 408, 409, 410],
            &[506, 507, 508, 509],
            &[510],
            &[380, 381, 382, 383],
        ],
        &[
            &[397, 398, 399, 400],
            &[411, 412, 413, 414, 415],
        ],
    ],
];

/// Tab order on screen and the pictures of each: (tab index in [`TABS`],
/// label `SubModeCommon.004` #, heading #, viewer name `CgViewer.004` #).
pub const TAB_PICTURES: [(usize, usize, usize, usize); 5] =
    [(0, 0, 10, 0), (1, 1, 11, 3), (2, 4, 14, 4), (3, 3, 13, 1), (6, 2, 12, 2)];

/// Size of a cell of the thumbnail sheets.
pub const THUMB_W: u32 = 105;
pub const THUMB_H: u32 = 81;
