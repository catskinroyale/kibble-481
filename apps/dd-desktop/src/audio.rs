//! The audio output: the engine runs inside cpal's callback and takes commands
//! through a channel. Adapted from Ambient MIDI Generator's player (MIT).

use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender, channel};

use dd_seq::engine::{Cmd, Engine, Shared};

pub struct Audio {
    tx: Sender<Cmd>,
    pub shared: Arc<Shared>,
    pub sample_rate: u32,
    _stream: cpal::Stream,
}

struct State {
    engine: Engine,
    rx: Receiver<Cmd>,
    shared: Arc<Shared>,
    left: Vec<f32>,
    right: Vec<f32>,
}

impl State {
    fn fill<T: Copy>(&mut self, data: &mut [T], channels: usize, conv: impl Fn(f32) -> T) {
        while let Ok(cmd) = self.rx.try_recv() {
            self.engine.handle(cmd);
        }
        let frames = data.len() / channels.max(1);
        self.shared.set_latency(frames as f64 / self.engine.sample_rate() as f64);
        if self.left.len() < frames {
            self.left.resize(frames, 0.0);
            self.right.resize(frames, 0.0);
        }
        let (l, r) = (&mut self.left[..frames], &mut self.right[..frames]);
        self.engine.render(l, r);
        for (i, frame) in data.chunks_mut(channels.max(1)).enumerate() {
            for (c, s) in frame.iter_mut().enumerate() {
                *s = conv(if c % 2 == 0 { l[i] } else { r[i] });
            }
        }
    }
}

impl Audio {
    pub fn start() -> Result<Audio, String> {
        use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
        let host = cpal::default_host();
        let device = host.default_output_device().ok_or("No audio output device found")?;
        let supported = device.default_output_config().map_err(|e| e.to_string())?;
        let format = supported.sample_format();
        let config: cpal::StreamConfig = supported.into();
        let sample_rate = config.sample_rate;
        let channels = config.channels as usize;
        let (tx, rx) = channel::<Cmd>();
        let shared = Shared::new();
        let mut state = State {
            engine: Engine::new(sample_rate, shared.clone()),
            rx,
            shared: shared.clone(),
            left: vec![0.0; 4096],
            right: vec![0.0; 4096],
        };
        let err_fn = |e| eprintln!("audio stream error: {e}");
        let stream = match format {
            cpal::SampleFormat::F32 => device.build_output_stream(
                config,
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| state.fill(data, channels, |x| x),
                err_fn,
                None,
            ),
            cpal::SampleFormat::I16 => device.build_output_stream(
                config,
                move |data: &mut [i16], _: &cpal::OutputCallbackInfo| {
                    state.fill(data, channels, |x| (x.clamp(-1.0, 1.0) * i16::MAX as f32) as i16)
                },
                err_fn,
                None,
            ),
            other => return Err(format!("Unsupported output sample format {other:?}")),
        }
        .map_err(|e| e.to_string())?;
        stream.play().map_err(|e| e.to_string())?;
        Ok(Audio { tx, shared, sample_rate, _stream: stream })
    }

    pub fn send(&self, cmd: Cmd) {
        let _ = self.tx.send(cmd);
    }

    pub fn sender(&self) -> Sender<Cmd> {
        self.tx.clone()
    }
}
