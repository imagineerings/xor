use std::time::Duration;

use anyhow::{Context as _, Result, bail};
use cpal::{
    DeviceId, FromSample, SampleFormat, SizedSample, SupportedStreamConfig,
    SupportedStreamConfigRange,
    traits::{DeviceTrait, HostTrait as _, StreamTrait as _},
};
use crossbeam::channel;
use parking_lot::Mutex;
use std::sync::Arc;
use util::ResultExt;

use crate::{AudioDeviceInfo, resolve_device};

pub use cpal::SampleFormat as CaptureSampleFormat;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CaptureConfig {
    pub sample_rate: Option<u32>,
    pub channels: Option<u16>,
    pub sample_format: Option<CaptureSampleFormat>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActiveCaptureConfig {
    pub sample_rate: u32,
    pub channels: u16,
    pub sample_format: CaptureSampleFormat,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CapturedAudio {
    pub samples: Vec<i16>,
    pub config: ActiveCaptureConfig,
}

#[derive(Debug)]
pub struct MicrophoneCapture;

impl MicrophoneCapture {
    pub fn list_devices() -> Result<Vec<AudioDeviceInfo>> {
        let devices = cpal::default_host()
            .input_devices()
            .context("failed to enumerate audio input devices")?;
        Ok(devices
            .filter_map(|device| {
                let id = device.id().log_err()?;
                let desc = device.description().log_err()?;
                Some(AudioDeviceInfo { id, desc })
            })
            .collect())
    }

    pub fn start_capture(
        device_id: Option<&DeviceId>,
        config: CaptureConfig,
    ) -> Result<CaptureStream> {
        let device = resolve_device(device_id, true)?;
        let device_name = device
            .description()
            .map(|description| description.name().to_string())
            .unwrap_or_else(|_| "<unknown>".to_string());
        let supported_config = select_config(&device, &config)?;
        let active_config = ActiveCaptureConfig {
            sample_rate: supported_config.sample_rate(),
            channels: supported_config.channels(),
            sample_format: supported_config.sample_format(),
        };
        let (samples_tx, samples_rx) = channel::unbounded();
        let stream_error = Arc::new(Mutex::new(None));
        let stream_error_callback = stream_error.clone();
        let sample_format = supported_config.sample_format();
        let stream = device
            .build_input_stream_raw(
                &supported_config.config(),
                sample_format,
                move |data, _| {
                    let samples = sample_data_to_i16(sample_format, data).log_err();
                    let Some(samples) = samples else {
                        return;
                    };
                    samples_tx.send(samples).log_err();
                },
                move |error| {
                    log::error!("error capturing microphone input: {error:?}");
                    *stream_error_callback.lock() = Some(error.to_string());
                },
                Some(Duration::from_millis(100)),
            )
            .with_context(|| format!("failed to build input stream for {device_name}"))?;

        stream
            .play()
            .with_context(|| format!("failed to start input stream for {device_name}"))?;
        log::info!(
            "Opened microphone capture: {device_name} {:?}",
            supported_config.config()
        );

        Ok(CaptureStream {
            stream,
            samples_rx,
            active_config,
            stream_error,
        })
    }
}

pub struct CaptureStream {
    stream: cpal::Stream,
    samples_rx: channel::Receiver<Vec<i16>>,
    active_config: ActiveCaptureConfig,
    stream_error: Arc<Mutex<Option<String>>>,
}

impl CaptureStream {
    pub fn config(&self) -> &ActiveCaptureConfig {
        &self.active_config
    }

    pub fn read_available(&self) -> Result<CapturedAudio> {
        self.check_stream_error()?;
        let samples = self.drain_samples();
        Ok(CapturedAudio {
            samples,
            config: self.active_config.clone(),
        })
    }

    pub fn stop_capture(self) -> Result<CapturedAudio> {
        let Self {
            stream,
            samples_rx,
            active_config,
            stream_error,
        } = self;
        drop(stream);
        if let Some(error) = stream_error.lock().take() {
            bail!("microphone capture stream failed: {error}");
        }
        let samples = drain_receiver(&samples_rx);
        Ok(CapturedAudio {
            samples,
            config: active_config,
        })
    }

    fn check_stream_error(&self) -> Result<()> {
        if let Some(error) = self.stream_error.lock().take() {
            bail!("microphone capture stream failed: {error}");
        }
        Ok(())
    }

    fn drain_samples(&self) -> Vec<i16> {
        drain_receiver(&self.samples_rx)
    }
}

fn select_config(device: &cpal::Device, config: &CaptureConfig) -> Result<SupportedStreamConfig> {
    if config.sample_rate.is_none() && config.channels.is_none() && config.sample_format.is_none() {
        return device
            .default_input_config()
            .context("failed to get default input config");
    }

    let mut supported_configs = device
        .supported_input_configs()
        .context("failed to get supported input configs")?;
    let supported_config = supported_configs
        .find(|supported_config| config_matches(supported_config, config))
        .with_context(|| format!("no supported input config matches {config:?}"))?;
    let sample_rate = config
        .sample_rate
        .unwrap_or_else(|| supported_config.max_sample_rate());
    Ok(supported_config.with_sample_rate(sample_rate))
}

fn config_matches(supported_config: &SupportedStreamConfigRange, config: &CaptureConfig) -> bool {
    if let Some(sample_format) = config.sample_format
        && supported_config.sample_format() != sample_format
    {
        return false;
    }
    if let Some(channels) = config.channels
        && supported_config.channels() != channels
    {
        return false;
    }
    if let Some(sample_rate) = config.sample_rate
        && (sample_rate < supported_config.min_sample_rate()
            || sample_rate > supported_config.max_sample_rate())
    {
        return false;
    }
    true
}

fn sample_data_to_i16(sample_format: SampleFormat, data: &cpal::Data) -> Result<Vec<i16>> {
    match sample_format {
        SampleFormat::I8 => convert_sample_data::<i8, i16>(data),
        SampleFormat::I16 => data
            .as_slice::<i16>()
            .map(|samples| samples.to_vec())
            .context("input audio data did not contain i16 samples"),
        SampleFormat::I24 => convert_sample_data::<cpal::I24, i16>(data),
        SampleFormat::I32 => convert_sample_data::<i32, i16>(data),
        SampleFormat::I64 => convert_sample_data::<i64, i16>(data),
        SampleFormat::U8 => convert_sample_data::<u8, i16>(data),
        SampleFormat::U16 => convert_sample_data::<u16, i16>(data),
        SampleFormat::U32 => convert_sample_data::<u32, i16>(data),
        SampleFormat::U64 => convert_sample_data::<u64, i16>(data),
        SampleFormat::F32 => convert_sample_data::<f32, i16>(data),
        SampleFormat::F64 => convert_sample_data::<f64, i16>(data),
        _ => bail!("unsupported input sample format: {sample_format:?}"),
    }
}

fn convert_sample_data<TSource, TDestination>(data: &cpal::Data) -> Result<Vec<TDestination>>
where
    TSource: SizedSample,
    TDestination: SizedSample + FromSample<TSource>,
{
    let samples = data
        .as_slice::<TSource>()
        .context("input audio data did not match the declared sample format")?;
    Ok(samples
        .iter()
        .map(|sample| sample.to_sample::<TDestination>())
        .collect())
}

fn drain_receiver(samples_rx: &channel::Receiver<Vec<i16>>) -> Vec<i16> {
    let mut samples = Vec::new();
    while let Ok(mut chunk) = samples_rx.try_recv() {
        samples.append(&mut chunk);
    }
    samples
}
