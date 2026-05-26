use std::{
    cmp::Ordering,
    sync::{Arc, Mutex, mpsc::Receiver},
    thread::sleep,
    time::Duration,
};

use anyhow::Result;
use cpal::{
    Device,
    SupportedBufferSize::{Range, Unknown},
};
use ringbuf::{
    HeapRb,
    traits::{Consumer, Producer, Split},
};
use tracing::{debug, error, info};

use crate::{
    eq::{EqProfile, ParametricEq},
    settings::Settings,
};
use cpal::{
    StreamConfig,
    traits::{DeviceTrait, StreamTrait},
};

pub fn run(
    input_device: Device,
    output_device: Device,
    settings: Settings,
    profile: EqProfile,
) -> Result<()> {
    let input_config_range = input_device
        .supported_input_configs()?
        .into_iter()
        .min_by(|a, b| match (a.buffer_size(), b.buffer_size()) {
            (Range { min: a_min, max: _ }, Range { min: b_min, max: _ }) => a_min.cmp(b_min),
            (Range { min: _, max: _ }, Unknown) => Ordering::Less,
            (Unknown, Range { min: _, max: _ }) => Ordering::Greater,
            (Unknown, Unknown) => Ordering::Equal,
        })
        .expect("Can not find supported input config");
    let output_config_range = output_device
        .supported_output_configs()?
        .into_iter()
        .min_by(|a, b| match (a.buffer_size(), b.buffer_size()) {
            (Range { min: a_min, max: _ }, Range { min: b_min, max: _ }) => a_min.cmp(b_min),
            (Range { min: _, max: _ }, Unknown) => Ordering::Less,
            (Unknown, Range { min: _, max: _ }) => Ordering::Greater,
            (Unknown, Unknown) => Ordering::Equal,
        })
        .expect("Can not find supported output config");
    let sample_rate = input_config_range
        .min_sample_rate()
        .max(output_config_range.min_sample_rate());
    let buffer_size = match (
        input_config_range.buffer_size(),
        output_config_range.buffer_size(),
    ) {
        (Range { min: a_min, max: _ }, Range { min: b_min, max: _ }) => {
            cpal::BufferSize::Fixed(*a_min.max(b_min))
        }
        (Range { min: _, max: _ }, Unknown) => cpal::BufferSize::Default,
        (Unknown, Range { min: _, max: _ }) => cpal::BufferSize::Default,
        (Unknown, Unknown) => cpal::BufferSize::Default,
    };
    let channels = input_config_range
        .channels()
        .min(output_config_range.channels());
    let stream_config = StreamConfig {
        channels,
        sample_rate,
        buffer_size,
    };
    info!("Selected stream config: {stream_config:?}");

    let mut eq = ParametricEq::from_profile(&profile, sample_rate as f32);
    let latency_frames = (sample_rate as u32 * settings.latency) / 1000;
    let ring_buffer = HeapRb::<f32>::new(latency_frames as usize * channels as usize * 2); // stereo
    let (mut producer, mut consumer) = ring_buffer.split();

    let input_data_fn = move |data: &[f32], _: &cpal::InputCallbackInfo| {
        producer.push_slice(data);
    };
    let settings_cloned = settings.clone();
    let output_data_fn = move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
        let eq_enabled = settings_cloned
            .enable_eq
            .load(std::sync::atomic::Ordering::Relaxed);
        consumer.pop_slice(data);
        if eq_enabled {
            eq.process_buffer(data);
        }
    };
    let input_stream =
        input_device.build_input_stream(&stream_config, input_data_fn, err_fn, None)?;
    let output_stream =
        output_device.build_output_stream(&stream_config, output_data_fn, err_fn, None)?;
    input_stream.play()?;
    output_stream.play()?;
    let instance_id = settings
        .instance_id
        .load(std::sync::atomic::Ordering::Relaxed);
    loop {
        sleep(Duration::from_millis(settings.latency as u64));
        if instance_id
            != settings
                .instance_id
                .load(std::sync::atomic::Ordering::Relaxed)
        {
            break;
        }
    }
    Ok(())
}

pub fn run_realtime(
    input_device: Device,
    output_device: Device,
    settings: Settings,
    profile: EqProfile,
    receiver: Receiver<EqProfile>,
) -> Result<()> {
    let stream_config: StreamConfig = input_device.default_input_config()?.into();

    let eq = ParametricEq::from_profile(&profile, stream_config.sample_rate as f32);
    let latency_frames = (stream_config.sample_rate as u32 * settings.latency) / 1000;
    let ring_buffer = HeapRb::<f32>::new(latency_frames as usize * 2 * 2); // stereo
    let (mut producer, mut consumer) = ring_buffer.split();
    for _ in 0..latency_frames {
        producer.try_push(0.0).unwrap()
    }

    let input_data_fn = move |data: &[f32], _: &cpal::InputCallbackInfo| {
        for &sample in data {
            producer.try_push(sample).ok();
        }
    };
    let settings_cloned = settings.clone();
    let eq = Arc::new(Mutex::new(eq));
    let eq_cloned = eq.clone();
    let output_data_fn = move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
        let eq_enabled = settings_cloned
            .enable_eq
            .load(std::sync::atomic::Ordering::Relaxed);
        let mut eq = eq_cloned.try_lock();
        consumer.pop_slice(data);
        if eq_enabled && let Ok(eq) = eq.as_mut() {
            eq.process_buffer(data);
        }
    };
    let input_stream =
        input_device.build_input_stream(&stream_config, input_data_fn, err_fn, None)?;
    let output_stream =
        output_device.build_output_stream(&stream_config, output_data_fn, err_fn, None)?;
    input_stream.play()?;
    output_stream.play()?;
    while let Ok(profile) = receiver.recv() {
        if let Ok(mut eq) = eq.try_lock() {
            *eq = ParametricEq::from_profile(&profile, stream_config.sample_rate as f32);
        }
    }
    debug!("run_realtime exited");
    Ok(())
}

fn err_fn(err: cpal::StreamError) {
    error!("an error occurred on stream: {err}");
}
