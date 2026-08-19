use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color as RColor, Modifier, Style};
use std::collections::VecDeque;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};
use vt100::Parser;

/// A rectangular selection over the *visible* grid coordinates. `end_col` is
/// exclusive (same convention as `Screen::contents_between`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Selection {
    pub start_row: u16,
    pub start_col: u16,
    pub end_row: u16,
    pub end_col: u16,
}

/// A vt100-backed terminal emulator renderable as a ratatui widget.
///
/// Scrollback is served from a private plain-text line ring buffer: vt100 0.15
/// can only offset its visible view by at most one screen height before its
/// grid math underflows, so deep history is captured here line-by-line as the
/// remote output streams in. Live-screen rows keep their full ANSI styling;
/// scrolled-back rows render in a dim style.
pub struct TerminalPane {
    parser: Parser,
    rows: u16,
    cols: u16,
    /// Set when the remote side requested a different size than the widget
    /// provides; the caller re-issues resize once the widget grows.
    pub dirty: bool,
    /// Completed output lines, newest at the back.
    history: VecDeque<String>,
    /// Partially received line (no trailing newline yet).
    pending: String,
    /// Current scrollback view offset: how many lines back from the live
    /// screen (0 = live).
    scroll_offset: usize,
    /// Maximum number of scrollback lines retained (configurable via settings).
    scrollback: usize,
}

impl TerminalPane {
    pub fn new(cols: u16, rows: u16, scrollback: usize) -> Self {
        let scrollback = scrollback.max(100);
        Self {
            parser: Parser::new(rows, cols, scrollback),
            rows,
            cols,
            dirty: false,
            history: VecDeque::new(),
            pending: String::new(),
            scroll_offset: 0,
            scrollback,
        }
    }

    pub fn feed(&mut self, bytes: &[u8]) {
        self.parser.process(bytes);
        // Capture history lines from the same byte stream.
        let text = String::from_utf8_lossy(bytes);
        self.pending.push_str(&text);
        let mut added = 0usize;
        let taken = std::mem::take(&mut self.pending);
        let mut rest = taken.as_str();
        while let Some(idx) = rest.find('\n') {
            let line = rest[..idx].to_string();
            rest = &rest[idx + 1..];
            self.push_line(line);
            added += 1;
        }
        self.pending.push_str(rest);
        // Bound the partially received line (e.g. carriage-return progress
        // spinners) so it can never grow without limit.
        if self.pending.len() > 4096 {
            let flushed = std::mem::take(&mut self.pending);
            self.push_line(flushed);
            added += 1;
        }
        // Keep the scrolled view pinned to the same lines: new output pushes
        // history forward, so the offset must grow by the same amount.
        if self.scroll_offset > 0 && added > 0 {
            self.scroll_offset = (self.scroll_offset + added).min(self.history.len());
        }
    }

    fn push_line(&mut self, line: String) {
        let line = line.strip_suffix('\r').unwrap_or(&line).to_string();
        let truncated = truncate_by_width(&line, self.cols as usize);
        self.history.push_back(truncated);
        while self.history.len() > self.scrollback {
            self.history.pop_front();
        }
    }

    pub fn size(&self) -> (u16, u16) {
        (self.rows, self.cols)
    }

    /// Absolute cursor position on the live screen (unaffected by scrolling).
    pub fn cursor_position(&self) -> (u16, u16) {
        self.parser.screen().cursor_position()
    }

    /// Resize the emulator to match the widget area.
    pub fn resize(&mut self, cols: u16, rows: u16) {
        if cols == 0 || rows == 0 {
            return;
        }
        if cols == self.cols && rows == self.rows {
            return;
        }
        self.cols = cols;
        self.rows = rows;
        self.parser.set_size(rows, cols);
        // Re-truncate captured history to the new width.
        for line in &mut self.history {
            let t = truncate_by_width(line, cols as usize);
            line.clear();
            line.push_str(&t);
        }
        self.scroll_to_bottom();
        self.dirty = true;
    }

    // ------------------------------------------------------------------
    // Scrollback view
    // ------------------------------------------------------------------

    pub fn scroll_offset(&self) -> usize {
        self.scroll_offset
    }

    pub fn is_at_bottom(&self) -> bool {
        self.scroll_offset == 0
    }

    /// Scroll up `n` lines into the scrollback. Returns the new offset.
    pub fn scroll_up(&mut self, n: usize) -> usize {
        self.scroll_offset = (self.scroll_offset + n).min(self.history.len());
        self.scroll_offset
    }

    /// Scroll down `n` lines toward the live screen. Returns the new offset.
    pub fn scroll_down(&mut self, n: usize) -> usize {
        self.scroll_offset = self
            .scroll_offset
            .saturating_sub(n)
            .min(self.history.len());
        self.scroll_offset
    }

    /// Jump to the top of the scrollback. Returns the new offset.
    pub fn scroll_top(&mut self) -> usize {
        self.scroll_offset = self.history.len();
        self.scroll_offset
    }

    /// Return to the live screen.
    pub fn scroll_to_bottom(&mut self) {
        self.scroll_offset = 0;
    }

    /// The plain text of the line shown at visible row `r` in the current view
    /// (`None` for blank rows below the start of history).
    fn view_line(&self, r: usize, cols: u16) -> Option<String> {
        let k = self.scroll_offset;
        let idx = self.history.len().saturating_sub(k) + r;
        if idx < self.history.len() {
            return self.history.get(idx).cloned();
        }
        // Live-screen row (only reachable when the view includes the screen).
        if idx >= self.history.len() + usize::from(self.rows) {
            return None;
        }
        let screen_row = (idx - self.history.len()) as u16;
        if screen_row >= self.rows {
            return None;
        }
        Some(self.parser.screen().rows(0, cols).nth(usize::from(screen_row)).unwrap_or_default())
    }

    /// Extract the plain text of the selected region over the visible view.
    pub fn selection_text(&self, sel: Selection) -> String {
        let mut s = sel;
        let max_row = self.rows.saturating_sub(1);
        s.start_row = s.start_row.min(max_row);
        s.end_row = s.end_row.min(max_row);
        s.start_col = s.start_col.min(self.cols.saturating_sub(1));
        s.end_col = s.end_col.min(self.cols);
        if (s.start_row, s.start_col) > (s.end_row, s.end_col) {
            std::mem::swap(&mut s.start_row, &mut s.end_row);
            std::mem::swap(&mut s.start_col, &mut s.end_col);
        }
        let mut out = String::new();
        for r in s.start_row..=s.end_row {
            let line = self.view_line(usize::from(r), self.cols).unwrap_or_default();
            let chars: Vec<char> = line.chars().collect();
            let start = usize::from(if r == s.start_row { s.start_col } else { 0 });
            let end = usize::from(if r == s.end_row { s.end_col } else { self.cols });
            let start = start.min(chars.len());
            let end = end.min(chars.len());
            if start < end {
                out.extend(&chars[start..end]);
            }
            if r < s.end_row {
                out.push('\n');
            }
        }
        out
    }

    fn to_rcolor(c: vt100::Color) -> RColor {
        match c {
            vt100::Color::Default => RColor::Reset,
            vt100::Color::Idx(i) => RColor::Indexed(i),
            vt100::Color::Rgb(r, g, b) => RColor::Rgb(r, g, b),
        }
    }

    fn cell_selected(sel: Option<Selection>, cols: u16, row: u16, col: u16) -> bool {
        let Some(sel) = sel else {
            return false;
        };
        let index = |r: u16, c: u16| u32::from(r) * u32::from(cols) + u32::from(c);
        let start = index(sel.start_row, sel.start_col);
        let end = index(sel.end_row, sel.end_col);
        let cur = index(row, col);
        cur >= start && cur < end
    }

    pub fn render(
        &self,
        area: Rect,
        buf: &mut Buffer,
        focused: bool,
        highlight_cursor: bool,
        selection: Option<Selection>,
    ) {
        let draw_rows = self.rows.min(area.height) as usize;
        let draw_cols = self.cols.min(area.width) as usize;

        if self.scroll_offset > 0 {
            // Scrolled view: dim plain-text history rows, blank padding below.
            for row in 0..draw_rows {
                let line = self.view_line(row, self.cols).unwrap_or_default();
                let y = area.y + row as u16;
                let mut style = Style::default().fg(RColor::DarkGray);
                for (col, ch) in line.chars().take(draw_cols).enumerate() {
                    let x = area.x + col as u16;
                    if selection.is_some_and(|sel| {
                        Self::cell_selected(Some(sel), self.cols, row as u16, col as u16)
                    }) {
                        style = style.add_modifier(Modifier::REVERSED);
                    }
                    buf[(x, y)].set_symbol(&ch.to_string()).set_style(style);
                }
            }
            return;
        }

        // Live view: full-color rendering from the vt100 screen.
        let screen = self.parser.screen();
        for row in 0..draw_rows {
            for col in 0..draw_cols {
                let Some(cell) = screen.cell(row as u16, col as u16) else {
                    continue;
                };
                if cell.is_wide_continuation() {
                    continue;
                }
                let mut style = Style::default();
                if cell.bold() {
                    style = style.add_modifier(Modifier::BOLD);
                }
                if cell.italic() {
                    style = style.add_modifier(Modifier::ITALIC);
                }
                if cell.underline() {
                    style = style.add_modifier(Modifier::UNDERLINED);
                }
                if cell.inverse() {
                    style = style.add_modifier(Modifier::REVERSED);
                }
                style = style.fg(Self::to_rcolor(cell.fgcolor()));
                style = style.bg(Self::to_rcolor(cell.bgcolor()));

                let x = area.x + col as u16;
                let y = area.y + row as u16;
                if Self::cell_selected(selection, self.cols, row as u16, col as u16) {
                    style = style.add_modifier(Modifier::REVERSED);
                }
                let symbol = cell.contents();
                if symbol.is_empty() {
                    buf[(x, y)].set_symbol(" ").set_style(style);
                } else {
                    buf[(x, y)].set_symbol(&symbol).set_style(style);
                    let w = UnicodeWidthStr::width(symbol.as_str()).max(1);
                    if w > 1 {
                        // Wide glyph: blank the following cells so the host
                        // terminal keeps alignment.
                        let mut i = 1u16;
                        while i < w as u16 && x + i < area.x + area.width {
                            buf[(x + i, y)].set_symbol(" ").set_style(style);
                            i += 1;
                        }
                    }
                }
            }
        }

        // Cursor — only meaningful on the live screen (scrolled views hide it).
        if focused && highlight_cursor && !screen.hide_cursor() {
            let (crow, ccol) = screen.cursor_position();
            if crow < draw_rows as u16 && ccol < draw_cols as u16 {
                let x = area.x + ccol;
                let y = area.y + crow;
                let existing = buf[(x, y)].symbol().to_string();
                let style = buf[(x, y)].style();
                buf[(x, y)]
                    .set_symbol(&existing)
                    .set_style(style.add_modifier(Modifier::REVERSED));
            }
        }
    }
}

/// Truncate `line` so its display width is at most `max_width` chars.
fn truncate_by_width(line: &str, max_width: usize) -> String {
    if max_width == 0 {
        return String::new();
    }
    let mut out = String::new();
    let mut width = 0usize;
    for ch in line.chars() {
        let w = UnicodeWidthChar::width(ch).unwrap_or(0);
        if width + w > max_width {
            break;
        }
        out.push(ch);
        width += w;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn paged_pane() -> TerminalPane {
        let mut p = TerminalPane::new(40, 5, 10_000);
        for i in 0..30 {
            p.feed(format!("line {i}\r\n").as_bytes());
        }
        p
    }

    #[test]
    fn scroll_up_offsets_and_bottom_resets() {
        let mut p = paged_pane();
        assert!(p.is_at_bottom());
        let off = p.scroll_up(3);
        assert_eq!(off, 3);
        assert!(!p.is_at_bottom());
        p.scroll_to_bottom();
        assert!(p.is_at_bottom());
        assert_eq!(p.scroll_offset(), 0);
    }

    #[test]
    fn scroll_up_clamps_at_top() {
        let mut p = paged_pane();
        let top = p.scroll_top();
        assert_eq!(top, 30);
        let again = p.scroll_up(1000);
        assert_eq!(again, top); // clamped, no overflow
    }

    #[test]
    fn scroll_down_returns_to_bottom() {
        let mut p = paged_pane();
        p.scroll_top();
        assert!(!p.is_at_bottom());
        let off = p.scroll_down(1000);
        assert_eq!(off, 0);
        assert!(p.is_at_bottom());
    }

    #[test]
    fn history_captures_lines_across_feeds() {
        let mut p = TerminalPane::new(40, 5, 10_000);
        p.feed(b"hel");
        p.feed(b"lo world\nsecond line\n");
        p.scroll_top();
        let top_line = p.view_line(0, 40).expect("history row 0");
        assert_eq!(top_line, "hello world");
    }

    #[test]
    fn scroll_offset_pins_view_when_new_output_arrives() {
        let mut p = paged_pane();
        p.scroll_up(10); // offset 10
        for i in 30..35 {
            p.feed(format!("line {i}\r\n").as_bytes());
        }
        // 5 new lines push history forward; the offset must grow to keep the
        // same lines in view: idx = len - offset + 0 stays at line 20.
        assert_eq!(p.scroll_offset(), 15);
        let line = p.view_line(0, 40).expect("history row 0");
        assert_eq!(line, "line 20");
    }

    #[test]
    fn partial_line_survives_across_feeds() {
        let mut p = TerminalPane::new(40, 5, 10_000);
        p.feed(b"abc\r\n");
        p.feed(b"def");
        p.scroll_top();
        // "def" has no newline yet: history holds only the completed line.
        assert_eq!(p.view_line(0, 40).unwrap(), "abc");
        p.feed(b"\n");
        p.scroll_top();
        assert_eq!(p.view_line(1, 40).unwrap(), "def");
    }

    #[test]
    fn selection_text_extracts_scrolled_rows() {
        let mut p = paged_pane();
        p.scroll_top();
        let sel = Selection {
            start_row: 0,
            start_col: 0,
            end_row: 0,
            end_col: 6, // exclusive
        };
        let text = p.selection_text(sel);
        assert_eq!(text, "line 0");
    }

    #[test]
    fn selection_text_handles_backwards_selection() {
        let mut p = paged_pane();
        p.scroll_top();
        // Reverse direction: end before start in linear order.
        let sel = Selection {
            start_row: 2,
            start_col: 0,
            end_row: 1,
            end_col: 0, // exclusive
        };
        let text = p.selection_text(sel);
        // After swap: start=(1,0), end=(2,0) -> row 1 in full + newline + empty row 2 slice.
        assert_eq!(text, "line 1\n");
    }
}