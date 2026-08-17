use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color as RColor, Modifier, Style};
use unicode_width::UnicodeWidthStr;
use vt100::Parser;

const SCROLLBACK_LEN: usize = 10_000;

/// A vt100-backed terminal emulator renderable as a ratatui widget.
pub struct TerminalPane {
    parser: Parser,
    rows: u16,
    cols: u16,
    /// Set when the remote side requested a different size than the widget
    /// provides; the caller re-issues resize once the widget grows.
    pub dirty: bool,
}

impl TerminalPane {
    pub fn new(cols: u16, rows: u16) -> Self {
        Self {
            parser: Parser::new(rows, cols, SCROLLBACK_LEN),
            rows,
            cols,
            dirty: false,
        }
    }

    pub fn feed(&mut self, bytes: &[u8]) {
        self.parser.process(bytes);
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
        self.dirty = true;
    }

    fn to_rcolor(c: vt100::Color) -> RColor {
        match c {
            vt100::Color::Default => RColor::Reset,
            vt100::Color::Idx(i) => RColor::Indexed(i),
            vt100::Color::Rgb(r, g, b) => RColor::Rgb(r, g, b),
        }
    }

    pub fn render(&self, area: Rect, buf: &mut Buffer, focused: bool, highlight_cursor: bool) {
        let screen = self.parser.screen();
        let (rows, cols) = screen.size();
        let draw_rows = rows.min(area.height) as usize;
        let draw_cols = cols.min(area.width) as usize;

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

        // Cursor
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