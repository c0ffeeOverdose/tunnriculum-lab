// ui.rs — theme, splash wordmark, render + event loop
use std::io;

use crossterm::{
    cursor,
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, KeyModifiers, MouseButton, MouseEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
    Terminal,
};

use crate::app::{App, Focus, Page, SuspendedAction};
use crate::lab;

pub const C_SKY: Color = Color::Rgb(205, 235, 250);
pub const C_SKY_DEEP: Color = Color::Rgb(143, 203, 224);
pub const C_PINK: Color = Color::Rgb(255, 199, 211);
pub const C_PINK_DEEP: Color = Color::Rgb(255, 158, 187);
pub const C_BERRY: Color = Color::Rgb(255, 107, 138);
pub const C_MUTED: Color = Color::Rgb(160, 140, 146);

fn centered(w: u16, h: u16, r: Rect) -> Rect {
    Rect {
        x: r.x + r.width.saturating_sub(w) / 2,
        y: r.y + r.height.saturating_sub(h) / 2,
        width: w.min(r.width),
        height: h.min(r.height),
    }
}

// ANSI Shadow wordmark 'Tunnriculum', 6 rows x 94 cols
const WORDMARK: [&str; 6] = [
    "████████╗██╗   ██╗███╗   ██╗███╗   ██╗██████╗ ██╗ ██████╗██╗   ██╗██╗     ██╗   ██╗███╗   ███╗",
    "╚══██╔══╝██║   ██║████╗  ██║████╗  ██║██╔══██╗██║██╔════╝██║   ██║██║     ██║   ██║████╗ ████║",
    "   ██║   ██║   ██║██╔██╗ ██║██╔██╗ ██║██████╔╝██║██║     ██║   ██║██║     ██║   ██║██╔████╔██║",
    "   ██║   ██║   ██║██║╚██╗██║██║╚██╗██║██╔══██╗██║██║     ██║   ██║██║     ██║   ██║██║╚██╔╝██║",
    "   ██║   ╚██████╔╝██║ ╚████║██║ ╚████║██║  ██║██║╚██████╗╚██████╔╝███████╗╚██████╔╝██║ ╚═╝ ██║",
    "   ╚═╝    ╚═════╝ ╚═╝  ╚═══╝╚═╝  ╚═══╝╚═╝  ╚═╝╚═╝ ╚═════╝ ╚═════╝ ╚══════╝ ╚═════╝ ╚═╝     ╚═╝",
];

// splash wordmark with per-row pastel gradient; bold text fallback on narrow terminals
fn splash_lines(width: u16) -> Vec<Line<'static>> {
    let row_colors = [C_SKY, C_SKY_DEEP, C_PINK, C_PINK_DEEP, C_BERRY, C_BERRY];
    if 94 <= width {
        return WORDMARK.iter().enumerate().map(|(i, l)| {
            Line::from(Span::styled(l.to_string(), Style::default().fg(row_colors[i % row_colors.len()]))).alignment(Alignment::Center)
        }).collect();
    }
    vec![Line::from(Span::styled("Tunnriculum", Style::default().fg(C_PINK_DEEP).add_modifier(Modifier::BOLD))).alignment(Alignment::Center)]
}

pub fn tui() -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let mut app = App::new();
    let splash = splash_lines(terminal.size()?.width);
    let splash_h = splash.len() as u16;
    loop {
        // a menu action asked for the real terminal (SSH): leave the TUI, run it, come back
        if let Some(act) = app.suspended_action.take() {
            disable_raw_mode()?;
            // ratatui hides the cursor while drawing; restore it so bash shows a blinking prompt
            execute!(io::stdout(), cursor::Show, LeaveAlternateScreen, DisableMouseCapture)?;
            let back_note = match act {
                SuspendedAction::SshKali => { let code = lab::ssh_kali(); code == 0 }
            };
            let _ = back_note;
            enable_raw_mode()?;
            execute!(io::stdout(), EnterAlternateScreen, EnableMouseCapture)?;
            terminal.clear()?;
            app.status_line = "\u{21B5} back from Kali".to_string();
            continue;
        }
        // layout + derived values are computed INSIDE the draw closure from
        // f.area() so widgets can never render outside the real buffer
        // (terminal.size() vs draw-time resize race panicked before).
        // The event loop below reuses the exported values from the last draw.
        let mut menu_rect = Rect::ZERO;
        let mut output_rect = Rect::ZERO;
        let mut output_width = 0u16;
        let mut output_height = 0u16;
        let mut max_scroll_y = 0u16;
        let mut max_scroll_x = 0u16;
        terminal.draw(|f| {
        let area = f.area();
        let chunks = Layout::default().direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Min(0), Constraint::Length(2)]).split(area);
        // layout computed once, shared by draw + events
        let (m_rect, o_rect, splash_rect) = if app.page == Page::Main {
            // full-screen splash: art + flags + menu stacked, centered as a whole
            let inner = chunks[1];
            let total = splash_h + 5; // art + blank + flags + blank + menu(5 items in box)
            let top = inner.y + inner.height.saturating_sub(total) / 2;
            let s = Rect { x: inner.x, y: top, width: inner.width, height: splash_h + 2 };
            let m = centered(26, 5, Rect { x: inner.x, y: top + splash_h + 2, width: inner.width, height: inner.height.saturating_sub(top + splash_h + 2 - inner.y) });
            (m, Rect::ZERO, s)
        } else {
            let body = Layout::default().direction(Direction::Horizontal)
                .constraints([Constraint::Length(30), Constraint::Min(0)]).split(chunks[1]);
            (body[0], body[1], Rect::ZERO)
        };
        menu_rect = m_rect;
        output_rect = o_rect;
        output_width = o_rect.width.saturating_sub(2);
        output_height = o_rect.height.saturating_sub(2);
        let lines = app.output.lines().count() as u16;
        // ratatui renders inner rows scroll..scroll+output_height-1, so the
        // last line (index lines-1) is visible at scroll = lines - output_height.
        // (a +1 here hid the first line of short outputs via End/Home pinning)
        max_scroll_y = lines.saturating_sub(output_height);
        let max_line_len = app.output.lines().map(|l| l.chars().count() as u16).max().unwrap_or(0);
        max_scroll_x = max_line_len.saturating_sub(output_width);

        // while a job streams, follow the tail while follow_tail is true
        // (start_run sets it; any upward scroll detaches, reaching the bottom
        // re-attaches). Pinning only while running: a stale follow flag must
        // not yank freshly-opened static pages (guide, kali info) to the bottom.
        // Jobs that finish before any draw get pinned by poll_job's finalize.
        if app.running.is_some() && app.follow_tail {
            app.output_scroll.0 = max_scroll_y;
        }
        // keep scroll sane at all times (also consumes the finalize u16::MAX)
        if app.output_scroll.0 > max_scroll_y {
            app.output_scroll.0 = max_scroll_y;
        }
            if app.page == Page::Main {
                // full-screen splash: no header, just art + flags + menu
                let mut art = splash.clone();
                let dots: String = app.flags_submitted.iter()
                    .map(|&b| if b { "\u{25CF} " } else { "\u{25CB} " }).collect::<Vec<_>>().join(" ");
                art.push(Line::from(""));
                art.push(Line::from(Span::styled(format!("Flags  {}  {}/3", dots, app.progress()), Style::default().fg(C_BERRY))).alignment(Alignment::Center));
                let splash_p = Paragraph::new(art);
                f.render_widget(splash_p, splash_rect);

                let items = vec![
                    ListItem::new("Play"),
                    ListItem::new("Settings"),
                    ListItem::new("Exit"),
                ];
                let list = List::new(items)
                    .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(C_SKY)))
                    .highlight_style(Style::default().bg(C_PINK).fg(Color::Black).add_modifier(Modifier::BOLD))
                    .highlight_symbol("\u{25B6} ");
                f.render_stateful_widget(list, menu_rect, &mut app.main_state);
            } else {
                // thin header: breadcrumb only
                let crumb = match app.page {
                    Page::Play => "Play".to_string(),
                    Page::Guide => format!("Guide \u{00B7} step {}/{}", app.guide_step + 1,
                        app.guide.as_ref().map(|g| g.len()).unwrap_or(0)),
                    Page::Flags => "Play \u{203A} Submit flags".to_string(),
                    Page::Connect => "Play \u{203A} Connect".to_string(),
                    Page::Settings => if app.pin_ok { "Settings".to_string() } else { "Settings \u{1F512}".to_string() },
                    Page::Main => unreachable!(),
                };
                let prog = app.progress();
                let progress_str = if prog > 0 { format!("  \u{00B7}  {}/3", prog) } else { String::new() };
                let header = Paragraph::new(Line::from(vec![
                    Span::styled("Tunnriculum", Style::default().fg(C_PINK_DEEP).add_modifier(Modifier::BOLD)),
                    Span::styled(format!(" \u{203A} {}", crumb), Style::default().fg(C_SKY_DEEP).add_modifier(Modifier::BOLD)),
                    Span::styled(progress_str, Style::default().fg(C_BERRY)),
                ]));
                f.render_widget(header, chunks[0]);
                // menu
                let menu_border = if app.focus == Focus::Menu && !app.input_mode { Style::default().fg(C_PINK_DEEP) } else { Style::default().fg(C_SKY) };
                let output_border = if app.focus == Focus::Output && !app.input_mode { Style::default().fg(C_PINK_DEEP) } else { Style::default().fg(C_SKY) };
                let (menu_title, menu_items) = match app.page {
                    Page::Play => {
                        let title = format!(" Play \u{2014} {}/3 ", app.progress());
                        let items = vec![
                            ListItem::new(Span::styled("How to play \u{1F4D6} ", Style::default().fg(C_SKY_DEEP))),
                            ListItem::new(Span::styled("Submit flags \u{1F6A9} ", Style::default().fg(C_SKY_DEEP))),
                            ListItem::new(Span::styled("Connect \u{1F50C} ", Style::default().fg(C_SKY_DEEP))),
                            ListItem::new(Span::styled("\u{2190} Back", Style::default().fg(C_MUTED))),
                        ];
                        (title, items)
                    }
                    Page::Guide => {
                        let mut items = Vec::new();
                        if let Some(g) = &app.guide {
                            let step = &g.steps[app.guide_step.min(g.len() - 1)];
                            for (i, ch) in step.choices.iter().enumerate() {
                                let mark = if app.guide_choice == Some(i) { "\u{25CF} " } else { "\u{25CB} " };
                                items.push(ListItem::new(Line::from(Span::styled(format!("{}{}", mark, ch.name), Style::default().fg(C_SKY_DEEP)))));
                            }
                            if step.has_connect_action {
                                items.push(ListItem::new(Span::styled("Open Connect menu \u{1F50C}", Style::default().fg(C_SKY_DEEP))));
                            }
                            if app.guide_step > 0 {
                                items.push(ListItem::new(Span::styled("\u{2190} Previous step", Style::default().fg(C_SKY_DEEP))));
                            }
                            if app.guide_step + 1 < g.len() {
                                items.push(ListItem::new(Span::styled("Next step \u{2192}", Style::default().fg(C_BERRY).add_modifier(Modifier::BOLD))));
                            }
                        }
                        items.push(ListItem::new(Span::styled("\u{2190} Back", Style::default().fg(C_MUTED))));
                        (format!(" Step {}/{} ", app.guide_step + 1, app.guide.as_ref().map(|g| g.len()).unwrap_or(0)), items)
                    }
                    Page::Flags => {
                        let title = format!(" Submit flags \u{2014} {}/3 ", app.progress());
                        let mut items = Vec::new();
                        for i in 0..3 {
                            let done = app.flags_submitted[i];
                            let icon = if done { "\u{25CF} " } else { "\u{25CB} " };
                            let style = if done { Style::default().fg(C_BERRY).add_modifier(Modifier::BOLD) } else { Style::default() };
                            items.push(ListItem::new(Line::from(Span::styled(format!("{}Flag{}", icon, i + 1), style))));
                        }
                        items.push(ListItem::new(Span::styled("\u{2190} Back", Style::default().fg(C_MUTED))));
                        (title, items)
                    }
                    Page::Connect => {
                        (" Connect ".to_string(), vec![
                            ListItem::new(Span::styled("Kali desktop \u{1F5A5} ", Style::default().fg(C_SKY_DEEP))),
                            ListItem::new(Span::styled("SSH into Kali \u{1F5DD} ", Style::default().fg(C_SKY_DEEP))),
                            ListItem::new(Span::styled("WireGuard config \u{1F511} ", Style::default().fg(C_SKY_DEEP))),
                            ListItem::new(Span::styled("Restart Kali \u{1F504} ", Style::default().fg(C_SKY_DEEP))),
                            ListItem::new(Span::styled("Reset Kali (fresh desktop) \u{26A0} ", Style::default().fg(C_BERRY))),
                            ListItem::new(Span::styled("\u{2190} Back", Style::default().fg(C_MUTED))),
                        ])
                    }
                    Page::Settings => {
                        if !app.pin_ok {
                            (" Settings ".to_string(), vec![ListItem::new(Span::styled("\u{1F512} Locked \u{2014} press Enter for PIN", Style::default().fg(C_BERRY)))])
                        } else {
                            (" Settings ".to_string(), vec![
                                ListItem::new("Status"), ListItem::new("Logs"), ListItem::new("Smoke test"),
                                ListItem::new("Peer config"), ListItem::new("Start"), ListItem::new("Stop"),
                                ListItem::new("Restart"), ListItem::new("Reset"),
                                ListItem::new(Span::styled("\u{2190} Back", Style::default().fg(C_MUTED))),
                            ])
                        }
                    }
                    Page::Main => unreachable!(),
                };
                let list = List::new(menu_items)
                    .block(Block::default().borders(Borders::ALL).border_style(menu_border).title(Span::styled(menu_title, Style::default().fg(C_SKY_DEEP))))
                    .highlight_style(Style::default().bg(C_PINK).fg(Color::Black).add_modifier(Modifier::BOLD))
                    .highlight_symbol("\u{25B6} ");
                match app.page {
                    Page::Play => f.render_stateful_widget(list, menu_rect, &mut app.play_state),
                    Page::Guide => f.render_stateful_widget(list, menu_rect, &mut app.guide_state),
                    Page::Flags => f.render_stateful_widget(list, menu_rect, &mut app.flags_state),
                    Page::Connect => f.render_stateful_widget(list, menu_rect, &mut app.connect_state),
                    Page::Settings => f.render_stateful_widget(list, menu_rect, &mut app.settings_state),
                    Page::Main => unreachable!(),
                }

                // output with selection highlight
                let inline_input = app.input_mode;
                let title = if inline_input { format!(" {} ", app.input_prompt) } else { " Output ".to_string() };
                let content: Vec<Line> = if inline_input {
                    let shown = if app.input_action == "pin" { "*".repeat(app.input.len()) } else { app.input.clone() };
                    vec![
                        Line::from(format!("{}_", shown)),
                        Line::from(Span::styled("Enter to confirm \u{2014} Esc to cancel", Style::default().fg(C_MUTED))),
                    ]
                } else {
                    let n = app.sel_norm();
                    app.output.lines().enumerate().map(|(i, l)| {
                        if let Some(((x1, y1), (x2, y2))) = n {
                            let y = i as u16;
                            if y >= y1 && y <= y2 {
                                let chars: Vec<char> = l.chars().collect();
                                let s = if y == y1 { (x1 as usize).min(chars.len()) } else { 0 };
                                let e = if y == y2 { (x2 as usize).min(chars.len()) } else { chars.len() };
                                return Line::from(vec![
                                    Span::raw(chars[..s].iter().collect::<String>()),
                                    Span::styled(chars[s..e.max(s)].iter().collect::<String>(), Style::default().bg(C_PINK).fg(Color::Black)),
                                    Span::raw(chars[e.max(s)..].iter().collect::<String>()),
                                ]);
                            }
                        }
                        Line::raw(l.to_string())
                    }).collect()
                };
                let scroll_info = if !app.input_mode && lines > output_height {
                    format!("{}% ", ((app.output_scroll.0 as f32 / max_scroll_y.max(1) as f32) * 100.0) as u16)
                } else { String::new() };
                let output_title = format!("{}{}", title, scroll_info);
                // no Wrap: columns must map 1:1 for mouse selection
                let output = Paragraph::new(content)
                    .block(Block::default().borders(Borders::ALL).border_style(output_border).title(Span::styled(output_title, Style::default().fg(C_SKY_DEEP))))
                    .scroll((app.output_scroll.0, app.output_scroll.1));
                f.render_widget(output, output_rect);
                if app.focus == Focus::Output && !app.input_mode && output_width > 0 {
                    let cx = output_rect.x + 1 + app.cursor.0.min(output_width.saturating_sub(1));
                    let cy = output_rect.y + 1 + app.cursor.1.min(output_height.saturating_sub(1));
                    f.set_cursor_position((cx, cy));
                }
            }

            // confirm dialog: modal popup centered over the page
            if let Some(c) = &app.confirm {
                let popup_area = centered(48.min(area.width), 5 + c.lines.len() as u16 + 2, area);
                f.render_widget(Clear, popup_area);
                let mut popup_text: Vec<Line> = vec![Line::from("")];
                for l in &c.lines {
                    popup_text.push(Line::from(Span::styled(l.clone(), Style::default().fg(C_BERRY))).alignment(Alignment::Center));
                }
                popup_text.push(Line::from(""));
                // buttons row: selected inverts (pink bg), danger unselected in berry
                let mut row = Vec::new();
                for (i, b) in c.buttons.iter().enumerate() {
                    if i > 0 { row.push(Span::raw("    ")); }
                    let (bg, fg) = if i == c.selected {
                        (C_PINK, Color::Black)
                    } else if b.danger {
                        (Color::Reset, C_BERRY)
                    } else {
                        (Color::Reset, C_SKY_DEEP)
                    };
                    let mut style = Style::default().fg(fg);
                    if i == c.selected { style = style.bg(bg).add_modifier(Modifier::BOLD); }
                    row.push(Span::styled(format!("  {}  ", b.label), style));
                }
                popup_text.push(Line::from(row).alignment(Alignment::Center));
                let popup_w = Paragraph::new(popup_text)
                    .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(C_BERRY)).title(Span::styled(c.title.clone(), Style::default().fg(C_PINK_DEEP).add_modifier(Modifier::BOLD))));
                f.render_widget(popup_w, popup_area);
            }

            // footer: desc line + key hints line
            let desc = if !app.status_line.is_empty() { app.status_line.clone() } else { app.selected_desc() };
            let keys = if app.confirm.is_some() {
                "\u{2190}\u{2192} choose \u{00B7} Enter confirm \u{00B7} Esc safe option".to_string()
            } else if app.input_mode {
                "Enter confirm \u{00B7} Esc cancel".to_string()
            } else if app.page == Page::Main {
                "\u{2191}\u{2193} move \u{00B7} Enter select".to_string()
            } else if !app.pin_ok && app.page == Page::Settings {
                "Enter PIN \u{00B7} Esc back".to_string()
            } else {
                match app.focus {
                    Focus::Menu => "Enter run \u{00B7} Tab output \u{00B7} Esc back".to_string(),
                    Focus::Output => "Tab/Esc back \u{00B7} \u{2191}\u{2193} scroll \u{00B7} y copy".to_string(),
                }
            };
            let footer = Paragraph::new(vec![
                Line::from(Span::styled(desc, Style::default().fg(C_SKY_DEEP))),
                Line::from(Span::styled(keys, Style::default().fg(C_MUTED))),
            ]).style(Style::default().bg(Color::Black));
            f.render_widget(footer, chunks[2]);
        })?;

        // pump the background job (if any), then wait for input:
        // shorter tick while streaming so output flows smoothly
        app.poll_job();
        let tick = if app.running.is_some() { 100 } else { 200 };
        if event::poll(std::time::Duration::from_millis(tick))? {
            match event::read()? {
                Event::Key(key) => {
                    if key.kind != KeyEventKind::Press { continue; }
                    // confirm dialog: modal over everything (idle or busy)
                    if app.confirm.is_some() {
                        match key.code {
                            KeyCode::Left | KeyCode::Char('h') => { if let Some(c) = app.confirm.as_mut() { c.move_sel(-1); } }
                            KeyCode::Right | KeyCode::Char('l') => { if let Some(c) = app.confirm.as_mut() { c.move_sel(1); } }
                            KeyCode::Enter => {
                                let confirmed = app.confirm.as_ref()
                                    .map(|c| c.buttons[c.selected].danger)
                                    .unwrap_or(false);
                                app.resolve_confirm(confirmed);
                            }
                            // Esc always takes the safe option (selected index 0)
                            KeyCode::Esc => app.resolve_confirm(false),
                            _ => {}
                        }
                        continue;
                    }
                    // while a job runs: Esc/q ask to cancel (instant for read-only jobs),
                    // output scrolling stays live; everything else is locked
                    if app.is_busy() {
                        if app.page != Page::Main && app.focus == Focus::Output {
                            match key.code {
                                KeyCode::Left | KeyCode::Char('h') => { app.cursor_left(); continue; }
                                KeyCode::Right | KeyCode::Char('l') => { app.cursor_right(max_scroll_x); continue; }
                                KeyCode::Up | KeyCode::Char('k') => { app.cursor_up(); continue; }
                                KeyCode::Down | KeyCode::Char('j') => { app.cursor_down(output_height); continue; }
                                KeyCode::PageUp => { app.scroll_up(10); continue; }
                                KeyCode::PageDown => { app.scroll_down(10, max_scroll_y); continue; }
                                KeyCode::Home => { app.follow_tail = false; app.output_scroll = (0, 0); app.cursor = (0, 0); continue; }
                                KeyCode::End => { app.follow_tail = true; app.output_scroll = (max_scroll_y, max_scroll_x); app.cursor = (0, output_height.saturating_sub(1)); continue; }
                                _ => {}
                            }
                        }
                        match key.code {
                            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => {
                                app.request_cancel();
                                continue;
                            }
                            KeyCode::Tab => {
                                if app.page != Page::Main {
                                    if app.focus == Focus::Menu { app.focus = Focus::Output; } else { app.focus = Focus::Menu; }
                                }
                                continue;
                            }
                            _ => {} // locked while busy
                        }
                        continue;
                    }
                    if key.code == KeyCode::BackTab {
                        if app.page != Page::Main { if app.focus == Focus::Menu { app.focus = Focus::Output; } else { app.focus = Focus::Menu; } }
                        continue;
                    }
                    if app.input_mode {
                        match key.code {
                            KeyCode::Enter => app.submit_input(),
                            KeyCode::Esc => app.cancel_input(),
                            KeyCode::Backspace => { app.input.pop(); }
                            KeyCode::Char(c) => {
                                if app.input_action == "pin" {
                                    if c.is_ascii_digit() && app.input.len() < 4 { app.input.push(c); }
                                } else if app.input_action.starts_with("flag") {
                                    if c.is_ascii_graphic() { app.input.push(c); }
                                } else {
                                    if c.is_ascii_alphanumeric() || c == '.' || c == '-' { app.input.push(c); }
                                }
                            }
                            _ => {}
                        }
                        continue;
                    }
                    // 1/2/3 page switch
                    if let KeyCode::Char(c) = key.code {
                        if !key.modifiers.contains(KeyModifiers::CONTROL) && !key.modifiers.contains(KeyModifiers::ALT) {
                            if c == '1' && app.page != Page::Main { app.go_main(); continue; }
                            if c == '2' { app.switch_to_page(Page::Play); continue; }
                            if c == '3' { app.switch_to_page(Page::Settings); continue; }
                        }
                    }
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Char('Q') => {
                            if app.page == Page::Main { break; }
                            else { app.go_main(); continue; }
                        }
                        KeyCode::Esc => {
                            // Esc: Output -> Menu first, Menu -> up one level
                            if app.page != Page::Main {
                                if app.focus == Focus::Output { app.focus = Focus::Menu; }
                                else if app.page == Page::Play { app.go_main(); }
                                else if app.page == Page::Guide || app.page == Page::Flags || app.page == Page::Connect { app.go_play(); }
                                else { app.go_main(); }
                            }
                            continue;
                        }
                        KeyCode::Tab => {
                            if app.page != Page::Main {
                                if app.focus == Focus::Menu { app.focus = Focus::Output; } else { app.focus = Focus::Menu; }
                            }
                            continue;
                        }
                        _ => {}
                    }
                    match app.page {
                        Page::Main => match key.code {
                            KeyCode::Down | KeyCode::Char('j') => app.next_item(),
                            KeyCode::Up | KeyCode::Char('k') => app.prev_item(),
                            KeyCode::Enter => { if app.execute_main() { break; } }
                            _ => {}
                        },
                        _ => match app.focus {
                            Focus::Menu => match key.code {
                                KeyCode::Down | KeyCode::Char('j') => app.next_item(),
                                KeyCode::Up | KeyCode::Char('k') => app.prev_item(),
                                KeyCode::Enter => {
                                    match app.page {
                                        Page::Play => app.execute_play(),
                                        Page::Guide => app.execute_guide(),
                                        Page::Flags => app.execute_flags(),
                                        Page::Connect => app.execute_connect(),
                                        _ => app.execute_settings(),
                                    }
                                }
                                // guide: flip steps with arrows while the menu has focus
                                KeyCode::Right if app.page == Page::Guide => app.guide_next_step(),
                                KeyCode::Left if app.page == Page::Guide => app.guide_prev_step(),
                                _ => {}
                            },
                            Focus::Output => match key.code {
                                KeyCode::Left | KeyCode::Char('h') => { app.cursor_left(); }
                                KeyCode::Right | KeyCode::Char('l') => { app.cursor_right(max_scroll_x); }
                                KeyCode::Up | KeyCode::Char('k') => { app.cursor_up(); }
                                KeyCode::Down | KeyCode::Char('j') => { app.cursor_down(output_height); }
                                KeyCode::PageUp => app.scroll_up(10),
                                KeyCode::PageDown => app.scroll_down(10, max_scroll_y),
                                KeyCode::Home => { app.follow_tail = false; app.output_scroll = (0, 0); app.cursor = (0, 0); }
                                KeyCode::End => { app.follow_tail = true; app.output_scroll = (max_scroll_y, max_scroll_x); app.cursor = (0, output_height.saturating_sub(1)); }
                                KeyCode::Char('y') | KeyCode::Char('Y') => app.copy_all(),
                                _ => {}
                            }
                        }
                    }
                }
                Event::Mouse(m) => {
                    if app.input_mode || app.confirm.is_some() { continue; }
                    // while a job runs only the wheel is live (no click-selection)
                    if app.is_busy() && !matches!(m.kind, MouseEventKind::ScrollUp | MouseEventKind::ScrollDown) { continue; }
                    let in_output = app.page != Page::Main
                        && m.column >= output_rect.x + 1 && m.column < output_rect.x + output_rect.width - 1
                        && m.row >= output_rect.y + 1 && m.row < output_rect.y + output_rect.height - 1;
                    let in_menu = m.column >= menu_rect.x && m.column < menu_rect.x + menu_rect.width
                        && m.row >= menu_rect.y && m.row < menu_rect.y + menu_rect.height;
                    match m.kind {
                        MouseEventKind::Down(MouseButton::Left) => {
                            if in_output {
                                // start text selection
                                app.focus = Focus::Output;
                                let col = app.output_scroll.1 + (m.column - output_rect.x - 1);
                                let row = app.output_scroll.0 + (m.row - output_rect.y - 1);
                                app.sel = Some(((col, row), (col, row)));
                                app.warp_to(m.column - output_rect.x - 1, m.row - output_rect.y - 1, output_width, output_height);
                            } else if in_menu {
                                let idx = (m.row - menu_rect.y - 1) as usize;
                                if idx < app.current_items_len() {
                                    if app.page == Page::Settings && !app.pin_ok { app.settings_state.select(Some(0)); }
                                    else { app.current_state_mut().select(Some(idx)); }
                                    app.focus = Focus::Menu;
                                }
                            }
                        }
                        MouseEventKind::Drag(MouseButton::Left) => {
                            if in_output {
                                app.focus = Focus::Output;
                                if let Some((_, end)) = &mut app.sel {
                                    let col = app.output_scroll.1 + (m.column - output_rect.x - 1);
                                    let row = app.output_scroll.0 + (m.row - output_rect.y - 1);
                                    *end = (col, row);
                                }
                            }
                        }
                        MouseEventKind::Up(MouseButton::Left) => app.copy_selection(),
                        MouseEventKind::ScrollUp => {
                            if app.page != Page::Main && app.focus == Focus::Output { app.scroll_up(3); } else { app.prev_item(); }
                        }
                        MouseEventKind::ScrollDown => {
                            if app.page != Page::Main && app.focus == Focus::Output { app.scroll_down(3, max_scroll_y); } else { app.next_item(); }
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }
    }
    // never leave a child behind: kill+reap before restoring the terminal
    app.cancel_job();
    let _ = app.poll_job();
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
    terminal.show_cursor()?;
    if !app.output.is_empty() {
        println!("\n--- Last output ---\n{}", app.output);
    }
    Ok(())
}
