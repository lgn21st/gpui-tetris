use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use crossbeam_channel::{Receiver, Sender, TrySendError};

use crate::game::state::SoundEvent;

#[derive(Clone)]
pub struct AudioEngine {
    sender: Sender<SoundEvent>,
    _stream: Arc<cpal::Stream>,
    master_gain: Arc<AtomicU32>,
}

const DEFAULT_MASTER_GAIN: f32 = 0.6;
const MAX_VOICES: usize = 16;
const SOUND_QUEUE_CAPACITY: usize = 256;

impl AudioEngine {
    pub fn new(asset_dir: &Path) -> anyhow::Result<Self> {
        let (tx, rx) = crossbeam_channel::bounded(SOUND_QUEUE_CAPACITY);
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
        let _ = try_enqueue_sound(&self.sender, event);
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

    anyhow::ensure!(
        !assets.is_empty(),
        "no usable SFX assets in {}",
        asset_dir.display()
    );
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
    let sample_rate = config.sample_rate();

    let params = StreamParams {
        channels,
        sample_rate,
        rx,
        assets,
        voices: Vec::with_capacity(MAX_VOICES),
        master_gain,
    };

    let stream = match config.sample_format() {
        cpal::SampleFormat::F32 => build_typed_stream::<f32>(&device, &config, params)?,
        cpal::SampleFormat::I16 => build_typed_stream::<i16>(&device, &config, params)?,
        cpal::SampleFormat::U16 => build_typed_stream::<u16>(&device, &config, params)?,
        _ => {
            return Err(anyhow::anyhow!(
                "unsupported sample format: {:?}",
                config.sample_format()
            ));
        }
    };

    Ok(stream)
}

struct StreamParams {
    channels: usize,
    sample_rate: u32,
    rx: Receiver<SoundEvent>,
    assets: HashMap<&'static str, SoundAsset>,
    voices: Vec<Voice>,
    master_gain: Arc<AtomicU32>,
}

fn build_typed_stream<T>(
    device: &cpal::Device,
    config: &cpal::SupportedStreamConfig,
    mut params: StreamParams,
) -> anyhow::Result<cpal::Stream>
where
    T: cpal::SizedSample + cpal::FromSample<f32>,
{
    device
        .build_output_stream(
            (*config).into(),
            move |data: &mut [T], _| {
                let gain = bits_to_f32(params.master_gain.load(Ordering::Relaxed));
                render_audio(
                    data,
                    params.channels,
                    params.sample_rate,
                    &params.rx,
                    &params.assets,
                    &mut params.voices,
                    gain,
                );
            },
            move |err| eprintln!("audio stream error: {err}"),
            None,
        )
        .map_err(Into::into)
}

fn select_output_config(device: &cpal::Device) -> anyhow::Result<cpal::SupportedStreamConfig> {
    let candidates = device.supported_output_configs()?;
    let mut selected = None;

    for config in candidates {
        if config.channels() == 2 {
            let min = config.min_sample_rate();
            let max = config.max_sample_rate();
            if min <= 44_100 && max >= 44_100 {
                selected = Some(config.with_sample_rate(44_100));
                break;
            }
        }
    }

    match selected {
        Some(config) => Ok(config),
        None => Ok(device.default_output_config()?),
    }
}

fn render_audio<T: cpal::Sample + cpal::FromSample<f32>>(
    output: &mut [T],
    channels: usize,
    device_rate: u32,
    rx: &Receiver<SoundEvent>,
    assets: &HashMap<&'static str, SoundAsset>,
    voices: &mut Vec<Voice>,
    master_gain: f32,
) {
    // A bounded queue alone cannot bound draining while a producer refills it.
    for event in rx.try_iter().take(32) {
        let (key, gain) = sound_event_spec(&event);
        if let Some(asset) = assets.get(key) {
            push_voice(
                voices,
                Voice {
                    samples: asset.samples.clone(),
                    channels: asset.channels,
                    position: 0.0,
                    step: asset.sample_rate as f32 / device_rate as f32,
                    gain,
                },
            );
        }
    }
    let master = master_gain.clamp(0.0, 1.0);
    for frame in output.chunks_mut(channels) {
        let (mut left, mut right) = (0.0, 0.0);
        for voice in voices.iter_mut() {
            let base = voice.position as usize * voice.channels as usize;
            let l = voice.samples.get(base).copied().unwrap_or(0.0) * voice.gain;
            let r = if voice.channels > 1 {
                voice.samples.get(base + 1).copied().unwrap_or(0.0) * voice.gain
            } else {
                l
            };
            left += l;
            right += r;
            voice.position += voice.step;
        }
        frame.fill(T::from_sample(0.0));
        let sample = if channels == 1 {
            (left + right) * 0.5
        } else {
            left
        };
        frame[0] = T::from_sample(soft_clip(sample * master).clamp(-1.0, 1.0));
        if frame.len() > 1 {
            frame[1] = T::from_sample(soft_clip(right * master).clamp(-1.0, 1.0));
        }
    }
    voices
        .retain(|voice| (voice.position as usize) < voice.samples.len() / voice.channels as usize);
}

fn load_wav(path: &Path) -> anyhow::Result<SoundAsset> {
    let mut reader = hound::WavReader::open(path)?;
    let spec = reader.spec();
    if spec.channels == 0 || spec.sample_rate == 0 || !(1..=32).contains(&spec.bits_per_sample) {
        return Err(anyhow::anyhow!(
            "WAV must have channels, sample rate, and 1..=32 bits per sample"
        ));
    }
    let samples: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Float => reader.samples::<f32>().collect::<Result<_, _>>()?,
        hound::SampleFormat::Int if spec.bits_per_sample <= 8 => {
            let peak = ((1_i64 << (spec.bits_per_sample - 1)) - 1).max(1) as f32;
            reader
                .samples::<i8>()
                .map(|sample| sample.map(|value| value as f32 / peak))
                .collect::<Result<_, _>>()?
        }
        hound::SampleFormat::Int if spec.bits_per_sample <= 16 => {
            let peak = ((1_i64 << (spec.bits_per_sample - 1)) - 1) as f32;
            reader
                .samples::<i16>()
                .map(|sample| sample.map(|value| value as f32 / peak))
                .collect::<Result<_, _>>()?
        }
        hound::SampleFormat::Int => {
            let peak = ((1_i64 << (spec.bits_per_sample - 1)) - 1) as f32;
            reader
                .samples::<i32>()
                .map(|sample| sample.map(|value| value as f32 / peak))
                .collect::<Result<_, _>>()?
        }
    };

    Ok(SoundAsset {
        samples: Arc::new(samples),
        channels: spec.channels,
        sample_rate: spec.sample_rate,
    })
}

fn try_enqueue_sound(sender: &Sender<SoundEvent>, event: SoundEvent) -> bool {
    match sender.try_send(event) {
        Ok(()) => true,
        Err(TrySendError::Full(_) | TrySendError::Disconnected(_)) => false,
    }
}

pub fn sound_event_spec(event: &SoundEvent) -> (&'static str, f32) {
    match event {
        SoundEvent::Move => ("move", 0.25),
        SoundEvent::Rotate => ("rotate", 0.35),
        SoundEvent::SoftDrop => ("soft_drop", 0.2),
        SoundEvent::HardDrop => ("hard_drop", 0.6),
        SoundEvent::Hold => ("hold", 0.5),
        SoundEvent::LineClear(1) => ("line_clear_1", 0.6),
        SoundEvent::LineClear(2) => ("line_clear_2", 0.7),
        SoundEvent::LineClear(3) => ("line_clear_3", 0.8),
        SoundEvent::LineClear(_) => ("line_clear_4", 0.9),
        SoundEvent::GameOver => ("game_over", 0.8),
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
    fn mixer_converts_samples_and_removes_finished_voices_without_scratch_allocation() {
        let (_tx, rx) = crossbeam_channel::bounded(1);
        let assets = HashMap::new();
        let voice = Voice {
            samples: Arc::new(vec![1.0, -1.0]),
            channels: 2,
            position: 0.0,
            step: 1.0,
            gain: 1.0,
        };
        let mut voices = Vec::with_capacity(MAX_VOICES);
        voices.push(voice.clone());
        let mut output = [0.0_f32; 4];
        render_audio(&mut output, 2, 44_100, &rx, &assets, &mut voices, 0.5);
        assert_eq!(output, [soft_clip(0.5), soft_clip(-0.5), 0.0, 0.0]);
        assert!(voices.is_empty());
        assert_eq!(voices.capacity(), MAX_VOICES);
        voices.push(voice);
        let mut integer = [0_i16; 2];
        render_audio(&mut integer, 2, 44_100, &rx, &assets, &mut voices, 0.5);
        assert_eq!(integer[0], <i16 as cpal::Sample>::from_sample(output[0]));
        assert_eq!(integer[1], <i16 as cpal::Sample>::from_sample(output[1]));
    }

    #[test]
    fn mixer_limits_events_per_callback_even_with_queue_backlog() {
        let (tx, rx) = crossbeam_channel::bounded(64);
        for _ in 0..64 {
            tx.send(SoundEvent::Move).unwrap();
        }
        render_audio(
            &mut [0.0_f32; 2],
            2,
            44_100,
            &rx,
            &HashMap::new(),
            &mut Vec::with_capacity(MAX_VOICES),
            1.0,
        );
        assert_eq!(rx.len(), 32);
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

    #[test]
    fn sound_queue_drops_excess_without_growing() {
        let (sender, receiver) = crossbeam_channel::bounded(1);
        assert!(try_enqueue_sound(&sender, SoundEvent::Move));
        assert!(!try_enqueue_sound(&sender, SoundEvent::Rotate));
        assert_eq!(receiver.len(), 1);
    }

    #[test]
    fn load_wav_accepts_float_samples() {
        let path = std::env::temp_dir().join(format!(
            "gpui-tetris-audio-{}-{}.wav",
            std::process::id(),
            std::thread::current().name().unwrap_or("test")
        ));
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: 44_100,
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        };
        let mut writer = hound::WavWriter::create(&path, spec).expect("create WAV");
        writer.write_sample(0.5_f32).expect("write sample");
        writer.finalize().expect("finalize WAV");

        let asset = load_wav(&path).expect("load float WAV");
        let _ = std::fs::remove_file(&path);
        assert_eq!(asset.samples.as_slice(), &[0.5]);
    }
}
