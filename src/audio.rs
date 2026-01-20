use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};

use crossbeam_channel::{Receiver, Sender};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

use crate::game::state::SoundEvent;

#[derive(Clone)]
pub struct AudioEngine {
    sender: Sender<SoundEvent>,
    _stream: Arc<cpal::Stream>,
}

const MASTER_GAIN: f32 = 0.6;

impl AudioEngine {
    pub fn new(asset_dir: &Path) -> anyhow::Result<Self> {
        let (tx, rx) = crossbeam_channel::unbounded();
        let assets = load_assets(asset_dir)?;

        let stream = build_output_stream(rx, assets)?;
        stream.play()?;

        Ok(Self {
            sender: tx,
            _stream: Arc::new(stream),
        })
    }

    pub fn play(&self, event: SoundEvent) {
        let _ = self.sender.send(event);
    }
}

#[derive(Clone)]
struct SoundAsset {
    samples: Arc<Vec<f32>>,
    channels: u16,
    sample_rate: u32,
}

#[derive(Clone)]
struct Voice {
    samples: Arc<Vec<f32>>,
    channels: u16,
    position: f32,
    step: f32,
    gain: f32,
}

fn load_assets(asset_dir: &Path) -> anyhow::Result<HashMap<&'static str, SoundAsset>> {
    let mut assets = HashMap::new();

    for key in [
        "move",
        "rotate",
        "soft_drop",
        "hard_drop",
        "hold",
        "line_clear_1",
        "line_clear_2",
        "line_clear_3",
        "line_clear_4",
        "game_over",
    ] {
        let path = asset_dir.join(format!("{key}.wav"));
        if let Ok(asset) = load_wav(&path) {
            assets.insert(key, asset);
        }
    }

    Ok(assets)
}

fn build_output_stream(
    rx: Receiver<SoundEvent>,
    assets: HashMap<&'static str, SoundAsset>,
) -> anyhow::Result<cpal::Stream> {
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .ok_or_else(|| anyhow::anyhow!("no output device available"))?;

    let config = select_output_config(&device)?;
    let channels = config.channels() as usize;
    let sample_rate = config.sample_rate().0;

    let assets = Arc::new(assets);
    let voices = Arc::new(Mutex::new(Vec::<Voice>::new()));

    let stream = match config.sample_format() {
        cpal::SampleFormat::F32 => device.build_output_stream(
            &config.into(),
            {
                let assets = assets.clone();
                let voices = voices.clone();
                move |data: &mut [f32], _| {
                    render_audio(data, channels, sample_rate, &rx, &assets, &voices);
                }
            },
            move |err| {
                eprintln!("audio stream error: {err}");
            },
            None,
        )?,
        cpal::SampleFormat::I16 => device.build_output_stream(
            &config.into(),
            {
                let assets = assets.clone();
                let voices = voices.clone();
                move |data: &mut [i16], _| {
                    let mut buffer = vec![0.0f32; data.len()];
                    render_audio(&mut buffer, channels, sample_rate, &rx, &assets, &voices);
                    for (dst, sample) in data.iter_mut().zip(buffer.iter()) {
                        *dst = <i16 as cpal::Sample>::from_sample(*sample);
                    }
                }
            },
            move |err| {
                eprintln!("audio stream error: {err}");
            },
            None,
        )?,
        cpal::SampleFormat::U16 => device.build_output_stream(
            &config.into(),
            {
                let assets = assets.clone();
                let voices = voices.clone();
                move |data: &mut [u16], _| {
                    let mut buffer = vec![0.0f32; data.len()];
                    render_audio(&mut buffer, channels, sample_rate, &rx, &assets, &voices);
                    for (dst, sample) in data.iter_mut().zip(buffer.iter()) {
                        *dst = <u16 as cpal::Sample>::from_sample(*sample);
                    }
                }
            },
            move |err| {
                eprintln!("audio stream error: {err}");
            },
            None,
        )?,
        _ => {
            return Err(anyhow::anyhow!(
                "unsupported sample format: {:?}",
                config.sample_format()
            ));
        }
    };

    Ok(stream)
}

fn select_output_config(device: &cpal::Device) -> anyhow::Result<cpal::SupportedStreamConfig> {
    let mut candidates = device.supported_output_configs()?;
    let mut selected = None;

    while let Some(config) = candidates.next() {
        if config.channels() == 2 {
            let min = config.min_sample_rate().0;
            let max = config.max_sample_rate().0;
            if min <= 44_100 && max >= 44_100 {
                selected = Some(config.with_sample_rate(cpal::SampleRate(44_100)));
                break;
            }
        }
    }

    Ok(selected.unwrap_or_else(|| device.default_output_config().unwrap()))
}

fn render_audio(
    output: &mut [f32],
    channels: usize,
    device_rate: u32,
    rx: &Receiver<SoundEvent>,
    assets: &Arc<HashMap<&'static str, SoundAsset>>,
    voices: &Arc<Mutex<Vec<Voice>>>,
) {
    for event in rx.try_iter() {
        if let Some(asset_key) = sound_event_to_asset(&event) {
            if let Some(asset) = assets.get(asset_key) {
                let step = asset.sample_rate as f32 / device_rate as f32;
                let voice = Voice {
                    samples: asset.samples.clone(),
                    channels: asset.channels,
                    position: 0.0,
                    step,
                    gain: sound_event_gain(&event) * MASTER_GAIN,
                };
                if let Ok(mut guard) = voices.lock() {
                    guard.push(voice);
                }
            }
        }
    }

    for sample in output.iter_mut() {
        *sample = 0.0;
    }

    let mut dead = Vec::new();
    if let Ok(mut guard) = voices.lock() {
        for (index, voice) in guard.iter_mut().enumerate() {
            for frame in output.chunks_mut(channels) {
                if voice.position as usize >= voice.samples.len() / voice.channels as usize {
                    dead.push(index);
                    break;
                }

                let frame_index = voice.position as usize;
                let base = frame_index * voice.channels as usize;
                let left = voice.samples.get(base).copied().unwrap_or(0.0) * voice.gain;
                let right = if voice.channels > 1 {
                    voice.samples.get(base + 1).copied().unwrap_or(0.0) * voice.gain
                } else {
                    left
                };

                if channels == 1 {
                    frame[0] += (left + right) * 0.5;
                } else {
                    frame[0] += left;
                    frame[1] += right;
                }

                voice.position += voice.step;
            }
        }

        for &index in dead.iter().rev() {
            guard.swap_remove(index);
        }
    }

    for sample in output.iter_mut() {
        *sample = soft_clip(*sample).clamp(-1.0, 1.0);
    }
}

fn load_wav(path: &Path) -> anyhow::Result<SoundAsset> {
    let mut reader = hound::WavReader::open(path)?;
    let spec = reader.spec();
    let samples: Vec<f32> = reader
        .samples::<i16>()
        .filter_map(Result::ok)
        .map(|s| s as f32 / i16::MAX as f32)
        .collect();

    Ok(SoundAsset {
        samples: Arc::new(samples),
        channels: spec.channels,
        sample_rate: spec.sample_rate,
    })
}

pub fn sound_event_to_asset(event: &SoundEvent) -> Option<&'static str> {
    match event {
        SoundEvent::Move => Some("move"),
        SoundEvent::Rotate => Some("rotate"),
        SoundEvent::SoftDrop => Some("soft_drop"),
        SoundEvent::HardDrop => Some("hard_drop"),
        SoundEvent::Hold => Some("hold"),
        SoundEvent::LineClear(1) => Some("line_clear_1"),
        SoundEvent::LineClear(2) => Some("line_clear_2"),
        SoundEvent::LineClear(3) => Some("line_clear_3"),
        SoundEvent::LineClear(_) => Some("line_clear_4"),
        SoundEvent::GameOver => Some("game_over"),
    }
}

pub fn sound_event_gain(event: &SoundEvent) -> f32 {
    match event {
        SoundEvent::Move => 0.25,
        SoundEvent::Rotate => 0.35,
        SoundEvent::SoftDrop => 0.2,
        SoundEvent::HardDrop => 0.6,
        SoundEvent::Hold => 0.5,
        SoundEvent::LineClear(1) => 0.6,
        SoundEvent::LineClear(2) => 0.7,
        SoundEvent::LineClear(3) => 0.8,
        SoundEvent::LineClear(_) => 0.9,
        SoundEvent::GameOver => 0.8,
    }
}

fn soft_clip(sample: f32) -> f32 {
    sample / (1.0 + sample.abs())
}
