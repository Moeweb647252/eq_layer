use std::sync::mpsc::Receiver;

use crate::{eq::EqProfile, settings::Settings, utils::OneShot};

#[derive(Clone, Copy, Debug)]
pub struct State {
    pub enabled: bool,
    pub running: bool,
    pub realtime: bool,
}

impl Default for State {
    fn default() -> Self {
        Self {
            enabled: true,
            running: true,
            realtime: false,
        }
    }
}

pub struct Info {
    pub device_names: Vec<String>,
    pub input_dev: String,
    pub output_dev: String,
}

#[derive(Debug)]
pub enum SetDevice {
    Input,
    Output,
}

#[derive(Debug)]
pub enum SetRealtime {
    Off,
    On(Receiver<EqProfile>),
}

#[derive(Debug)]
pub enum Command {
    SetState(State),
    UpdateSettings(Settings),
    UpdateProfile(EqProfile),
    Save(Settings, EqProfile),
    GetState(OneShot<State>),
    SetDevice(SetDevice, String),
    SetRealtime(SetRealtime),
    Restart,
}
