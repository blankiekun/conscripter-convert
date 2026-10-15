//! 続・殺戮のジャンゴ -地獄の賞金首- (Nitroplus, 2008)'s own additions to
//! the NSS compiler ([`DJANGO`]): an 800×600 screen and the boot script
//! `boot.nss`, whose `#include`s are the helper library.

use super::Game;

/// The game's hooks; the CLI uses them for a game with `Django.exe`.
pub const DJANGO: Game = Game { screen: (800, 600), boot: "boot.nss", ..Game::PLAIN };
