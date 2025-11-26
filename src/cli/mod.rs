use std::{
    cell::RefCell,
    io::Write,
    str::FromStr,
    sync::{Arc, atomic::AtomicBool},
    thread,
};

use cpal::{
    Host,
    traits::{DeviceTrait, HostTrait},
};

use clap::Parser;

use crate::{eq, run, settings};

#[derive(Debug, Parser)]
struct Args {
    #[clap(long, short)]
    pub input_device: Option<String>,
    #[clap(long, short)]
    pub output_device: Option<String>,
    #[clap(long, short)]
    pub list: bool,
    #[clap(long, short = 'L', default_value_t = 100)]
    pub latency: u32,
    #[clap(long, short)]
    pub eq_file: Option<String>,
}

fn cli_main() {
    let mut args = Args::parse();
    let host = cpal::default_host();
    if args.list {
        list_devices(&host);
        return;
    }
    let devices = host.devices().unwrap();
    let mut input_device = None;
    let mut output_device = None;

    let mut eq_profile = if let Some(eq_file) = args.eq_file.as_ref() {
        let eq_contents = std::fs::read_to_string(eq_file).expect("Failed to read EQ file");
        eq::EqProfile::from_str(&eq_contents).expect("Failed to parse EQ profile")
    } else {
        Default::default()
    };

    for device in devices {
        if device.name().as_ref().unwrap()
            == args
                .input_device
                .as_ref()
                .expect("Input device not specified")
        {
            input_device = Some(device);
            continue;
        }
        if device.name().as_ref().unwrap()
            == args
                .output_device
                .as_ref()
                .expect("Output device not specified")
        {
            output_device = Some(device);
        }
    }
    let input_device = input_device.expect("Input device not found");
    let output_device = output_device.expect("Output device not found");
    let mut settings = settings::Settings {
        latency: args.latency,
        enable_eq: Arc::new(AtomicBool::new(true)),
        instance_id: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
        eq_profile: eq_profile,
    };

    let runner_handle = Arc::new(RefCell::new(thread::spawn(|| {})));

    let start = |settings| {
        let input_device_cloned = input_device.clone();
        let output_device_cloned = output_device.clone();
        runner_handle.replace(thread::spawn(|| {
            run::run(input_device_cloned, output_device_cloned, settings)
                .expect("Failed to run audio processing");
        }));
    };
    start(settings.clone());

    let running = true;
    let mut command = String::new();
    let mut stdout = std::io::stdout();
    let stdin = std::io::stdin();
    loop {
        print!(">>>");
        stdout.flush().unwrap();
        command.clear();
        stdin.read_line(&mut command).unwrap();
        println!("{}", command);
        match command.trim() {
            "quit" | "q" => {
                println!("Quitting...");
                break;
            }
            "stop" => {
                if running {
                    settings
                        .instance_id
                        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    println!("Stopped.");
                } else {
                    println!("Already stoppped.");
                }
            }
            "start" => {
                if !running || runner_handle.borrow().is_finished() {
                    start(settings.clone());
                    println!("Started.");
                } else {
                    println!("Already started.");
                }
            }
            "reload" => {
                settings
                    .instance_id
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                println!("Reloading EQ profile...");
                eq_profile = if let Some(eq_file) = args.eq_file.as_ref() {
                    let eq_contents =
                        std::fs::read_to_string(eq_file).expect("Failed to read EQ file");
                    eq::EqProfile::from_str(&eq_contents).expect("Failed to parse EQ profile")
                } else {
                    Default::default()
                };
                settings.eq_profile = eq_profile;
                start(settings.clone());
            }
            "disable" | "d" => {
                let currently_enabled = settings
                    .enable_eq
                    .load(std::sync::atomic::Ordering::Relaxed);
                if currently_enabled {
                    settings
                        .enable_eq
                        .store(false, std::sync::atomic::Ordering::Relaxed);
                    println!("EQ disabled.");
                } else {
                    println!("EQ is already disabled.");
                }
            }
            "enable" | "e" => {
                let currently_enabled = settings
                    .enable_eq
                    .load(std::sync::atomic::Ordering::Relaxed);
                if !currently_enabled {
                    settings
                        .enable_eq
                        .store(true, std::sync::atomic::Ordering::Relaxed);
                    println!("EQ enabled.");
                } else {
                    println!("EQ is already enabled.");
                }
            }
            x if x.starts_with("load") => {
                if let Some(space_index) = x.find(' ') {
                    let eq_file = x[space_index + 1..].to_string().replace("\\ ", " ");
                    println!("Loading EQ profile from {}...", eq_file);
                    let eq_contents =
                        std::fs::read_to_string(eq_file.as_str()).expect("Failed to read EQ file");
                    eq_profile =
                        eq::EqProfile::from_str(&eq_contents).expect("Failed to parse EQ profile");
                    settings.eq_profile = eq_profile;
                    args.eq_file = Some(eq_file.to_string());
                    settings
                        .instance_id
                        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    start(settings.clone());
                } else {
                    println!("Usage: load <eq_file>");
                    continue;
                }
            }
            _ => {
                println!("Unknown command. Use 'e' to toggle EQ, 'q' to quit.");
            }
        }
    }
}

pub fn list_devices(host: &Host) {
    let devices = host.devices().unwrap();
    for device in devices {
        println!("{}", device.name().unwrap());
    }
}
