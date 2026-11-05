use bevy::prelude::Vec2;

use crate::analyzer::Detection;

/// Stable identity of a tracked object across frames
pub type TrackId = u64;

/// How detections are joined into tracks.
#[derive(Debug, Clone)]
pub struct TrackerConfig {
    /// Furthest a detection may be from a track's last position to continue it, in normalised
    /// camera units
    pub max_distance: f32,
    /// Frames in a row a new object must be seen before it is reported (filters single-frame noise)
    pub min_hits: u32,
    /// Frames an object may go unseen before its track is lost
    pub max_missed: u32,
}

impl Default for TrackerConfig {
    fn default() -> Self {
        Self { max_distance: 0.08, min_hits: 2, max_missed: 6 }
    }
}

/// A confirmed track seen in this frame.
#[derive(Debug, Clone, PartialEq)]
pub struct TrackedDetection {
    pub track: TrackId,
    /// First frame this track is reported in
    pub appeared: bool,
    pub detection: Detection,
}

/// A confirmed track that is no longer seen.
#[derive(Debug, Clone, PartialEq)]
pub struct LostTrack {
    pub track: TrackId,
    pub label: String,
}

#[derive(Debug, Default)]
pub struct TrackerUpdate {
    pub seen: Vec<TrackedDetection>,
    pub lost: Vec<LostTrack>,
}

struct Track {
    id: TrackId,
    label: String,
    pos: Vec2,
    hits: u32,
    missed: u32,
    confirmed: bool,
}

/// Joins per-frame detections into tracks by nearest neighbour within the same label.
#[derive(Default)]
pub struct Tracker {
    pub config: TrackerConfig,
    tracks: Vec<Track>,
    next_id: TrackId,
}

impl Tracker {
    pub fn new(config: TrackerConfig) -> Self {
        Self { config, ..Default::default() }
    }

    pub fn update(&mut self, detections: &[Detection]) -> TrackerUpdate {
        let mut update = TrackerUpdate::default();
        let mut matched = vec![false; self.tracks.len()];

        // Strongest detections claim their nearest track first
        let mut order: Vec<usize> = (0..detections.len()).collect();
        order.sort_by(|&a, &b| detections[b].confidence.total_cmp(&detections[a].confidence));

        for index in order {
            let detection = &detections[index];
            let nearest = self
                .tracks
                .iter()
                .enumerate()
                .filter(|(i, t)| !matched[*i] && t.label == detection.label)
                .map(|(i, t)| (i, t.pos.distance(detection.norm_pos)))
                .filter(|(_, d)| *d <= self.config.max_distance)
                .min_by(|a, b| a.1.total_cmp(&b.1))
                .map(|(i, _)| i);

            let slot = match nearest {
                Some(i) => {
                    matched[i] = true;
                    let track = &mut self.tracks[i];
                    track.pos = detection.norm_pos;
                    track.hits += 1;
                    track.missed = 0;
                    i
                }
                None => {
                    self.next_id += 1;
                    self.tracks.push(Track {
                        id: self.next_id,
                        label: detection.label.clone(),
                        pos: detection.norm_pos,
                        hits: 1,
                        missed: 0,
                        confirmed: false,
                    });
                    matched.push(true);
                    self.tracks.len() - 1
                }
            };

            let track = &mut self.tracks[slot];
            let appeared = !track.confirmed && track.hits >= self.config.min_hits;
            track.confirmed |= appeared;
            if track.confirmed {
                update.seen.push(TrackedDetection { track: track.id, appeared, detection: detection.clone() });
            }
        }

        let max_missed = self.config.max_missed;
        let mut index = 0;
        self.tracks.retain_mut(|track| {
            let was_matched = matched[index];
            index += 1;
            if was_matched {
                return true;
            }
            track.missed += 1;
            if track.missed <= max_missed {
                return true;
            }
            if track.confirmed {
                update.lost.push(LostTrack { track: track.id, label: track.label.clone() });
            }
            false
        });

        update
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dot(x: f32, y: f32) -> Detection {
        Detection { label: "laser".into(), norm_pos: Vec2::new(x, y), size: Vec2::ZERO, confidence: 1.0 }
    }

    #[test]
    fn a_moving_dot_keeps_one_track_until_it_is_gone() {
        let mut tracker = Tracker::new(TrackerConfig { max_distance: 0.1, min_hits: 2, max_missed: 1 });

        assert!(tracker.update(&[dot(0.5, 0.5)]).seen.is_empty(), "not reported before min_hits");

        let second = tracker.update(&[dot(0.52, 0.5)]);
        assert_eq!(second.seen.len(), 1);
        assert!(second.seen[0].appeared);
        let id = second.seen[0].track;

        let third = tracker.update(&[dot(0.54, 0.51)]);
        assert_eq!(third.seen[0].track, id);
        assert!(!third.seen[0].appeared);

        assert!(tracker.update(&[]).lost.is_empty(), "one missed frame is tolerated");
        assert_eq!(tracker.update(&[]).lost, vec![LostTrack { track: id, label: "laser".into() }]);
    }

    #[test]
    fn far_apart_or_differently_labelled_detections_are_separate_tracks() {
        let mut tracker = Tracker::new(TrackerConfig { min_hits: 1, ..Default::default() });
        let mut person = dot(0.51, 0.5);
        person.label = "person".into();

        let update = tracker.update(&[dot(0.5, 0.5), dot(0.9, 0.9), person]);
        let mut ids: Vec<TrackId> = update.seen.iter().map(|s| s.track).collect();
        ids.dedup();
        assert_eq!(ids.len(), 3);
    }

    #[test]
    fn unconfirmed_noise_is_never_reported_as_lost() {
        let mut tracker = Tracker::new(TrackerConfig { min_hits: 3, max_missed: 0, ..Default::default() });
        tracker.update(&[dot(0.5, 0.5)]);
        assert!(tracker.update(&[]).lost.is_empty());
    }
}
