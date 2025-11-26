use crate::{
    run::run,
    settings::Settings,
    ui::command::{SetDevice, State},
};
use cpal::{
    Device,
    traits::{DeviceTrait, HostTrait},
};

use crate::{config::Config, ui::command::Command};
use std::sync::mpsc::Receiver;
pub fn executor(
    receiver: Receiver<Command>,
    mut config: Config,
    mut settings: crate::settings::Settings,
) {
    let mut input_device = None;
    let mut output_device = None;
    let host = cpal::default_host();
    if let Some(input_device_name) = config.input_dev_name.as_ref() {
        for device in host.devices().unwrap() {
            if device.name().as_ref().unwrap() == input_device_name {
                input_device = Some(device);
                break;
            }
        }
    }
    if let Some(output_device_name) = config.output_dev_name.as_ref() {
        for device in host.devices().unwrap() {
            if device.name().as_ref().unwrap() == output_device_name {
                output_device = Some(device);
                break;
            }
        }
    }
    let mut state = State::default();
    if input_device.is_none() || output_device.is_none() {
        state.running = false;
    } else {
        let input_device = input_device.clone();
        let output_device = output_device.clone();
        start(&state, &settings, &input_device, &output_device);
    }
    while let Ok(command) = receiver.recv() {
        println!("New command: {:?}", command);
        match command {
            Command::SetState(new_state) => {
                if state.running != new_state.running {
                    state.running = new_state.running;
                    if state.running {
                        state.running = new_state.running;
                        start(&state, &settings, &input_device, &output_device);
                    } else {
                        settings
                            .instance_id
                            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    }
                }
                if state.enabled != new_state.enabled {
                    state.enabled = new_state.enabled;
                    settings
                        .enable_eq
                        .store(state.enabled, std::sync::atomic::Ordering::Relaxed);
                }
            }
            Command::UpdateSettings(new_settings) => {
                settings = new_settings.clone();
                config.eq_profile = settings.eq_profile.clone();
                config.latency = settings.latency;
                config.save().unwrap();
                settings
                    .instance_id
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                start(&state, &settings, &input_device, &output_device);
            }
            Command::GetState(oneshot) => {
                oneshot.send(state);
            }
            Command::SetDevice(set_device, name) => {
                let mut dev = None;
                for device in host.devices().unwrap() {
                    if device.name().as_ref().unwrap() == name.as_str() {
                        dev = Some(device);
                        break;
                    }
                }
                match set_device {
                    SetDevice::Input => {
                        input_device = dev;
                        config.input_dev_name = Some(name)
                    }
                    SetDevice::Output => {
                        output_device = dev;
                        config.output_dev_name = Some(name)
                    }
                }
                config.save().unwrap();
            }
        }
    }
}

fn start(
    state: &State,
    settings: &Settings,
    input_device: &Option<Device>,
    output_device: &Option<Device>,
) {
    if state.running && input_device.is_some() && output_device.is_some() {
        let input = input_device.clone().unwrap();
        let output = output_device.clone().unwrap();
        let settings = settings.clone();
        std::thread::spawn(move || {
            run(input, output, settings)
                .inspect_err(|e| println!("{:?}", e))
                .ok();
        });
    }
}
