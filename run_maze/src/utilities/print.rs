use crate::maze;

use crossterm::{cursor, terminal, ExecutableCommand, QueueableCommand};
use std::io::{stdout, Write};

// The mazes look WAY better if the cursor square disapears while it builds.
#[derive(Clone)]
pub struct InvisibleCursor;

impl InvisibleCursor {

    pub fn new() -> Self {
        Self
    }

    pub fn hide(&self) {
        stdout().execute(cursor::Hide).expect("Failed to hide cursor.");
    }

}

impl Drop for InvisibleCursor {
    fn drop(&mut self) {
        stdout().execute(cursor::Show).expect(
            "Failed to unhide your cursor. Sorry! Restart your terminal."
        );
    }
}

// DO NOT use this in this unless you are exiting program early and Rust won't call drop.
pub fn unhide_cursor_on_process_exit() {
    stdout().execute(cursor::Show).expect(
        "Failed to unhide your cursor. Sorry! Restart your terminal."
    );
}

// Execute the command so clearing the screen forcefully flushes for the caller.
pub fn clear_screen() {
    stdout()
        .execute(terminal::Clear(terminal::ClearType::All))
        .expect("Could not clear screen, terminal may be incompatible.");
}

// Queue the command so setting the cursor position does NOT forcefully flush for caller.
pub fn set_cursor_position(p: maze::Point) {
    stdout()
        .queue(cursor::MoveTo((p.col) as u16, (p.row) as u16))
        .expect("Could not move cursor, terminal may be incompatible.");
}

pub fn flush() {
    stdout()
        .flush()
        .expect("Could not clear screen,terminal may be incompatible.");
}
