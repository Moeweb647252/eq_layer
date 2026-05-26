use crate::{
    eq::EqProfile,
    settings::Settings,
    ui::command::{Command, Info, State},
    utils::DerefMutHook,
};
use eframe::egui::{self, CentralPanel};
use std::sync::mpsc::SyncSender;
use tracing::debug;

pub mod command;
mod equalizer;
mod graph;
mod heading;

pub struct App {
    eq_settings: Settings,
    eq_profile: DerefMutHook<EqProfile>,
    eq_settings_back: Settings,
    eq_profile_back: EqProfile,
    sender: SyncSender<Command>,
    state: State,
    info: Info,
    window_hidden: bool,
    quitting: bool,
}

impl App {
    pub fn new(
        eq_settings: Settings,
        eq_profile: EqProfile,
        sender: SyncSender<Command>,
        state: State,
        info: Info,
    ) -> Self {
        Self {
            eq_settings_back: eq_settings.clone(),
            eq_profile_back: eq_profile.clone(),
            eq_settings,
            eq_profile: DerefMutHook::new(eq_profile),
            sender,
            state,
            info,
            window_hidden: false,
            quitting: false,
        }
    }
}

impl eframe::App for App {
    fn logic(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        debug!("logic tick");
        if self.window_hidden
            && ctx.has_requested_repaint()
            && ctx.input(|i| i.viewport().focused == Some(true))
        {
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
            self.window_hidden = false;
        }
    }

    fn ui(&mut self, ui: &mut eframe::egui::Ui, _frame: &mut eframe::Frame) {
        let close_requested = ui.ctx().input(|i| i.viewport().close_requested());

        if close_requested && !self.quitting {
            ui.ctx()
                .send_viewport_cmd(egui::ViewportCommand::CancelClose);
            ui.ctx()
                .send_viewport_cmd(egui::ViewportCommand::Visible(false));
            self.window_hidden = true;
            return;
        }
        CentralPanel::default().show_inside(ui, |ui| {
            ui.vertical(|ui| {
                self.heading_ui(ui);
                self.equalizer_ui(ui);
                self.graph_ui(ui);
            })
        });
    }
}
