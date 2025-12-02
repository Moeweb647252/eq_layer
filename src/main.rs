use std::sync::Arc;

use cpal::traits::{DeviceTrait, HostTrait};
use eframe::egui;
use font_kit::{family_name::FamilyName, properties::Properties, source::SystemSource};
use settings::Settings;
use ui::App;

use crate::{
    config::{Config, config_dir},
    executor::Executor,
    ui::command::Info,
    utils::OneShot,
};
mod config;
mod eq;
mod executor;
mod run;
mod settings;
mod ui;
mod utils;

fn main() {
    env_logger::init();
    println!("{}", config_dir().to_string_lossy());
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1000.0, 450.0]),
        ..Default::default()
    };
    let (sender, receiver) = std::sync::mpsc::sync_channel(1024);

    let config_dir = dirs::config_dir().unwrap().join("eq_layer");
    std::fs::create_dir_all(&config_dir).unwrap();
    let config_path = config_dir.join("config.toml");
    let config = if config_path.exists()
        && let Ok(config_contents) = std::fs::read_to_string(&config_path)
        && let Ok(config) = toml::from_str(config_contents.as_str())
    {
        config
    } else {
        Config::default()
    };
    let settings = Settings {
        enable_eq: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true)),
        instance_id: std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0)),
        latency: 100,
    };
    let settings_cloned = settings.clone();
    let config_cloned = config.clone();
    std::thread::spawn(move || {
        Executor::new(receiver, config_cloned, settings_cloned).run();
    });
    let oneshot = OneShot::new();
    sender
        .send(ui::command::Command::GetState(oneshot.clone()))
        .unwrap();
    let state = oneshot.recv();
    let dev_names = cpal::Host::default()
        .devices()
        .unwrap()
        .map(|v| v.name())
        .filter_map(|v| v.ok())
        .collect();
    let info = Info {
        device_names: dev_names,
        input_dev: config.input_dev_name.clone().unwrap_or(String::new()),
        output_dev: config.output_dev_name.clone().unwrap_or(String::new()),
    };
    let app = App::new(settings, config.eq_profile, sender, state, info);

    eframe::run_native(
        "Eq Layer",
        options,
        Box::new(|ctx| {
            load_font(&ctx.egui_ctx);
            Ok(Box::new(app))
        }),
    )
    .unwrap();
}

fn load_font(ctx: &egui::Context) {
    let source = SystemSource::new();

    let font_families = [
        FamilyName::Title("Microsoft YaHei".to_string()), // Windows SC
        FamilyName::Title("PingFang SC".to_string()),     // macOS SC
        FamilyName::Title("Noto Sans CJK SC".to_string()), // Linux SC
        FamilyName::Title("Arial".to_string()),           // Common English
        FamilyName::SansSerif,                            // Fallback
    ];

    // 3. 尝试查找字体
    let mut font_data_bytes: Option<Vec<u8>> = None;

    for family in &font_families {
        let properties = Properties::new();

        if let Ok(handle) = source.select_best_match(&[family.clone()], &properties) {
            if let Ok(font) = handle.load() {
                if let Some(data) = font.copy_font_data() {
                    font_data_bytes = Some(data.to_vec());
                    println!("Found font: {:?}", family);
                    break;
                }
            }
        }
    }

    if let Some(font_data) = font_data_bytes {
        let mut fonts = egui::FontDefinitions::default();

        fonts.font_data.insert(
            "system_font".to_owned(),
            Arc::new(egui::FontData::from_owned(font_data)),
        );

        if let Some(family) = fonts.families.get_mut(&egui::FontFamily::Proportional) {
            family.insert(0, "system_font".to_owned());
        }

        if let Some(family) = fonts.families.get_mut(&egui::FontFamily::Monospace) {
            family.push("system_font".to_owned());
        }

        ctx.set_fonts(fonts);
    }
}
