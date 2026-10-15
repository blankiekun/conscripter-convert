//! 続・殺戮のジャンゴ -地獄の賞金首- (Nitroplus, 2008)'s own additions to
//! the NSS compiler ([`DJANGO`]): an 800×600 screen, the boot script
//! `boot.nss`, whose `#include`s are the helper library, and the story
//! driver compiled whole.

use super::Game;

/// The game's hooks; the CLI uses them for a game with `Django.exe`.
pub const DJANGO: Game = Game { screen: (800, 600), boot: "boot.nss", story: Some((STORY, "SelectStoryMode")), ..Game::PLAIN };

/// The story driver: its `SelectStoryMode` (what `main` runs unless
/// `$SelectDebug`, the developers' chapter select, which the boot script
/// clears) calls `main_root`, which calls the 56 chapter files in turn
/// and ends the game after a death (a bad end).
const STORY: &str = "dja000_開始スクリプト.nss";
