//! Hand-off from the Bevy tick (≈60 Hz, jittery) to the DAC thread (one sample per point, on the
//! DAC clock).
//!
//! The Bevy side renders blocks and [`GalvoAudioBus::push`]es them. The DAC thread owns a
//! [`GalvoAudioOutput`] and asks it for one offset per point of each frame it writes. The
//! output keeps a small local backlog, so it locks the shared queue with `try_lock` at most once
//! per frame and never blocks the DAC deadline. Missing samples fade out (see [`crate::guard`]).
//!
//! The DAC owns the clock: the output publishes the real point rate, and the producer renders
//! at that rate.

use crate::guard::{Guard, GuardSettings};
use common::audio_stream::StereoFrame as Frame;
use bevy::prelude::Resource;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicU32, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

/// Point rate assumed until a DAC output starts
pub const DEFAULT_SAMPLE_RATE: u32 = 30_000;

struct Shared {
    queue: Mutex<VecDeque<Frame>>,
    sample_rate: AtomicU32,
    consumers: AtomicUsize,
    settings: Mutex<GuardSettings>,
    settings_generation: AtomicU64,
}

/// Producer side, shared as a Bevy resource. Cheap to clone.
#[derive(Resource, Clone)]
pub struct GalvoAudioBus(Arc<Shared>);

impl Default for GalvoAudioBus {
    fn default() -> Self {
        Self::new(GuardSettings::default())
    }
}

impl GalvoAudioBus {
    pub fn new(settings: GuardSettings) -> Self {
        Self(Arc::new(Shared {
            queue: Mutex::new(VecDeque::new()),
            sample_rate: AtomicU32::new(DEFAULT_SAMPLE_RATE),
            consumers: AtomicUsize::new(0),
            settings: Mutex::new(settings),
            settings_generation: AtomicU64::new(1),
        }))
    }

    /// Samples per second the galvos are consuming
    pub fn sample_rate(&self) -> u32 {
        self.0.sample_rate.load(Ordering::Acquire)
    }

    /// True while a DAC thread holds an output
    pub fn has_consumer(&self) -> bool {
        self.0.consumers.load(Ordering::Acquire) > 0
    }

    /// Frames waiting in the shared queue
    pub fn queued(&self) -> usize {
        self.0.queue.lock().map(|q| q.len()).unwrap_or(0)
    }

    pub fn push(&self, frames: &[Frame]) {
        if let Ok(mut queue) = self.0.queue.lock() {
            queue.extend(frames.iter().copied());
        }
    }

    pub fn clear(&self) {
        if let Ok(mut queue) = self.0.queue.lock() {
            queue.clear();
        }
    }

    /// New guard settings; outputs pick them up at their next frame
    pub fn set_settings(&self, settings: GuardSettings) {
        if let Ok(mut current) = self.0.settings.lock() {
            if *current != settings {
                *current = settings;
                self.0.settings_generation.fetch_add(1, Ordering::AcqRel);
            }
        }
    }

    pub fn settings(&self) -> GuardSettings {
        self.0.settings.lock().map(|s| s.clone()).unwrap_or_default()
    }

    /// Consumer for a DAC thread running at `sample_rate` points per second
    pub fn output(&self, sample_rate: u32) -> GalvoAudioOutput {
        self.0.sample_rate.store(sample_rate.max(1), Ordering::Release);
        self.0.consumers.fetch_add(1, Ordering::AcqRel);
        GalvoAudioOutput {
            bus: self.clone(),
            local: VecDeque::new(),
            guard: Guard::new(self.settings(), sample_rate),
            generation: self.0.settings_generation.load(Ordering::Acquire),
            sample_rate,
            offsets: Vec::new(),
        }
    }
}

/// Consumer side, owned by the DAC thread.
pub struct GalvoAudioOutput {
    bus: GalvoAudioBus,
    local: VecDeque<Frame>,
    guard: Guard,
    generation: u64,
    sample_rate: u32,
    offsets: Vec<(i16, i16)>,
}

impl GalvoAudioOutput {
    /// Whether audio should also drive frames with nothing lit
    pub fn play_when_blank(&self) -> bool {
        self.guard.settings().play_when_blank
    }

    /// Offsets in DAC units for the next `n` points, to add to the frame's coordinates.
    /// Never blocks.
    pub fn render(&mut self, n: usize) -> &[(i16, i16)] {
        self.refresh_settings();
        if self.local.len() < n {
            if let Ok(mut queue) = self.bus.0.queue.try_lock() {
                // Take this frame plus one spare frame, so a contested lock next time is harmless
                let take = (2 * n - self.local.len()).min(queue.len());
                self.local.extend(queue.drain(..take));
            }
        }
        self.offsets.clear();
        for _ in 0..n {
            let sample = self.local.pop_front();
            self.offsets.push(self.guard.process(sample));
        }
        &self.offsets
    }

    fn refresh_settings(&mut self) {
        let generation = self.bus.0.settings_generation.load(Ordering::Acquire);
        if generation == self.generation {
            return;
        }
        if let Ok(settings) = self.bus.0.settings.try_lock() {
            self.guard.configure(settings.clone(), self.sample_rate);
            self.generation = generation;
        }
    }
}

impl Drop for GalvoAudioOutput {
    fn drop(&mut self) {
        self.bus.0.consumers.fetch_sub(1, Ordering::AcqRel);
    }
}

/// Adds `offsets` to 12-bit coordinates, clamped to `0..=max`
pub fn apply_offset(coord: u16, offset: i16, max: u16) -> u16 {
    (coord as i32 + offset as i32).clamp(0, max as i32) as u16
}
