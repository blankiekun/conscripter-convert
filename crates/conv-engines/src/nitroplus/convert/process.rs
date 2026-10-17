//! `CreateProcess`: a function that runs beside the script once
//! `Request(…, Start)` starts it. Converted when it only plays music on a
//! timer (続・殺戮のジャンゴ: a loop a set time after its intro, a song
//! five seconds after a line, a jingle in two parts): its sounds become
//! [`Op::BgmLater`] at their time from the start; or when it only runs
//! timed actions (装甲悪鬼村正's `CP_RollBar`: the roll bar and the sky
//! turning), run at its start as actions that don't block, which a
//! `WaitAction` on the process waits for. Others are noted.

use super::*;

/// What a music process may call.
const MUSIC_CALLS: &[&str] = &["SoundPlay", "MusicStart", "Wait", "WaitPlay", "CreateBGM"];

/// What an action process may call.
const ACTION_CALLS: &[&str] = &["Rotate", "Move", "Fade", "Zoom"];

/// Whether `body` only calls these (and has labels).
fn calls_only(body: &[Stmt], calls: &[&str]) -> bool {
    body.iter().all(|s| match &s.kind {
        StmtKind::Label(_) => true,
        StmtKind::Expr(Expr::Call { name, .. }) => calls.contains(&name.as_str()),
        _ => false,
    })
}

impl Compiler<'_, '_> {
    pub(super) fn create_process(&mut self, name: String, function: &str) {
        match self.function(function) {
            Some((file, f)) if calls_only(&f.body, MUSIC_CALLS) || calls_only(&f.body, ACTION_CALLS) || super::loops::is_loop(f) || super::one_shot::is_one_shot(f) => {
                self.create(name, Kind::Process { file, function: f }, 0, 0, 0);
            }
            _ => self.note(format!("CreateProcess {function}")),
        }
    }

    /// `CreateProcess` of a process the game runs in line when it starts,
    /// its waits holding the script (装甲悪鬼村正's ending roll, which the
    /// script waits for anyway).
    pub(super) fn inline_process(&mut self, args: &[Value]) -> bool {
        let Some((file, function)) = args.get(4).and_then(|f| self.function(&f.string())) else { return false };
        let name = self.object_name(&args[0]);
        self.inline_processes.push(name.clone());
        self.create(name, Kind::Process { file, function }, 0, 0, 0);
        true
    }

    /// An action process runs, whose waits hold only itself (not one run
    /// in line): they add to [`Compiler::process_lead`].
    pub(super) fn waits_alone(&self) -> bool {
        self.action_process.as_ref().is_some_and(|p| !self.inline_processes.contains(p))
    }

    /// Runs a music process's function at once, its waits adding to
    /// [`Compiler::process_clock`] instead of waiting.
    pub(super) fn start_process(&mut self, name: &str) {
        let Some(Object { kind: Kind::Process { file, function }, .. }) = self.objects.get(name) else { return };
        let (file, function) = (*file, *function);
        if super::loops::is_loop(function) {
            self.start_loop(name, file, function);
            return;
        }
        // (One run in line holds the script with its waits: as an action process.)
        if !calls_only(&function.body, ACTION_CALLS) && !calls_only(&function.body, MUSIC_CALLS) && !self.inline_processes.iter().any(|p| p == name) {
            self.start_one_shot(name, file, function);
            return;
        }
        if !calls_only(&function.body, MUSIC_CALLS) {
            self.action_process = Some(name.to_string());
            self.call_function(file, function, Vec::new());
            self.action_process = None;
            return;
        }
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
