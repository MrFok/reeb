use crate::{
    mixer::{Mixer, Shared},
    samples::SampleBank,
};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::{Arc, atomic::Ordering};

pub struct Audio {
    _stream: cpal::Stream,
    pub description: String,
    pub device_name: String,
}

pub fn output_devices() -> Result<Vec<String>, String> {
    cpal::default_host()
        .output_devices()
        .map_err(|error| error.to_string())?
        .map(|device| device.name().map_err(|error| error.to_string()))
        .collect()
}

impl Audio {
    pub fn start(
        bank: Arc<SampleBank>,
        shared: Arc<Shared>,
        name: Option<&str>,
    ) -> Result<Self, String> {
        let host = cpal::default_host();
        let device = if let Some(name) = name {
            host.output_devices()
                .map_err(|error| error.to_string())?
                .find(|device| device.name().ok().as_deref() == Some(name))
                .ok_or_else(|| format!("Output device is unavailable: {name}"))?
        } else {
            host.default_output_device()
                .ok_or("No audio output device found")?
        };
        let supported = device
            .default_output_config()
            .map_err(|error| error.to_string())?;
        let format = supported.sample_format();
        let mut config: cpal::StreamConfig = supported.into();
        // Request a small callback only when the device explicitly advertises it.
        if let Ok(configs) = device.supported_output_configs() {
            for range in configs {
                if range.channels() == config.channels
                    && range.sample_format() == format
                    && range.min_sample_rate() <= config.sample_rate
                    && range.max_sample_rate() >= config.sample_rate
                {
                    if let cpal::SupportedBufferSize::Range { min, max } = *range.buffer_size() {
                        config.buffer_size = cpal::BufferSize::Fixed(128.clamp(min, max));
                    }
                    break;
                }
            }
        }
        if config.channels == 0 || config.sample_rate.0 == 0 {
            return Err("The output device returned an invalid audio format".to_owned());
        }
        let build = |config: &cpal::StreamConfig| match format {
            cpal::SampleFormat::F32 => {
                build_stream::<f32>(&device, config, bank.clone(), shared.clone())
            }
            cpal::SampleFormat::F64 => {
                build_stream::<f64>(&device, config, bank.clone(), shared.clone())
            }
            cpal::SampleFormat::I16 => {
                build_stream::<i16>(&device, config, bank.clone(), shared.clone())
            }
            cpal::SampleFormat::I32 => {
                build_stream::<i32>(&device, config, bank.clone(), shared.clone())
            }
            cpal::SampleFormat::U16 => {
                build_stream::<u16>(&device, config, bank.clone(), shared.clone())
            }
            other => Err(format!("Unsupported audio sample format: {other}")),
        };
        let stream = match build(&config) {
            Ok(stream) => stream,
            Err(_) if config.buffer_size != cpal::BufferSize::Default => {
                config.buffer_size = cpal::BufferSize::Default;
                build(&config)?
            }
            Err(error) => return Err(error),
        };
        shared.stream_failed.store(false, Ordering::Relaxed);
        stream.play().map_err(|error| error.to_string())?;
        let device_name = device.name().unwrap_or_else(|_| "Output".to_owned());
        Ok(Self {
            _stream: stream,
            description: format!(
                "{} · {} Hz · {} ch",
                device_name, config.sample_rate.0, config.channels
            ),
            device_name,
        })
    }
}

fn build_stream<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    bank: Arc<SampleBank>,
    shared: Arc<Shared>,
) -> Result<cpal::Stream, String>
where
    T: cpal::SizedSample + cpal::FromSample<f32>,
{
    let mut mixer = Mixer::new(bank, shared.clone(), config.sample_rate.0);
    let channels = usize::from(config.channels);
    device
        .build_output_stream(
            config,
            move |output: &mut [T], _| mixer.render(output, channels),
            move |_| {
                shared.stream_failed.store(true, Ordering::Relaxed);
            },
            None,
        )
        .map_err(|error| error.to_string())
}
