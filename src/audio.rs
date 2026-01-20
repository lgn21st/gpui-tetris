use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use crossbeam_channel::{Receiver, Sender};

use crate::game::state::SoundEvent;

#[derive(Clone)]
pub struct AudioEngine {
    sender: Sender<SoundEvent>,
    _stream: Arc<cpal::Stream>,
    master_gain: Arc<AtomicU32>,
}

const DEFAULT_MASTER_GAIN: f32 = 0.6;
const MAX_VOICES: usize = 16;

impl AudioEngine {
    pub fn new(asset_dir: &Path) -> anyhow::Result<Self> {
        let (tx, rx) = crossbeam_channel::unbounded();
        let assets = load_assets(asset_dir)?;
        let master_gain = Arc::new(AtomicU32::new(f32_to_bits(DEFAULT_MASTER_GAIN)));

        let stream = build_output_stream(rx, assets, master_gain.clone())?;
        stream.play()?;

        Ok(Self {
            sender: tx,
            _stream: Arc::new(stream),
            master_gain,
        })
    }

    pub fn play(&self, event: SoundEvent) {
        let _ = self.sender.send(event);
    }

    pub fn set_master_gain(&self, gain: f32) {
        let clamped = gain.clamp(0.0, 1.0);
        self.master_gain
            .store(f32_to_bits(clamped), Ordering::Relaxed);
    }

    pub fn master_gain(&self) -> f32 {
        bits_to_f32(self.master_gain.load(Ordering::Relaxed))
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
    master_gain: Arc<AtomicU32>,
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
        cpal::SampleFormat::F32 => build_output_stream_f32(
            &device,
            &config,
            channels,
            sample_rate,
            rx,
            &assets,
            &voices,
            &master_gain,
        )?,
        cpal::SampleFormat::I16 => build_output_stream_i16(
            &device,
            &config,
            channels,
            sample_rate,
            rx,
            &assets,
            &voices,
            &master_gain,
        )?,
        cpal::SampleFormat::U16 => build_output_stream_u16(
            &device,
            &config,
            channels,
            sample_rate,
            rx,
            &assets,
            &voices,
            &master_gain,
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

fn build_output_stream_f32(
    device: &cpal::Device,
    config: &cpal::SupportedStreamConfig,
    channels: usize,
    sample_rate: u32,
    rx: Receiver<SoundEvent>,
    assets: &Arc<HashMap<&'static str, SoundAsset>>,
    voices: &Arc<Mutex<Vec<Voice>>>,
    master_gain: &Arc<AtomicU32>,
) -> anyhow::Result<cpal::Stream> {
    let assets = assets.clone();
    let voices = voices.clone();
    let master_gain = master_gain.clone();
    device
        .build_output_stream(
            &config.clone().into(),
            move |data: &mut [f32], _| {
                let gain = bits_to_f32(master_gain.load(Ordering::Relaxed));
                render_audio(data, channels, sample_rate, &rx, &assets, &voices, gain);
            },
            move |err| {
                eprintln!("audio stream error: {err}");
            },
            None,
        )
        .map_err(Into::into)
}

fn build_output_stream_i16(
    device: &cpal::Device,
    config: &cpal::SupportedStreamConfig,
    channels: usize,
    sample_rate: u32,
    rx: Receiver<SoundEvent>,
    assets: &Arc<HashMap<&'static str, SoundAsset>>,
    voices: &Arc<Mutex<Vec<Voice>>>,
    master_gain: &Arc<AtomicU32>,
) -> anyhow::Result<cpal::Stream> {
    let assets = assets.clone();
    let voices = voices.clone();
    let master_gain = master_gain.clone();
    let mut scratch: Vec<f32> = Vec::new();
    device
        .build_output_stream(
            &config.clone().into(),
            move |data: &mut [i16], _| {
                if scratch.len() != data.len() {
                    scratch.resize(data.len(), 0.0);
                }
                let gain = bits_to_f32(master_gain.load(Ordering::Relaxed));
                render_audio(
                    &mut scratch,
                    channels,
                    sample_rate,
                    &rx,
                    &assets,
                    &voices,
                    gain,
                );
                for (dst, sample) in data.iter_mut().zip(scratch.iter()) {
                    *dst = <i16 as cpal::Sample>::from_sample(*sample);
                }
            },
            move |err| {
                eprintln!("audio stream error: {err}");
            },
            None,
        )
        .map_err(Into::into)
}

fn build_output_stream_u16(
    device: &cpal::Device,
    config: &cpal::SupportedStreamConfig,
    channels: usize,
    sample_rate: u32,
    rx: Receiver<SoundEvent>,
    assets: &Arc<HashMap<&'static str, SoundAsset>>,
    voices: &Arc<Mutex<Vec<Voice>>>,
    master_gain: &Arc<AtomicU32>,
) -> anyhow::Result<cpal::Stream> {
    let assets = assets.clone();
    let voices = voices.clone();
    let master_gain = master_gain.clone();
    let mut scratch: Vec<f32> = Vec::new();
    device
        .build_output_stream(
            &config.clone().into(),
            move |data: &mut [u16], _| {
                if scratch.len() != data.len() {
                    scratch.resize(data.len(), 0.0);
                }
                let gain = bits_to_f32(master_gain.load(Ordering::Relaxed));
                render_audio(
                    &mut scratch,
                    channels,
                    sample_rate,
                    &rx,
                    &assets,
                    &voices,
                    gain,
                );
                for (dst, sample) in data.iter_mut().zip(scratch.iter()) {
                    *dst = <u16 as cpal::Sample>::from_sample(*sample);
                }
            },
            move |err| {
                eprintln!("audio stream error: {err}");
            },
            None,
        )
        .map_err(Into::into)
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
    master_gain: f32,
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
                    gain: sound_event_gain(&event),
                };
                if let Ok(mut guard) = voices.lock() {
                    push_voice(&mut guard, voice);
                }
            }
        }
    }

    for sample in output.iter_mut() {
        *sample = 0.0;
    }

    let mut dead = [0usize; MAX_VOICES];
    let mut dead_len = 0usize;
    if let Ok(mut guard) = voices.lock() {
        for (index, voice) in guard.iter_mut().enumerate() {
            for frame in output.chunks_mut(channels) {
                if voice.position as usize >= voice.samples.len() / voice.channels as usize {
                    if dead_len < dead.len() {
                        dead[dead_len] = index;
                        dead_len += 1;
                    }
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

        for slot in (0..dead_len).rev() {
            let index = dead[slot];
            guard.swap_remove(index);
        }
    }

    let master = master_gain.clamp(0.0, 1.0);
    for sample in output.iter_mut() {
        *sample = soft_clip(*sample * master).clamp(-1.0, 1.0);
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
    Some(sound_spec(event).key)
}

pub fn sound_event_gain(event: &SoundEvent) -> f32 {
    sound_spec(event).gain
}

struct SoundSpec {
    key: &'static str,
    gain: f32,
}

fn sound_spec(event: &SoundEvent) -> SoundSpec {
    match event {
        SoundEvent::Move => SoundSpec {
            key: "move",
            gain: 0.25,
        },
        SoundEvent::Rotate => SoundSpec {
            key: "rotate",
            gain: 0.35,
        },
        SoundEvent::SoftDrop => SoundSpec {
            key: "soft_drop",
            gain: 0.2,
        },
        SoundEvent::HardDrop => SoundSpec {
            key: "hard_drop",
            gain: 0.6,
        },
        SoundEvent::Hold => SoundSpec {
            key: "hold",
            gain: 0.5,
        },
        SoundEvent::LineClear(1) => SoundSpec {
            key: "line_clear_1",
            gain: 0.6,
        },
        SoundEvent::LineClear(2) => SoundSpec {
            key: "line_clear_2",
            gain: 0.7,
        },
        SoundEvent::LineClear(3) => SoundSpec {
            key: "line_clear_3",
            gain: 0.8,
        },
        SoundEvent::LineClear(_) => SoundSpec {
            key: "line_clear_4",
            gain: 0.9,
        },
        SoundEvent::GameOver => SoundSpec {
            key: "game_over",
            gain: 0.8,
        },
    }
}

fn soft_clip(sample: f32) -> f32 {
    sample / (1.0 + sample.abs())
}

fn f32_to_bits(value: f32) -> u32 {
    value.to_bits()
}

fn bits_to_f32(value: u32) -> f32 {
    f32::from_bits(value)
}

fn push_voice(voices: &mut Vec<Voice>, voice: Voice) {
    if voices.len() >= MAX_VOICES {
        voices.remove(0);
    }
    voices.push(voice);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_voice(gain: f32) -> Voice {
        Voice {
            samples: Arc::new(vec![0.0; 4]),
            channels: 1,
            position: 0.0,
            step: 1.0,
            gain,
        }
    }

    #[test]
    fn push_voice_caps_active_voices() {
        let mut voices = Vec::new();
        for i in 0..MAX_VOICES {
            push_voice(&mut voices, test_voice(i as f32));
        }
        assert_eq!(voices.len(), MAX_VOICES);

        push_voice(&mut voices, test_voice(99.0));
        assert_eq!(voices.len(), MAX_VOICES);
        assert!(voices.iter().any(|voice| voice.gain == 99.0));
        assert!(!voices.iter().any(|voice| voice.gain == 0.0));
    }

    #[test]
    fn master_gain_bits_roundtrip() {
        let value = 0.42;
        assert_eq!(bits_to_f32(f32_to_bits(value)), value);
    }
}
