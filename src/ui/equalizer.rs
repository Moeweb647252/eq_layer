use eframe::egui::{
    ComboBox, DragValue, ScrollArea, Slider, TextWrapMode, Ui, Widget,
    scroll_area::ScrollBarVisibility,
};

use crate::{
    eq::{Filter, FilterType},
    ui::App,
};

fn band_ui(index: usize, band: &mut Filter, ui: &mut Ui, remove: &mut bool) {
    ui.vertical(|ui| {
        ui.label("Type");
        ComboBox::new(format!("FilterType_{}", index), "")
            .selected_text(band.filter_type.to_string())
            .show_ui(ui, |ui| {
                ui.selectable_value(
                    &mut band.filter_type,
                    FilterType::Peaking,
                    FilterType::Peaking.to_string(),
                );
                ui.selectable_value(
                    &mut band.filter_type,
                    FilterType::HighShelf,
                    FilterType::HighShelf.to_string(),
                );
                ui.selectable_value(
                    &mut band.filter_type,
                    FilterType::LowShelf,
                    FilterType::LowShelf.to_string(),
                );
            });
        ui.label("Freq");
        Slider::new(&mut band.frequency, 20.0..=20000.0)
            .vertical()
            .logarithmic(true)
            .suffix(" Hz")
            .show_value(true)
            .ui(ui);
        ui.label("Q");
        DragValue::new(&mut band.q_factor)
            .speed(0.1)
            .range(0.01..=10.0)
            .ui(ui);
        ui.label("Gain");
        DragValue::new(&mut band.gain)
            .speed(0.1)
            .range(-12.0..=12.0)
            .ui(ui);
        ui.checkbox(&mut band.enabled, format!("Band {}", index + 1));
        if ui.button("Remove").clicked() {
            *remove = true;
        }
    });
}

impl App {
    pub fn equalizer_ui(&mut self, ui: &mut Ui) {
        let profile = &mut self.eq_profile;
        ScrollArea::horizontal()
            .scroll_bar_visibility(ScrollBarVisibility::VisibleWhenNeeded)
            .show(ui, |ui| {
                ui.style_mut().wrap_mode = Some(TextWrapMode::Extend);
                ui.horizontal(|ui| {
                    let mut remove_index = None;
                    for (i, band) in profile.filters.iter_mut().enumerate() {
                        let mut remove = false;
                        band_ui(i, band, ui, &mut remove);
                        if remove {
                            remove_index = Some(i);
                        }
                    }
                    if let Some(i) = remove_index {
                        profile.filters.remove(i);
                    }
                });
            });
    }
}
