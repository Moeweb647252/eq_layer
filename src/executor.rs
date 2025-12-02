use crate::{
    eq::EqProfile,
    run::{run, run_realtime},
    settings::Settings,
    ui::command::{SetDevice, SetRealtime, State},
};
use cpal::{
    Device,
    traits::{DeviceTrait, HostTrait},
};

use crate::{config::Config, ui::command::Command};
use std::sync::{atomic::Ordering, mpsc::Receiver};

pub struct Executor {
    receiver: Receiver<Command>,
    config: Config,
    settings: Settings,
    input_device: Option<Device>,
    output_device: Option<Device>,
    state: State,
}

impl Executor {
    pub fn new(receiver: Receiver<Command>, config: Config, settings: Settings) -> Self {
        Executor {
            receiver,
            config,
            settings,
            input_device: None,
            output_device: None,
            state: State::default(),
        }
    }

    pub fn run(&mut self) {
        let host = cpal::default_host();
        if let Some(input_device_name) = self.config.input_dev_name.as_ref() {
            for device in host.devices().unwrap() {
                if device.name().as_ref().unwrap() == input_device_name {
                    self.input_device = Some(device);
                    break;
                }
            }
        }
        if let Some(output_device_name) = self.config.output_dev_name.as_ref() {
            for device in host.devices().unwrap() {
                if device.name().as_ref().unwrap() == output_device_name {
                    self.output_device = Some(device);
                    break;
                }
            }
        }
        if self.input_device.is_none() || self.output_device.is_none() {
            self.state.running = false;
        } else {
            self.start_proc();
        }
        while let Ok(command) = self.receiver.recv() {
            println!("New command: {:?}", command);
            match command {
                Command::SetState(new_state) => {
                    if self.state.running != new_state.running {
                        self.state.running = new_state.running;
                        if self.state.running {
                            self.state.running = new_state.running;
                            self.start_proc();
                        } else {
                            self.settings.instance_id.fetch_add(1, Ordering::Relaxed);
                        }
                    }
                    if self.state.enabled != new_state.enabled {
                        self.state.enabled = new_state.enabled;
                        self.settings
                            .enable_eq
                            .store(self.state.enabled, Ordering::Relaxed);
                    }
                }
                Command::UpdateSettings(new_settings) => {
                    self.settings = new_settings.clone();
                    self.config.latency = self.settings.latency;
                    self.settings.instance_id.fetch_add(1, Ordering::Relaxed);
                    self.start_proc();
                }
                Command::UpdateProfile(new_profile) => {
                    self.config.eq_profile = new_profile;
                    self.settings.instance_id.fetch_add(1, Ordering::Relaxed);
                    self.start_proc();
                }
                Command::Save(settings, profile) => {
                    self.settings = settings.clone();
                    self.config.latency = self.settings.latency;
                    self.config.eq_profile = profile;
                    self.config.save().unwrap();
                }
                Command::GetState(oneshot) => {
                    oneshot.send(self.state);
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
                            self.input_device = dev;
                            self.config.input_dev_name = Some(name)
                        }
                        SetDevice::Output => {
                            self.output_device = dev;
                            self.config.output_dev_name = Some(name)
                        }
                    }
                    self.config.save().unwrap();
                }
                Command::SetRealtime(set_realtime) => match set_realtime {
                    SetRealtime::Off => {
                        self.state.realtime = false;
                        if self.state.enabled {
                            self.start_proc();
                        }
                    }
                    SetRealtime::On(receiver) => {
                        self.state.realtime = true;
                        self.settings.instance_id.fetch_add(1, Ordering::Relaxed);
                        self.start_proc_realtime(receiver);
                    }
                },
            }
        }
    }

    fn start_proc(&self) {
        if self.state.running
            && let Some(input) = self.input_device.clone()
            && let Some(output) = self.output_device.clone()
        {
            let settings = self.settings.clone();
            let profile = self.config.eq_profile.clone();
            std::thread::spawn(move || {
                run(input, output, settings, profile)
                    .inspect_err(|e| println!("{:?}", e))
                    .ok();
            });
        }
    }

    fn start_proc_realtime(&self, receiver: Receiver<EqProfile>) {
        if self.state.running
            && let Some(input) = self.input_device.clone()
            && let Some(output) = self.output_device.clone()
        {
            let settings = self.settings.clone();
            let profile = self.config.eq_profile.clone();
            std::thread::spawn(move || {
                run_realtime(input, output, settings, profile, receiver)
                    .inspect_err(|e| println!("{:?}", e))
                    .ok();
            });
        }
    }
}
