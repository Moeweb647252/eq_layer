use cpal::traits::{DeviceTrait, HostTrait};
use eframe::egui;
use settings::Settings;
use ui::App;

use crate::{
    config::{Config, config_dir},
    executor::executor,
    ui::command::{Info, oneshot::OneShot},
};
mod config;
mod eq;
mod executor;
mod run;
mod settings;
mod ui;

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
        eq_profile: config.eq_profile.clone(),
        instance_id: std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0)),
        latency: 100,
    };
    let settings_cloned = settings.clone();
    let config_cloned = config.clone();
    std::thread::spawn(move || {
        executor(receiver, config_cloned, settings_cloned);
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
    let app = App::new(settings, sender, state, info);

    eframe::run_native("Eq Layer", options, Box::new(|_| Ok(Box::new(app)))).unwrap();
}
