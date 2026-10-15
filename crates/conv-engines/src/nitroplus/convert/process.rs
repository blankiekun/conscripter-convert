//! `CreateProcess`: a function that runs beside the script once
//! `Request(…, Start)` starts it. Converted when it only plays music on a
//! timer (続・殺戮のジャンゴ: a loop a set time after its intro, a song
//! five seconds after a line, a jingle in two parts): its sounds become
//! [`Op::BgmLater`] at their time from the start. Others are noted.

use super::*;

/// What a music process may call.
const MUSIC_CALLS: &[&str] = &["SoundPlay", "MusicStart", "Wait", "WaitPlay", "CreateBGM"];

/// Whether `body` only plays music on a timer.
fn plays_music_only(body: &[Stmt]) -> bool {
    body.iter().all(|s| match &s.kind {
        StmtKind::Label(_) => true,
        StmtKind::Expr(Expr::Call { name, .. }) => MUSIC_CALLS.contains(&name.as_str()),
        _ => false,
    })
}

impl Compiler<'_, '_> {
    pub(super) fn create_process(&mut self, name: String, function: &str) {
        match self.function(function) {
            Some((file, f)) if plays_music_only(&f.body) => {
                self.create(name, Kind::Process { file, function: f }, 0, 0, 0);
            }
            _ => self.note(format!("CreateProcess {function}")),
        }
    }

    /// Runs a music process's function at once, its waits adding to
    /// [`Compiler::process_clock`] instead of waiting.
    pub(super) fn start_process(&mut self, name: &str) {
        let Some(Object { kind: Kind::Process { file, function }, .. }) = self.objects.get(name) else { return };
        let (file, function) = (*file, *function);
        self.flush_sounds();
        self.process_clock = Some(0);
        self.call_function(file, function, Vec::new());
        self.flush_sounds();
        self.process_clock = None;
    }

    /// A wait in a music process: the sounds started so far start at the
    /// clock's time, then it moves on.
    pub(super) fn process_wait(&mut self, ms: u32) {
        self.flush_sounds();
        if let Some(clock) = &mut self.process_clock {
            *clock += ms;
        }
    }
}
