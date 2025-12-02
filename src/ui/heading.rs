use std::{ops::DerefMut, str::FromStr, sync::mpsc};

use eframe::egui::{ComboBox, DragValue, Widget};

use crate::{
    eq::EqProfile,
    ui::{
        App,
        command::{Command, SetDevice, SetRealtime},
    },
};

impl App {
    pub fn heading_ui(&mut self, ui: &mut eframe::egui::Ui) {
        ui.horizontal(|ui| {
            if !self.state.realtime
                && ui
                    .button(if self.state.running { "Stop" } else { "Start" })
                    .clicked()
            {
                self.state.running = !self.state.running;
                self.sender.send(Command::SetState(self.state)).ok();
            }
            if ui
                .button(if self.state.enabled {
                    "Disable EQ"
                } else {
                    "Enable EQ"
                })
                .clicked()
            {
                self.state.enabled = !self.state.enabled;
                self.sender.send(Command::SetState(self.state)).ok();
            }
            ui.label("Inp:");
            ComboBox::new("inp_dev", "")
                .selected_text(self.info.input_dev.as_str())
                .show_ui(ui, |ui| {
                    for i in self.info.device_names.iter() {
                        if ui
                            .selectable_value(&mut self.info.input_dev, i.to_owned(), i)
                            .clicked()
                        {
                            println!("Changed");
                            self.sender
                                .send(Command::SetDevice(SetDevice::Input, i.clone()))
                                .ok();
                        }
                    }
                });
            ui.label("Out:");
            ComboBox::new("out_dev", "")
                .selected_text(self.info.output_dev.as_str())
                .show_ui(ui, |ui| {
                    for i in self.info.device_names.iter() {
                        if ui
                            .selectable_value(&mut self.info.output_dev, i.to_owned(), i)
                            .clicked()
                        {
                            self.sender
                                .send(Command::SetDevice(SetDevice::Output, i.clone()))
                                .ok();
                        }
                    }
                });
            if ui.button("Load").clicked() {
                if let Some(path) = rfd::FileDialog::new().pick_file()
                    && let Ok(content) = std::fs::read_to_string(path)
                    && let Ok(profile) = EqProfile::from_str(content.as_str())
                {
                    *self.eq_profile.deref_mut() = profile;
                    self.sender
                        .send(Command::UpdateSettings(self.eq_settings.clone()))
                        .ok();
                }
            }
            ui.label("Latency:");
            DragValue::new(&mut self.eq_settings.latency)
                .speed(1)
                .range(1.0..=1000.0)
                .ui(ui);
            ui.label("Preamp:");
            DragValue::new(&mut self.eq_profile.preamp_db)
                .speed(0.1)
                .range(-12.0..=12.0)
                .ui(ui);
            if ui.button("Reset").clicked() {
                *self.eq_profile.deref_mut() = EqProfile::default();
            }
            if ui.button("Undo").clicked() {
                *self.eq_profile.deref_mut() = self.eq_profile_back.clone();
            }
            if ui.checkbox(&mut self.state.realtime, "Realtime").changed() {
                if self.state.realtime {
                    let (sender, receiver) = mpsc::channel();
                    self.sender
                        .send(Command::SetRealtime(SetRealtime::On(receiver)))
                        .ok();
                    self.eq_profile.set_callback(move |p| {
                        sender.send(p.clone()).ok();
                    });
                } else {
                    self.sender
                        .send(Command::SetRealtime(SetRealtime::Off))
                        .ok();
                    self.eq_profile.remove_hook();
                }
            }
            if !self.state.realtime && ui.button("Apply").clicked() {
                self.eq_settings_back = self.eq_settings.clone();
                self.sender
                    .send(Command::UpdateSettings(self.eq_settings.clone()))
                    .ok();
                self.eq_profile_back = self.eq_profile.clone();
                self.sender
                    .send(Command::UpdateProfile(self.eq_profile.clone()))
                    .ok();
            }
            if ui.button("Add Band").clicked() {
                self.eq_profile.filters.push(crate::eq::Filter::default());
            }
        });
    }
}
