// app.rs — application state: pages, focus, menus, input, output cursor/selection
use std::io::Read;
use std::os::unix::io::AsRawFd;

use ratatui::widgets::ListState;

use crate::clipboard::osc52_copy;
use crate::lab::{read_flags, run_lab_capture, spawn_lab, PIN};


#[derive(PartialEq, Clone, Copy)]
pub enum Focus { Menu, Output }
#[derive(PartialEq, Clone, Copy)]
pub enum Page { Main, Play, Guide, Flags, Connect, Settings }

// A lab.sh action running in the background; polled from the event loop.
pub struct RunningJob {
    child: std::process::Child,
    pub action: String,
    stdout: std::process::ChildStdout,
    stderr: std::process::ChildStderr,
    out_open: bool,
    err_open: bool,
    code: Option<i32>,
    canceled: bool,
}

// fcntl O_NONBLOCK via raw libc call
extern "C" { fn fcntl(fd: i32, cmd: i32, arg: i32) -> i32; }

fn set_nonblocking(fd: i32) {
    const F_GETFL: i32 = 3;
    const F_SETFL: i32 = 4;
    const O_NONBLOCK: i32 = 0o4000;
    unsafe {
        let flags = fcntl(fd, F_GETFL, 0);
        if flags >= 0 { fcntl(fd, F_SETFL, flags | O_NONBLOCK); }
    }
}

// strip ANSI escape sequences (docker/compose colors) before showing in the pane
fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            if chars.peek() == Some(&'[') {
                chars.next();
                while let Some(&k) = chars.peek() {
                    chars.next();
                    if k.is_ascii_alphabetic() { break; }
                }
            } else {
                chars.next(); // single-char escape
            }
        } else {
            out.push(c);
        }
    }
    out
}

pub struct App {
    pub page: Page,
    pub main_state: ListState,
    pub play_state: ListState,
    pub guide_state: ListState,
    pub flags_state: ListState,
    pub connect_state: ListState,
    pub settings_state: ListState,
    pub focus: Focus,
    pub output: String,
    pub output_scroll: (u16, u16),
    pub cursor: (u16, u16),
    // mouse text selection, absolute (col,row) in chars; line_len counts chars
    // so multi-byte UTF-8 in output no longer skews the cursor/selection
    pub sel: Option<((u16, u16), (u16, u16))>,
    pub input_mode: bool,
    pub input: String,
    pub input_prompt: String,
    pub input_action: String,
    pub status_line: String,
    pub pin_ok: bool,
    pub flags_submitted: [bool; 3],
    pub running: Option<RunningJob>,
    // set when a menu action needs the real terminal (SSH); ui loop runs it after leaving raw mode
    pub suspended_action: Option<SuspendedAction>,
    // guide state: loaded steps + current step + chosen method (None = step intro)
    pub guide: Option<crate::guide::Guide>,
    pub guide_step: usize,
    pub guide_choice: Option<usize>,
    // the chosen method id (from @choice) — persists across steps, drives @variant text
    pub guide_method: Option<String>,
    // arrow-selectable confirmation dialog (Enter = confirm, Esc = safe option)
    pub confirm: Option<ConfirmState>,
    // true while the output view is glued to the tail of a streaming job;
    // any upward scroll detaches, End/PageDown-to-bottom re-attaches
    pub follow_tail: bool,
}

// one confirm button; danger ones render in berry, safe ones in sky
#[derive(Clone)]
pub struct ConfirmButton {
    pub label: String,
    pub danger: bool,
}

#[derive(Clone)]
pub struct ConfirmState {
    pub action: String,   // what to run when confirmed ("kali-reset", "cancel-job")
    pub title: String,    // popup title, e.g. " Reset Kali? "
    pub lines: Vec<String>, // explanation shown in the popup
    pub buttons: Vec<ConfirmButton>,
    pub selected: usize,  // starts on the SAFE button
}

impl ConfirmState {
    // two-button helper: safe default first, danger second
    pub fn new(action: &str, title: &str, lines: &[&str], safe: &str, danger: &str) -> Self {
        ConfirmState {
            action: action.to_string(),
            title: title.to_string(),
            lines: lines.iter().map(|s| s.to_string()).collect(),
            buttons: vec![
                ConfirmButton { label: safe.to_string(), danger: false },
                ConfirmButton { label: danger.to_string(), danger: true },
            ],
            selected: 0,
        }
    }
    pub fn move_sel(&mut self, dir: i32) {
        let n = self.buttons.len() as i32;
        self.selected = ((self.selected as i32 + dir).rem_euclid(n)) as usize;
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum SuspendedAction {
    SshKali,
}

impl App {
    pub fn new() -> Self {
        let mut main_state = ListState::default();
        main_state.select(Some(0));
        let mut play_state = ListState::default();
        play_state.select(Some(0));
        let mut settings_state = ListState::default();
        settings_state.select(Some(0));
        let mut guide_state = ListState::default();
        guide_state.select(Some(0));
        let mut flags_state = ListState::default();
        flags_state.select(Some(0));
        let mut connect_state = ListState::default();
        connect_state.select(Some(0));
        Self {
            page: Page::Main,
            main_state,
            play_state,
            guide_state,
            flags_state,
            connect_state,
            settings_state,
            focus: Focus::Menu,
            output: "Welcome to Tunnriculum lab console.".to_string(),
            output_scroll: (0, 0),
            cursor: (0, 0),
            sel: None,
            input_mode: false,
            input: String::new(),
            input_prompt: String::new(),
            input_action: String::new(),
            status_line: String::new(),
            pin_ok: false,
            flags_submitted: [false, false, false],
            running: None,
            suspended_action: None,
            guide: None,
            guide_step: 0,
            guide_choice: None,
            guide_method: None,
            confirm: None,
            follow_tail: false,
        }
    }

    pub fn progress(&self) -> usize { self.flags_submitted.iter().filter(|&&b| b).count() }

    pub fn current_state_mut(&mut self) -> &mut ListState {
        match self.page {
            Page::Main => &mut self.main_state,
            Page::Play => &mut self.play_state,
            Page::Guide => &mut self.guide_state,
            Page::Flags => &mut self.flags_state,
            Page::Connect => &mut self.connect_state,
            Page::Settings => &mut self.settings_state,
        }
    }
    pub fn current_state(&self) -> &ListState {
        match self.page {
            Page::Main => &self.main_state,
            Page::Play => &self.play_state,
            Page::Guide => &self.guide_state,
            Page::Flags => &self.flags_state,
            Page::Connect => &self.connect_state,
            Page::Settings => &self.settings_state,
        }
    }
    pub fn current_items_len(&self) -> usize {
        match self.page {
            Page::Main => 3,
            Page::Play => 4,
            Page::Guide => self.guide_menu_len(),
            Page::Flags => 4,
            Page::Connect => 6,
            Page::Settings => { if !self.pin_ok { 1 } else { 9 } }
        }
    }
    // guide menu: [choice 0..n] + connect action (if any) + prev + next + back
    pub fn guide_menu_len(&self) -> usize {
        if let Some(g) = &self.guide {
            let step = &g.steps[self.guide_step.min(g.len() - 1)];
            let extra = step.choices.len()
                + if step.has_connect_action { 1 } else { 0 }
                + if self.guide_step > 0 { 1 } else { 0 }   // ← Previous step
                + if self.guide_step + 1 < g.len() { 1 } else { 0 }; // Next step
            extra + 1 // Back
        } else { 1 }
    }
    pub fn next_item(&mut self) {
        self.status_line.clear();
        let len = self.current_items_len();
        let state = self.current_state_mut();
        let i = match state.selected() { Some(i) => (i + 1) % len, None => 0 };
        state.select(Some(i));
    }
    pub fn prev_item(&mut self) {
        self.status_line.clear();
        let len = self.current_items_len();
        let state = self.current_state_mut();
        let i = match state.selected() { Some(i) => (i + len - 1) % len, None => 0 };
        state.select(Some(i));
    }
    pub fn selected_idx(&self) -> usize { self.current_state().selected().unwrap_or(0) }

    pub fn selected_desc(&self) -> String {
        match self.page {
            Page::Main => match self.selected_idx() {
                0 => "Submit flags from the three zones".to_string(),
                1 => "Lab administration \u{2014} PIN required".to_string(),
                _ => "Quit the console".to_string(),
            },
            Page::Play => match self.selected_idx() {
                0 => "Step-by-step walkthrough: the bug, the pivots, tunnels & ligolo-ng".to_string(),
                1 => "Submit the flags you captured".to_string(),
                2 => "Kali desktop, SSH, WireGuard \u{2014} ways into the lab".to_string(),
                _ => "Return to the main menu".to_string(),
            },
            Page::Flags => match self.selected_idx() {
                0 | 1 | 2 => "Submit the flag you captured".to_string(),
                _ => "Return to the Play menu".to_string(),
            },
            Page::Connect => match self.selected_idx() {
                0 => "Browser Kali desktop \u{2014} URL + password, no WireGuard needed".to_string(),
                1 => "SSH straight into the Kali box".to_string(),
                2 => "Get WireGuard peer config for your own machine".to_string(),
                3 => "Restart the Kali container \u{2014} keeps your home files".to_string(),
                4 => "Wipe the Kali desktop (fresh container) \u{2014} other services untouched".to_string(),
                _ => "Return to the Play menu".to_string(),
            },
            Page::Guide => {
                if self.guide.is_none() { "Guide failed to load".to_string() }
                else { "Pick a method, then Next step".to_string() }
            }
            Page::Settings => {
                if !self.pin_ok { "Locked \u{2014} press Enter and type the 4-digit PIN".to_string() }
                else { match self.selected_idx() {
                    0 => "Show running containers".to_string(),
                    1 => "Tail service logs".to_string(),
                    2 => "Verify network topology".to_string(),
                    3 => "Print WireGuard peer config".to_string(),
                    4 => "Create the lab".to_string(),
                    5 => "Stop the lab".to_string(),
                    6 => "Recreate containers".to_string(),
                    7 => "Wipe DB and regenerate flags".to_string(),
                    _ => "Return to the main menu".to_string(),
                }}
            }
        }
    }

    pub fn switch_to_page(&mut self, target: Page) {
        if target == Page::Settings && !self.pin_ok {
            self.page = Page::Settings;
            self.input_mode = true;
            self.input.clear();
            self.input_action = "pin".to_string();
            self.input_prompt = "PIN:".to_string();
            self.status_line = "PIN required \u{2014} 4 digits, Enter to confirm".to_string();
            return;
        }
        self.page = target;
        self.focus = Focus::Menu;
        match target {
            Page::Main => self.main_state.select(Some(0)),
            Page::Play => self.play_state.select(Some(0)),
            Page::Guide => { self.open_guide(); }
            Page::Flags => self.flags_state.select(Some(0)),
            Page::Connect => self.connect_state.select(Some(0)),
            Page::Settings => self.settings_state.select(Some(0)),
        }
    }

    // load (once) and show the current guide step in the output pane
    pub fn open_guide(&mut self) {
        if self.guide.is_none() {
            self.guide = match crate::guide::Guide::load() {
                Ok(g) => Some(g),
                Err(e) => {
                    self.output = e;
                    self.focus = Focus::Output;
                    return;
                }
            };
        }
        self.guide_state.select(Some(0));
        self.show_guide_step();
    }

    fn show_guide_step(&mut self) {
        if let Some(g) = &self.guide {
            let step = &g.steps[self.guide_step.min(g.len() - 1)];
            // base body + the variant of the currently chosen method (if this step has one)
            let mut out = step.body.clone();
            if let Some(m) = &self.guide_method {
                if let Some((_, vbody)) = step.variants.iter().find(|(v, _)| v == m) {
                    out.push_str(&format!("\n\u{2500}\u{2500} with {} \u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\n\n", m));
                    out.push_str(vbody);
                }
            }
            // keep the menu marker on the chosen choice of this step
            self.guide_choice = self.guide_method.as_ref()
                .and_then(|m| step.choices.iter().position(|c| &c.id == m));
            self.output = out;
            self.output_scroll = (0, 0);
            self.cursor = (0, 0);
            self.sel = None;
            self.status_line = format!(
                "Step {}/{} \u{2014} Tab output to scroll \u{00B7} \u{2190}\u{2192} step \u{00B7} y copy",
                self.guide_step + 1, g.len()
            );
        }
    }

    fn show_guide_choice(&mut self, which: usize) {
        if let Some(g) = &self.guide {
            let step = &g.steps[self.guide_step.min(g.len() - 1)];
            if let Some(ch) = step.choices.get(which) {
                // keep the step intro (e.g. the payload to prepare) above the
                // chosen method, same pattern as the @variant append below
                self.output = step.body.clone()
                    + &format!("\n\u{2500}\u{2500} {} \u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\n\n", ch.name)
                    + &ch.body;
                self.output_scroll = (0, 0);
                self.cursor = (0, 0);
                self.sel = None;
                self.guide_choice = Some(which);
                // remember the method — later steps render their @variant of it
                self.guide_method = Some(ch.id.clone());
            }
        }
    }

    pub fn guide_next_step(&mut self) {
        if let Some(g) = &self.guide {
            if self.guide_step + 1 < g.len() {
                self.guide_step += 1;
                self.guide_state.select(Some(0));
                self.show_guide_step();
            }
        }
    }
    pub fn guide_prev_step(&mut self) {
        if self.guide_step > 0 {
            self.guide_step -= 1;
            self.guide_state.select(Some(0));
            self.show_guide_step();
        }
    }

    // guide menu layout: [choices...] [connect action] [← Previous] [Next step] [Back]
    pub fn execute_guide(&mut self) {
        if self.guide.is_none() { self.go_play(); return; }
        let idx = self.selected_idx();
        let n_choices = self.guide.as_ref().map(|g| g.steps[self.guide_step].choices.len()).unwrap_or(0);
        let has_action = self.guide.as_ref().map(|g| g.steps[self.guide_step].has_connect_action).unwrap_or(false);
        let has_prev = self.guide_step > 0;
        let has_next = self.guide.as_ref().map(|g| self.guide_step + 1 < g.len()).unwrap_or(false);
        if idx < n_choices {
            self.show_guide_choice(idx);
            return;
        }
        let mut i = n_choices;
        if has_action {
            if idx == i { self.switch_to_page(Page::Connect); return; }
            i += 1;
        }
        if has_prev {
            if idx == i { self.guide_prev_step(); return; }
            i += 1;
        }
        if has_next {
            if idx == i { self.guide_next_step(); return; }
            i += 1;
        }
        if idx == i { self.go_play(); }
    }

    pub fn go_play(&mut self) {
        self.page = Page::Play;
        self.focus = Focus::Menu;
        self.play_state.select(Some(0));
        self.output.clear();
        self.output_scroll = (0, 0);
        self.cursor = (0, 0);
        self.sel = None;
        self.status_line.clear();
    }

    pub fn go_main(&mut self) {
        self.page = Page::Main;
        self.focus = Focus::Menu;
        self.main_state.select(Some(0));
        self.output.clear();
        self.output_scroll = (0, 0);
        self.cursor = (0, 0);
        self.sel = None;
        self.status_line.clear();
    }

    // returns true when the app should exit
    pub fn execute_main(&mut self) -> bool {
        match self.selected_idx() {
            0 => self.switch_to_page(Page::Play),
            1 => self.switch_to_page(Page::Settings),
            2 => return true,
            _ => {}
        }
        false
    }

    pub fn execute_play(&mut self) {
        // menu: How to play | Submit flags | Connect | Back
        match self.selected_idx() {
            0 => self.switch_to_page(Page::Guide),
            1 => self.switch_to_page(Page::Flags),
            2 => self.switch_to_page(Page::Connect),
            _ => self.go_main(),
        }
    }

    pub fn execute_flags(&mut self) {
        let idx = self.selected_idx();
        if idx == 3 { self.go_play(); return; }
        let action = match idx { 0 => "flag1", 1 => "flag2", 2 => "flag3", _ => return };
        self.input_mode = true;
        self.input.clear();
        self.input_action = action.to_string();
        self.input_prompt = format!("{}:", action);
        self.status_line = "Type the flag then Enter \u{2014} Esc to cancel".to_string();
    }

    pub fn execute_connect(&mut self) {
        match self.selected_idx() {
            0 => {
                // Kali desktop info
                let (code, out) = run_lab_capture(&["kali".to_string()]);
                self.output = out;
                self.output_scroll = (0, 0);
                self.cursor = (0, 0);
                self.sel = None;
                self.focus = Focus::Output;
                self.status_line = if code == 0 { String::new() } else { format!("kali failed (exit {})", code) };
            }
            1 => { self.suspended_action = Some(SuspendedAction::SshKali); }
            2 => {
                // WireGuard peer config
                self.input_mode = true;
                self.input.clear();
                self.input_action = "peer".to_string();
                self.input_prompt = "Public host/IP:".to_string();
                self.status_line = "Type host or IP then Enter (empty = 127.0.0.1)".to_string();
            }
            3 => {
                // restart kali container, keep home files
                self.output = "Restarting kali \u{2014} keeps your home files...".to_string();
                self.output_scroll = (0, 0);
                self.cursor = (0, 0);
                self.start_run("kali-restart".to_string());
            }
            4 => {
                // reset kali container: confirm dialog (default = keep, danger = reset)
                self.confirm = Some(ConfirmState::new(
                    "kali-reset",
                    " Reset Kali? ",
                    &[
                        "The Kali desktop is recreated from scratch.",
                        "Files in the kali home directory are lost.",
                        "Other services and flags are untouched.",
                    ],
                    "Keep",
                    "Reset",
                ));
            }
            _ => self.go_play(),
        }
    }

    pub fn execute_settings(&mut self) {
        if !self.pin_ok {
            self.input_mode = true;
            self.input.clear();
            self.input_action = "pin".to_string();
            self.input_prompt = "PIN:".to_string();
            self.status_line = "PIN required \u{2014} 4 digits, Enter to confirm".to_string();
            return;
        }
        let idx = self.selected_idx();
        // menu index -> lab.sh action; start/restart map to 'up' (lab.sh has no 'start')
        let action = match idx {
            0 => "status", 1 => "logs", 2 => "smoke", 3 => "peer",
            4 => "up", 5 => "stop", 6 => "up", 7 => "reset",
            _ => { self.go_main(); return; }
        };
        if action == "peer" {
            self.input_mode = true;
            self.input.clear();
            self.input_action = "peer".to_string();
            self.input_prompt = "Public host/IP:".to_string();
            self.status_line = "Type host or IP then Enter (empty = 127.0.0.1)".to_string();
            return;
        }
        self.start_run(action.to_string());
    }

    fn start_run(&mut self, action: String) {
        match spawn_lab(&[action.clone()]) {
            Ok(mut child) => {
                let stdout = child.stdout.take().expect("piped stdout");
                let stderr = child.stderr.take().expect("piped stderr");
                set_nonblocking(stdout.as_raw_fd());
                set_nonblocking(stderr.as_raw_fd());
                self.running = Some(RunningJob {
                    child, action: action.clone(),
                    stdout, stderr,
                    out_open: true, err_open: true,
                    code: None, canceled: false,
                });
                self.output.clear();
                // follow the streaming tail; the draw loop pins scroll to
                // max_scroll_y while follow_tail stays true
                self.follow_tail = true;
                self.output_scroll = (0, 0);
                self.cursor = (0, 0);
                self.sel = None;
                self.status_line = format!("Running {} \u{2014} Esc to cancel", action);
            }
            Err(e) => {
                self.output = format!("failed to start {}: {}", action, e);
                self.status_line = format!("{} failed to start", action);
            }
        }
    }

    // pump available bytes from the job's pipes into `output`; finalize on exit.
    // Returns true while a job is still running.
    pub fn poll_job(&mut self) -> bool {
        let job = match self.running.as_mut() {
            Some(j) => j,
            None => return false,
        };
        let mut buf = [0u8; 8192];
        let mut got = String::new();
        if job.out_open {
            loop {
                match job.stdout.read(&mut buf) {
                    Ok(0) => { job.out_open = false; break; }
                    Ok(n) => got.push_str(&String::from_utf8_lossy(&buf[..n])),
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                    Err(_) => { job.out_open = false; break; }
                }
            }
        }
        if job.err_open {
            loop {
                match job.stderr.read(&mut buf) {
                    Ok(0) => { job.err_open = false; break; }
                    Ok(n) => got.push_str(&String::from_utf8_lossy(&buf[..n])),
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                    Err(_) => { job.err_open = false; break; }
                }
            }
        }
        if !got.is_empty() {
            self.output.push_str(&strip_ansi(&got));
        }
        match job.child.try_wait() {
            Ok(Some(status)) => { job.code = status.code(); }
            Ok(None) => return true,
            Err(_) => return true,
        }
        // process exited — finalize
        let action = job.action.clone();
        let code = job.code.unwrap_or(1);
        let canceled = job.canceled;
        self.running = None;
        // fast jobs can finish before any draw pinned the tail: if we are
        // still following, jump to the tail now (u16::MAX is clamped to
        // max_scroll_y by the not-running clamp in the draw loop)
        if self.follow_tail { self.output_scroll.0 = u16::MAX; }
        if self.output.trim().is_empty() {
            self.output = if canceled { "(cancelled)".to_string() }
                else if code == 0 { "(done)".to_string() }
                else { format!("(exit {})", code) };
        }
        self.status_line = if canceled {
            format!("{} cancelled \u{2014} rerun to finish state changes", action)
        } else if code == 0 {
            format!("{} done \u{2014} Tab to read output", action)
        } else {
            format!("{} failed (exit {}) \u{2014} Tab to read output", action, code)
        };
        false
    }

    pub fn cancel_job(&mut self) {
        let job = match self.running.as_mut() {
            Some(j) => j,
            None => return,
        };
        job.canceled = true;
        let _ = job.child.kill();
        let _ = job.child.wait(); // reap
    }

    // read-only actions can be aborted freely; state-changing ones need a confirm
    fn can_cancel(&self) -> bool {
        match self.running.as_ref() {
            Some(j) => matches!(j.action.as_str(), "status" | "logs" | "smoke" | "peer"),
            None => false,
        }
    }

    // Esc during a state-changing job: ask before killing (lab may be left half-applied)
    pub fn request_cancel(&mut self) {
        if self.running.is_none() || self.input_mode { return; }
        if self.can_cancel() {
            self.cancel_job();
            self.poll_job();
            return;
        }
        let action = self.running.as_ref().map(|j| j.action.clone()).unwrap_or_default();
        self.confirm = Some(ConfirmState::new(
            "cancel-job",
            &format!(" Cancel {}? ", action),
            &[
                &format!("{} may be left half-applied.", action),
                "Run it again afterwards to reconcile.",
            ],
            "Keep waiting",
            "Cancel job",
        ));
    }

    // confirm dialog result (called when the user picks a button)
    pub fn resolve_confirm(&mut self, confirmed: bool) {
        let Some(c) = self.confirm.take() else { return };
        match c.action.as_str() {
            "cancel-job" => {
                if confirmed {
                    self.cancel_job();
                    self.poll_job(); // finalize now, sets the "cancelled — rerun" status
                } else {
                    self.status_line = format!(
                        "Running {} \u{2014} Esc to cancel",
                        self.running.as_ref().map(|j| j.action.clone()).unwrap_or_default()
                    );
                }
            }
            "kali-reset" => {
                if confirmed {
                    self.output = "Recreating kali with a fresh desktop...".to_string();
                    self.output_scroll = (0, 0);
                    self.cursor = (0, 0);
                    self.start_run("kali-reset".to_string());
                } else {
                    self.output = "\u{2718} Kali reset cancelled".to_string();
                    self.output_scroll = (0, 0);
                    self.cursor = (0, 0);
                    self.sel = None;
                    self.focus = Focus::Output;
                    self.status_line = String::new();
                }
            }
            _ => {}
        }
    }

    pub fn is_busy(&self) -> bool { self.running.is_some() }

    pub fn submit_input(&mut self) {
        let act = self.input_action.clone();
        let val = self.input.trim().to_string();
        self.input.clear();
        self.input_mode = false;
        match act.as_str() {
            "pin" => {
                if val == PIN {
                    self.pin_ok = true;
                    self.page = Page::Settings;
                    self.focus = Focus::Menu;
                    self.settings_state.select(Some(0));
                    self.output = "\u{2714} PIN correct \u{2014} Settings unlocked".to_string();
                    self.status_line = String::new();
                    self.output_scroll = (0, 0);
                    self.cursor = (0, 0);
                } else {
                    self.output = format!("\u{2718} PIN incorrect");
                    self.status_line = "PIN incorrect \u{2014} try again (Esc to cancel)".to_string();
                    self.output_scroll = (0, 0);
                    self.focus = Focus::Output;
                }
            }
            "flag1" | "flag2" | "flag3" => {
                let (f1, f2, f3) = read_flags();
                let expected = match act.as_str() { "flag1" => f1, "flag2" => f2, "flag3" => f3, _ => String::new() };
                let idx = match act.as_str() { "flag1" => 0, "flag2" => 1, "flag3" => 2, _ => 0 };
                if !expected.is_empty() && val == expected {
                    self.flags_submitted[idx] = true;
                    let progress = self.progress();
                    self.output = format!("\u{2714} {} correct!  Progress {}/3", act, progress);
                    if progress == 3 {
                        self.output.push_str("\n\nAll flags submitted! \u{1F389}  You pwned the franchise.");
                    }
                } else {
                    self.output = format!("\u{2718} {} incorrect", act);
                    if expected.is_empty() { self.output.push_str(" (lab not initialized?)"); }
                }
                self.output_scroll = (0, 0);
                self.cursor = (0, 0);
                self.sel = None;
                self.focus = Focus::Output;
                self.status_line = String::new();
            }
            "peer" => {
                let ep = if val.is_empty() { "127.0.0.1".to_string() } else { val };
                let valid = !ep.is_empty() && ep.chars().all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-');
                if !valid {
                    self.output = "public host must be a hostname or IPv4 address".to_string();
                    self.focus = Focus::Output;
                    return;
                }
                let (code, out) = run_lab_capture(&["peer".to_string(), ep.clone()]);
                self.output = out;
                self.output_scroll = (0, 0);
                self.cursor = (0, 0);
                self.sel = None;
                self.focus = Focus::Output;
                self.status_line = if code == 0 { format!("peer {} done", ep) } else { format!("peer failed (exit {})", code) };
            }
            _ => {}
        }
        self.input_action.clear();
    }

    pub fn cancel_input(&mut self) {
        self.input_mode = false;
        self.input.clear();
        if self.page == Page::Settings && !self.pin_ok { self.go_main(); }
    }

    pub fn copy_all(&mut self) {
        osc52_copy(&self.output, "/tmp/labctl-output.txt");
        self.status_line = format!("copied {} chars to clipboard", self.output.chars().count());
    }

    pub fn copy_selection(&mut self) {
        if let Some(text) = self.selected_text() {
            osc52_copy(&text, "/tmp/labctl-selection.txt");
            self.status_line = format!("copied {} chars to clipboard", text.chars().count());
        }
    }

    // selection helpers
    pub fn sel_norm(&self) -> Option<((u16, u16), (u16, u16))> {
        let ((ax, ay), (bx, by)) = self.sel?;
        if (ay, ax) <= (by, bx) { Some(((ax, ay), (bx, by))) } else { Some(((bx, by), (ax, ay))) }
    }
    fn selected_text(&self) -> Option<String> {
        let ((x1, y1), (x2, y2)) = self.sel_norm()?;
        let lines: Vec<&str> = self.output.lines().collect();
        let mut out: Vec<String> = Vec::new();
        for y in y1..=y2 {
            if y as usize >= lines.len() { break; }
            let chars: Vec<char> = lines[y as usize].chars().collect();
            let s = if y == y1 { (x1 as usize).min(chars.len()) } else { 0 };
            let e = if y == y2 { (x2 as usize).min(chars.len()) } else { chars.len() };
            out.push(chars.get(s..e.max(s)).map(|c| c.iter().collect()).unwrap_or_default());
        }
        let text = out.join("\n");
        if text.is_empty() { None } else { Some(text) }
    }

    fn line_len(&self, abs_y: usize) -> u16 { self.output.lines().nth(abs_y).map(|l| l.chars().count() as u16).unwrap_or(0) }
    fn clamp_cursor_to_text(&mut self) {
        let abs_y = (self.output_scroll.0 + self.cursor.1) as usize;
        let len = self.line_len(abs_y);
        if len == 0 { self.cursor.0 = 0; } else if self.cursor.0 > len { self.cursor.0 = len.min(80); }
    }
    // upward scroll detaches from the tail; downward scroll to the bottom re-attaches
    pub fn scroll_up(&mut self, n: u16) {
        self.follow_tail = false;
        self.output_scroll.0 = self.output_scroll.0.saturating_sub(n);
        self.clamp_cursor_to_text();
    }
    pub fn scroll_down(&mut self, n: u16, max_y: u16) {
        self.output_scroll.0 = (self.output_scroll.0 + n).min(max_y);
        self.follow_tail = self.output_scroll.0 >= max_y;
        self.clamp_cursor_to_text();
    }
    pub fn cursor_up(&mut self) {
        if self.cursor.1 > 0 { self.cursor.1 -= 1; }
        else { self.follow_tail = false; self.output_scroll.0 = self.output_scroll.0.saturating_sub(1); }
        self.clamp_cursor_to_text();
    }
    pub fn cursor_down(&mut self, pane_h: u16) {
        let abs_y_next = (self.output_scroll.0 + self.cursor.1 + 1) as usize;
        if abs_y_next >= self.output.lines().count() {
            self.follow_tail = true; // reached the last line: glue to the tail
            return;
        }
        if self.cursor.1 + 1 < pane_h {
            self.cursor.1 += 1;
        } else {
            // cursor pinned on the last visible row: scroll one more line
            // (unbounded — the early-return above already guards the end of text)
            self.output_scroll.0 += 1;
        }
        self.clamp_cursor_to_text();
    }
    pub fn cursor_left(&mut self) {
        if self.cursor.0 > 0 { self.cursor.0 -= 1; } else if self.output_scroll.1 > 0 { self.output_scroll.1 -= 1; }
        self.clamp_cursor_to_text();
    }
    pub fn cursor_right(&mut self, max_x: u16) {
        let abs_y = (self.output_scroll.0 + self.cursor.1) as usize;
        let len = self.line_len(abs_y);
        let total_x = self.output_scroll.1 + self.cursor.0;
        if total_x >= len && len > 0 { return; }
        if self.cursor.0 + 1 < 60 { self.cursor.0 += 1; } else { self.output_scroll.1 = (self.output_scroll.1 + 1).min(max_x); }
        self.clamp_cursor_to_text();
    }
    pub fn warp_to(&mut self, x: u16, y: u16, vw: u16, vh: u16) {
        let abs_y = (self.output_scroll.0 + y) as usize;
        if abs_y >= self.output.lines().count() { return; }
        let len = self.line_len(abs_y);
        let target_x = (self.output_scroll.1 + x).min(len);
        if target_x < self.output_scroll.1 { self.output_scroll.1 = target_x; self.cursor.0 = 0; }
        else if target_x >= self.output_scroll.1 + vw { self.output_scroll.1 = target_x.saturating_sub(vw - 1); self.cursor.0 = vw - 1; }
        else { self.cursor.0 = target_x - self.output_scroll.1; }
        if y < vh { self.cursor.1 = y; }
        self.clamp_cursor_to_text();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app_with_lines(n: usize) -> App {
        let mut a = App::new();
        a.output = (0..n).map(|i| format!("line {}", i)).collect::<Vec<_>>().join("\n");
        a
    }

    // ---- scroll + follow_tail (regressions from the streaming-scroll fixes) ----

    #[test]
    fn cursor_down_reaches_the_last_line() {
        // 44 lines, pane 10 rows: walking down must land the cursor on
        // absolute line 43 (the last), not stop one line short
        let mut a = app_with_lines(44);
        for _ in 0..60 { a.cursor_down(10); }
        assert_eq!(a.output_scroll.0 + a.cursor.1, 43);
        // one more down is a no-op at the end of text
        a.cursor_down(10);
        assert_eq!(a.output_scroll.0 + a.cursor.1, 43);
    }

    #[test]
    fn reaching_bottom_sets_follow_tail() {
        let mut a = app_with_lines(44);
        a.follow_tail = false;
        for _ in 0..60 { a.cursor_down(10); }
        assert!(a.follow_tail, "reaching the last line must glue to the tail");
    }

    #[test]
    fn scroll_up_detaches_follow() {
        let mut a = app_with_lines(100);
        a.follow_tail = true;
        a.scroll_up(10);
        assert!(!a.follow_tail);
    }

    #[test]
    fn scroll_down_to_bottom_reattaches_follow() {
        let mut a = app_with_lines(100);
        a.follow_tail = false;
        a.output_scroll.0 = 80; // max = 100 - 20 + ... anything below max
        a.scroll_down(200, 90); // clamp to max=90
        assert_eq!(a.output_scroll.0, 90);
        assert!(a.follow_tail, "landing on the bottom must re-attach");
    }

    #[test]
    fn scroll_down_partial_keeps_detached() {
        let mut a = app_with_lines(100);
        a.follow_tail = false;
        a.output_scroll.0 = 10;
        a.scroll_down(5, 90);
        assert!(!a.follow_tail);
        assert_eq!(a.output_scroll.0, 15);
    }

    // ---- confirm dialog ----

    #[test]
    fn confirm_buttons_default_safe_and_wrap() {
        let mut c = ConfirmState::new("kali-reset", " Reset Kali? ", &["x"], "Keep", "Reset");
        assert_eq!(c.selected, 0, "default must be the safe button");
        c.move_sel(1);
        assert_eq!(c.selected, 1);
        c.move_sel(1); // wraps back to 0
        assert_eq!(c.selected, 0);
        c.move_sel(-1); // wraps to the danger button
        assert_eq!(c.selected, 1);
        assert!(!c.buttons[0].danger);
        assert!(c.buttons[1].danger);
    }

    #[test]
    fn resolve_confirm_cancel_does_not_run() {
        let mut a = App::new();
        a.confirm = Some(ConfirmState::new("kali-reset", " t ", &["x"], "Keep", "Reset"));
        a.resolve_confirm(false);
        assert!(a.confirm.is_none());
        assert!(a.running.is_none(), "cancel must not spawn a job");
        assert!(a.output.contains("cancelled"));
    }

    #[test]
    fn resolve_confirm_accept_starts_the_action() {
        let mut a = App::new();
        a.confirm = Some(ConfirmState::new("kali-reset", " t ", &["x"], "Keep", "Reset"));
        a.resolve_confirm(true);
        assert!(a.confirm.is_none(), "dialog must be consumed");
        // start_run was attempted: either a job is running, or the spawn error
        // names the action (lab.sh absent in sandboxed test envs)
        assert!(a.running.is_some() || a.output.contains("kali-reset"));
    }
}
