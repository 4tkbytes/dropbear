//! Used to aid with debugging any issues with the editor.

use crate::editor::Signal;
use egui::Ui;

pub(crate) fn show_menu_bar(ui: &mut Ui, signal: &mut Signal) {
    ui.menu_button("Debug", |ui_debug| {
        if ui_debug.button("Panic").clicked() {
            log::warn!("Panic caused on purpose from Menu Button Click");
            panic!("Testing out panicking with new panic module, this is a test")
        }

        if ui_debug.button("Show Entities Loaded").clicked() {
            log::debug!("Show Entities Loaded under Debug Menu is clicked");
            *signal = Signal::LogEntities;
        }

        if ui_debug.button("size_of::<Editor>()").clicked() {
            log::debug!("size_of::<Editor>() is clicked");
            let size = size_of::<crate::editor::Editor>();
            log::info!("size_of::<Editor>() is {}", size);
            log::debug!("I'm so fat - editor")
        }
    });
}
