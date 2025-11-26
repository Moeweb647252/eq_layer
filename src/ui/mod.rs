use crate::{
    settings::Settings,
    ui::command::{Command, Info, State},
};
use eframe::egui::CentralPanel;
use std::sync::mpsc::SyncSender;

pub mod command;
mod equalizer;
mod graph;
mod heading;

pub struct App {
    eq_settings: Settings,
    sender: SyncSender<Command>,
    state: State,
    info: Info,
}

impl App {
    pub fn new(
        eq_settings: Settings,
        sender: SyncSender<Command>,
        state: State,
        info: Info,
    ) -> Self {
        Self {
            eq_settings,
            sender,
            state,
            info,
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        CentralPanel::default().show(ctx, |ui| {
            ui.vertical(|ui| {
                self.heading_ui(ui);
                self.equalizer_ui(ui);
                self.graph_ui(ui);
            })
        });
    }
}
