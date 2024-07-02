use std::io;
use std::rc::Rc;

use crossterm::style::{self, Stylize};
use crossterm::{cursor, queue};

use super::AppColors;

pub const VERTICAL: &str = "│";
pub const HORIZONTAL: &str = "─";
pub const TOP_RIGHT: &str = "┐";
pub const TOP_LEFT: &str = "┌";
pub const BOTTOM_RIGHT: &str = "┘";
pub const BOTTOM_LEFT: &str = "└";
pub const TOP_TEE: &str = "┬";
pub const BOTTOM_TEE: &str = "┴";


/// Panels abstract away a terminal "window" (section of the screen),
/// and handle all methods associated with writing data to that window.
/// A panel includes a border and margin around the edge of the window,
/// and a title that appears at the top. Margins are set individually,
/// in the order (top, right, bottom, left). The Panel will translate
/// the x and y coordinates to account for the border and margins, so
/// users of the methods can calculate rows and columns relative to the
/// Panel (i.e., x = 0 and y = 0 represent the top-left printable
/// cell in the window).
#[derive(Debug)]
pub struct Panel {
	screen_pos: usize,
	pub colors: Rc<AppColors>,
	title: String,
	start_x: u16,
	n_row: u16,
	n_col: u16,
	margins: (u16, u16, u16, u16),
}

impl Panel {
	/// Creates a new panel.
	pub fn new(
		title: String,
		screen_pos: usize,
		colors: Rc<AppColors>,
		n_row: u16,
		n_col: u16,
		start_x: u16,
		margins: (u16, u16, u16, u16),
	) -> Self {
		return Panel {
			screen_pos: screen_pos,
			colors: colors,
			title: title,
			start_x: start_x,
			n_row: n_row,
			n_col: n_col,
			margins: margins,
		};
	}

	/// Redraws borders and refreshes the window to display on terminal.
	pub fn redraw(&self) {
		self.clear();
		self.draw_border();
	}

	/// Clears the whole Panel.
	pub fn clear(&self) {
		let empty = vec![" "; self.n_col as usize];
		let empty_string = empty.join("");
		for r in 0..(self.n_row - 1) {
			queue!(
				io::stdout(),
				cursor::MoveTo(self.start_x, r),
				style::PrintStyledContent(
					style::style(&empty_string)
						.with(self.colors.normal.0)
						.on(self.colors.normal.1)
				),
			)
			.unwrap();
		}
	}

	/// Clears the inner section of the Panel, leaving the borders
	/// intact.
	pub fn clear_inner(&self) {
		let empty = vec![" "; self.n_col as usize - 2];
		let empty_string = empty.join("");
		for r in 1..(self.n_row - 1) {
			queue!(
				io::stdout(),
				cursor::MoveTo(self.start_x + 1, r),
				style::PrintStyledContent(
					style::style(&empty_string)
						.with(self.colors.normal.0)
						.on(self.colors.normal.1)
				),
			)
			.unwrap();
		}
	}

	/// Draws a border around the window.
	fn draw_border(&self) {
		let top_left;
		let bot_left;
		match self.screen_pos {
			0 => {
				top_left = TOP_LEFT;
				bot_left = BOTTOM_LEFT;
			}
			_ => {
				top_left = TOP_TEE;
				bot_left = BOTTOM_TEE;
			}
		}
		let mut border_top = vec![top_left];
		let mut border_bottom = vec![bot_left];
		for _ in 0..(self.n_col - 2) {
			border_top.push(HORIZONTAL);
			border_bottom.push(HORIZONTAL);
		}
		border_top.push(TOP_RIGHT);
		border_bottom.push(BOTTOM_RIGHT);

		queue!(
			io::stdout(),
			style::SetColors(style::Colors::new(
				self.colors.normal.0,
				self.colors.normal.1
			)),
			cursor::MoveTo(self.start_x, 0),
			style::Print(border_top.join("")),
			cursor::MoveTo(self.start_x, self.n_row - 1),
			style::Print(border_bottom.join("")),
		)
		.unwrap();

		for r in 1..(self.n_row - 1) {
			queue!(
				io::stdout(),
				cursor::MoveTo(self.start_x, r),
				style::Print(VERTICAL.to_string()),
				cursor::MoveTo(self.start_x + self.n_col - 1, r),
				style::Print(VERTICAL.to_string()),
			)
			.unwrap();
		}

		queue!(
			io::stdout(),
			cursor::MoveTo(self.start_x + 2, 0),
			style::Print(&self.title),
			style::ResetColor,
		)
		.unwrap();
	}

	/// Writes a line of text to the window. Note that this does not do
	/// checking for line length, so strings that are too long will end
	/// up wrapping and may mess up the format. Use `write_wrap_line()`
	/// if you need line wrapping.
	pub fn write_line(&self, y: u16, string: String, style: Option<style::ContentStyle>) {
		let styled = match style {
			Some(style) => style.apply(string),
			None => style::style(string)
				.with(self.colors.normal.0)
				.on(self.colors.normal.1),
		};
		queue!(
			io::stdout(),
			cursor::MoveTo(self.abs_x(0), self.abs_y(y)),
			style::PrintStyledContent(styled)
		)
		.unwrap();
	}

	/// Writes a line of styled text to the window, representing a key
	/// and value. The text will be shown as "key: value", and styled
	/// with the provided styles. Note that this does not do checking
	/// for line length, so strings that are too long will end up
	/// wrapping and may mess up the format. Use `write_wrap_line()` if
	/// you need line wrapping.
	pub fn write_key_value_line(
		&self,
		y: u16,
		mut key: String,
		mut value: String,
		key_style: Option<style::ContentStyle>,
		value_style: Option<style::ContentStyle>,
	) {
		key.push(':');
		value.insert(0, ' ');

		queue!(io::stdout(), cursor::MoveTo(self.abs_x(0), self.abs_y(y))).unwrap();

		let key_styled = match key_style {
			Some(kstyle) => kstyle.apply(key),
			None => style::style(key)
				.with(self.colors.normal.0)
				.on(self.colors.normal.1),
		};
		queue!(io::stdout(), style::PrintStyledContent(key_styled)).unwrap();
		let value_styled = match value_style {
			Some(vstyle) => vstyle.apply(value),
			None => style::style(value)
				.with(self.colors.normal.0)
				.on(self.colors.normal.1),
		};
		queue!(io::stdout(), style::PrintStyledContent(value_styled)).unwrap();
	}

	/// Writes one or more lines of text from a String, word wrapping
	/// when necessary. `start_y` refers to the row to start at (word
	/// wrapping makes it unknown where text will end). Returns the row
	/// on which the text ended.
	pub fn write_wrap_line(
		&self,
		start_y: u16,
		string: &str,
		style: Option<style::ContentStyle>,
	) -> u16 {
		let mut row = start_y;
		let max_row = self.get_rows();
		if row >= max_row {
			return row;
		}
		let content_style = match style {
			Some(style) => style,
			None => style::ContentStyle::new()
				.with(self.colors.normal.0)
				.on(self.colors.normal.1),
		};
		let wrapper = textwrap::wrap(string, self.get_cols() as usize);
		for line in wrapper {
			queue!(
				io::stdout(),
				cursor::MoveTo(self.abs_x(0), self.abs_y(row)),
				style::PrintStyledContent(content_style.apply(line))
			)
			.unwrap();
			row += 1;

			if row >= max_row {
				break;
			}
		}
		return row - 1;
	}

	/// Updates window size.
	pub fn resize(&mut self, n_row: u16, n_col: u16, start_x: u16) {
		self.n_row = n_row;
		self.n_col = n_col;
		self.start_x = start_x;
	}

	/// Returns the effective number of rows (accounting for borders
	/// and margins).
	pub fn get_rows(&self) -> u16 {
		// 2 for borders on top and bottom
		return self.n_row - self.margins.0 - self.margins.2 - 2;
	}

	/// Returns the effective number of columns (accounting for
	/// borders and margins).
	pub fn get_cols(&self) -> u16 {
		// 2 for borders on left and right
		return self.n_col - self.margins.1 - self.margins.3 - 2;
	}

	/// Calculates the y-value relative to the terminal rather than to
	/// the panel (i.e., taking into account borders and margins).
	fn abs_y(&self, y: u16) -> u16 {
		return y + self.margins.0 + 1;
	}

	/// Calculates the x-value relative to the terminal rather than to
	/// the panel (i.e., taking into account borders and margins).
	fn abs_x(&self, x: u16) -> u16 {
		return x + self.start_x + self.margins.3 + 1;
	}
}
