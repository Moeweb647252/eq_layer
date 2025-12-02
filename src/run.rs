use std::{
    sync::{Arc, Mutex, atomic::Ordering, mpsc::Receiver},
    thread::sleep,
    time::Duration,
};

use anyhow::Result;
use cpal::Device;
use ringbuf::{
    HeapRb,
    traits::{Consumer, Producer, Split},
};

use crate::{
    eq::{EqProfile, Equalizer},
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
    let stream_config: StreamConfig = input_device.default_input_config()?.into();

    let mut eq = Equalizer::new(profile.clone(), stream_config.sample_rate.0);
    let latency_frames = (stream_config.sample_rate.0 as u32 * settings.latency) / 1000;
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
    let output_data_fn = move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
        let eq_enabled = settings_cloned.enable_eq.load(Ordering::Relaxed);
        for sample in data {
            *sample = match consumer.try_pop() {
                Some(s) => {
                    if eq_enabled {
                        eq.process_sample(s)
                    } else {
                        s
                    }
                }
                None => 0.0,
            };
        }
    };
    let input_stream =
        input_device.build_input_stream(&stream_config, input_data_fn, err_fn, None)?;
    let output_stream =
        output_device.build_output_stream(&stream_config, output_data_fn, err_fn, None)?;
    input_stream.play()?;
    output_stream.play()?;
    let instance_id = settings.instance_id.load(Ordering::Relaxed);
    loop {
        sleep(Duration::from_millis(settings.latency as u64));
        if instance_id != settings.instance_id.load(Ordering::Relaxed) {
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

    let eq = Equalizer::new(profile.clone(), stream_config.sample_rate.0);
    let latency_frames = (stream_config.sample_rate.0 as u32 * settings.latency) / 1000;
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
        let eq_enabled = settings_cloned.enable_eq.load(Ordering::Relaxed);
        let mut eq = eq_cloned.try_lock();
        for sample in data {
            *sample = match consumer.try_pop() {
                Some(s) => {
                    if eq_enabled && let Ok(eq) = eq.as_mut() {
                        eq.process_sample(s)
                    } else {
                        s
                    }
                }
                None => 0.0,
            };
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
            *eq = Equalizer::new(profile, stream_config.sample_rate.0)
        }
    }
    println!("run_realtime exited");
    Ok(())
}

fn err_fn(err: cpal::StreamError) {
    eprintln!("an error occurred on stream: {err}");
}
