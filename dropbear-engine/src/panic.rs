//! This module describes what happens when a panic occurs by setting up
//! a custom hook.

use std::panic;
use arboard::Clipboard;
use rfd::{MessageDialog, MessageLevel};

/// Creates a new panic hook for crash detection. Pretty nice for debugging.
pub fn set_hook() {
    panic::set_hook(Box::new(|info| {
        let msg = if let Some(s) = info.payload().downcast_ref::<&str>() {
            *s
        } else if let Some(s) = info.payload().downcast_ref::<String>() {
            s.as_str()
        } else {
            "Unknown panic payload"
        };

        let location = info
            .location()
            .map(|l| format!("{}:{}", l.file(), l.line()))
            .unwrap_or_else(|| "unknown location".to_string());

        let full_text = format!(
            "The application has encountered a fatal error and must close. Sorry :(\n\n\
             Location: {}\nError: {}\n\nPlease report this error to the developers \
             and attach the log please :)\n\nFor your convenience, the error message has been \
             copied to your clipboard to put straight to Google lol\n",
            location, msg
        );

        log::error!("PANIC AT THE REACTOR! SHUTDOWN SHUTDOWN SHUT THIS SHIT DOWN!!!\n\n\
=========================================================================
{}
=========================================================================\n\n\
        ", full_text.clone());

        if let Ok(mut clipboard) = Clipboard::new() {
            let _ = clipboard.set_text(full_text.clone());
        }

        let _ = MessageDialog::new()
            .set_title("Panic!")
            .set_description(&full_text)
            .set_level(MessageLevel::Error)
            .show();
    }));
}
