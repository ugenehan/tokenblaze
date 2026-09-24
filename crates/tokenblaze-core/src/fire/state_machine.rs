use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

use super::*;
use crate::data::UsageSource;

/// Lifecycle state of the rendered fire.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FirePhase {
    Unlit,
    Flame,
    Ember,
    Out,
}

/// Visual intensity band. Bands intentionally make the higher tiers harder
/// to reach so ordinary usage stays readable as a small fire.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FireTier {
    Hush,
    Glow,
    Crackle,
    Roar,
    Blaze,
}

impl FireTier {
    pub fn from_intensity(intensity: f64) -> Self {
        match intensity {
            x if x < 0.16 => Self::Hush,
            x if x < 0.34 => Self::Glow,
            x if x < 0.56 => Self::Crackle,
            x if x < 0.78 => Self::Roar,
            _ => Self::Blaze,
        }
    }

    pub fn label_key(self) -> &'static str {
        match self {
            Self::Hush => "tier.hush",
            Self::Glow => "tier.glow",
            Self::Crackle => "tier.crackle",
            Self::Roar => "tier.roar",
            Self::Blaze => "tier.blaze",
        }
    }
}

/// Immutable state passed from the logic layer to renderers and audio.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FireSnapshot {
    pub intensity: f64,
    pub fuel: f64,
    pub ember_heat: f64,
    pub phase: FirePhase,
    pub spark_burst: f64,
    pub tier: FireTier,
    pub color_mix: FlameColorMix,
}

impl FireSnapshot {
    pub const EXTINGUISHED: Self = Self {
        intensity: 0.0,
        fuel: 0.0,
        ember_heat: 0.0,
        phase: FirePhase::Unlit,
        spark_burst: 0.0,
        tier: FireTier::Hush,
        color_mix: FlameColorMix::CLASSIC,
    };
}

/// Fixed tuning values for the fire response curve.
#[derive(Debug, Clone)]
pub struct FireTuning {
    pub intensity_window_seconds: f64,
    pub tpm_anchors: [(f64, f64); 6],
    pub event_credit_tokens: f64,
    pub intensity_rise_seconds: f64,
    pub intensity_fall_seconds: f64,
    pub fuel_token_scale: f64,
    pub fuel_burn_per_second_full: f64,
    pub fuel_burn_per_second_idle: f64,
    pub ember_decay_per_second: f64,
    pub ember_from_fuel_gain: f64,
    pub max_spark_burst: f64,
    pub spark_decay_per_second: f64,
}

impl Default for FireTuning {
    fn default() -> Self {
        Self {
            intensity_window_seconds: 60.0,
            tpm_anchors: [
                (0.0, 0.0),
                (800.0, 0.10),
                (2_500.0, 0.26),
                (12_000.0, 0.48),
                (45_000.0, 0.72),
                (180_000.0, 1.0),
            ],
            event_credit_tokens: 45_000.0,
            intensity_rise_seconds: 2.5,
            intensity_fall_seconds: 14.0,
            fuel_token_scale: 120_000.0,
            fuel_burn_per_second_full: 1.0 / 90.0,
            fuel_burn_per_second_idle: 1.0 / 150.0,
            ember_decay_per_second: 1.0 / (2.5 * 60.0),
            ember_from_fuel_gain: 0.4,
            max_spark_burst: 1.0,
            spark_decay_per_second: 1.4,
        }
    }
}

/// Drives the fire simulation forward.
///
/// Pure-logic core: call `ingest()` when new tokens arrive,
/// call `tick(dt)` every frame to let the fire evolve,
/// read `snapshot()` to get the current visual state.
#[derive(Debug)]
pub struct FireStateMachine {
    live: FireSnapshot,
    tuning: FireTuning,

    /// Recent token inflows — the rolling window that drives intensity.
    recent_inflows: Vec<(Instant, f64, Option<UsageSource>)>,
    /// Events detected after startup, used only by the live rate estimate.
    recent_live_rate_inflows: Vec<(Instant, f64)>,
    /// Smoothed burn intensity (live only — daily floor is added at compose time).
    burn_intensity: f64,
    /// Smoothed per-source weights for color mixing.
    smoothed_weights: [f64; 7], // indexed by UsageSource discriminant
    /// Stable tokens/sec estimate for hover display.
    displayed_tokens_per_second: f64,

    today_by_source: [i64; 7],

    /// Today's tokens shown in the snapshot (cached after compose).
    displayed_color_mix: FlameColorMix,

    // Preview / debug
    preview_style: Option<FirePreviewStyle>,
    custom_preview: Option<FireSnapshot>,
    pub animation_paused: bool,
    pub reduce_motion: bool,
    pub color_palette_epoch: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FirePreviewStyle {
    Out,
    Ember,
    Hush,
    Glow,
    Crackle,
    Roar,
    Blaze,
}

impl FirePreviewStyle {
    pub fn label_key(self) -> &'static str {
        match self {
            FirePreviewStyle::Out => "phase.out",
            FirePreviewStyle::Ember => "phase.ember",
            FirePreviewStyle::Hush => "tier.hush",
            FirePreviewStyle::Glow => "tier.glow",
            FirePreviewStyle::Crackle => "tier.crackle",
            FirePreviewStyle::Roar => "tier.roar",
            FirePreviewStyle::Blaze => "tier.blaze",
        }
    }
}

/// Size of the desktop campfire.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FlameSize {
    Small,
    #[default]
    Medium,
    Large,
}

impl FlameSize {
    pub fn all() -> [FlameSize; 3] {
        [FlameSize::Small, FlameSize::Medium, FlameSize::Large]
    }

    /// Display scale for the high-detail render buffer. The simulation uses
    /// twice the former resolution, so these values preserve the established
    /// small, medium, and large window sizes.
    pub fn pixel_scale(self) -> f32 {
        match self {
            FlameSize::Small => 1.0,
            FlameSize::Medium => 1.75,
            FlameSize::Large => 2.75,
        }
    }

    /// Content size in logical pixels.
    pub fn content_size(self) -> (f32, f32) {
        use crate::fire::{FIRE_HEIGHT, FIRE_WIDTH, LOG_HEIGHT, LOG_WIDTH};
        let scale = self.pixel_scale();
        let w = FIRE_WIDTH.max(LOG_WIDTH) as f32 * scale;
        let h = (FIRE_HEIGHT + LOG_HEIGHT) as f32 * scale;
        (w + 28.0, h + 36.0) // padding around the fire
    }

    pub fn label_key(self) -> &'static str {
        match self {
            FlameSize::Small => "size.small",
            FlameSize::Medium => "size.medium",
            FlameSize::Large => "size.large",
        }
    }
}

impl FirePreviewStyle {
    pub fn all() -> [FirePreviewStyle; 7] {
        use FirePreviewStyle::*;
        [Out, Ember, Hush, Glow, Crackle, Roar, Blaze]
    }

    pub fn snapshot(self) -> FireSnapshot {
        use FirePhase::*;
        use FireTier::*;
        match self {
            FirePreviewStyle::Out => FireSnapshot {
                intensity: 0.0,
                fuel: 0.0,
                ember_heat: 0.0,
                phase: Out,
                spark_burst: 0.0,
                tier: Hush,
                color_mix: FlameColorMix::CLASSIC,
            },
            FirePreviewStyle::Ember => FireSnapshot {
                intensity: 0.0,
                fuel: 0.0,
                ember_heat: 0.85,
                phase: Ember,
                spark_burst: 0.15,
                tier: Hush,
                color_mix: FlameColorMix::CLASSIC,
            },
            FirePreviewStyle::Hush => FireSnapshot {
                intensity: 0.10,
                fuel: 0.18,
                ember_heat: 0.35,
                phase: Flame,
                spark_burst: 0.10,
                tier: Hush,
                color_mix: FlameColorMix::CLASSIC,
            },
            FirePreviewStyle::Glow => FireSnapshot {
                intensity: 0.22,
                fuel: 0.35,
                ember_heat: 0.45,
                phase: Flame,
                spark_burst: 0.25,
                tier: Glow,
                color_mix: FlameColorMix::CLASSIC,
            },
            FirePreviewStyle::Crackle => FireSnapshot {
                intensity: 0.45,
                fuel: 0.55,
                ember_heat: 0.55,
                phase: Flame,
                spark_burst: 0.45,
                tier: Crackle,
                color_mix: FlameColorMix::CLASSIC,
            },
            FirePreviewStyle::Roar => FireSnapshot {
                intensity: 0.72,
                fuel: 0.78,
                ember_heat: 0.70,
                phase: Flame,
                spark_burst: 0.75,
                tier: Roar,
                color_mix: FlameColorMix::CLASSIC,
            },
            FirePreviewStyle::Blaze => FireSnapshot {
                intensity: 1.0,
                fuel: 1.0,
                ember_heat: 0.90,
                phase: Flame,
                spark_burst: 1.0,
                tier: Blaze,
                color_mix: FlameColorMix::CLASSIC,
            },
        }
    }
}

impl Default for FireStateMachine {
    fn default() -> Self {
        Self::new()
    }
}

impl FireStateMachine {
    pub fn new() -> Self {
        FireStateMachine {
            live: FireSnapshot::EXTINGUISHED,
            tuning: FireTuning::default(),
            recent_inflows: Vec::new(),
            recent_live_rate_inflows: Vec::new(),
            burn_intensity: 0.0,
            smoothed_weights: [0.0; 7],
            displayed_tokens_per_second: 0.0,
            today_by_source: [0; 7],
            displayed_color_mix: FlameColorMix::CLASSIC,
            preview_style: None,
            custom_preview: None,
            animation_paused: false,
            reduce_motion: false,
            color_palette_epoch: 0,
        }
    }

    /// Current display snapshot — respects preview mode.
    pub fn snapshot(&self) -> FireSnapshot {
        if let Some(custom) = self.custom_preview {
            return custom;
        }
        if let Some(style) = self.preview_style {
            return style.snapshot();
        }
        self.live
    }

    pub fn is_previewing(&self) -> bool {
        self.preview_style.is_some() || self.custom_preview.is_some()
    }

    /// Intensity used to size the live flame, excluding the daily historical
    /// floor. Explicit previews continue to use their requested intensity.
    pub fn visual_intensity(&self) -> f64 {
        if self.is_previewing() {
            self.snapshot().intensity
        } else {
            self.burn_intensity
        }
    }

    pub fn preview_style(&self) -> Option<FirePreviewStyle> {
        self.preview_style
    }

    pub fn show_preview(&mut self, style: FirePreviewStyle) {
        self.custom_preview = None;
        self.preview_style = Some(style);
    }

    pub fn show_custom_preview(&mut self, intensity: f64) {
        let i = intensity.clamp(0.0, 1.0);
        self.preview_style = None;
        let tier = FireTier::from_intensity(i);
        let phase = match i {
            x if x < 0.02 => FirePhase::Out,
            x if x < 0.08 => FirePhase::Ember,
            _ => FirePhase::Flame,
        };
        self.custom_preview = Some(FireSnapshot {
            intensity: if phase == FirePhase::Ember || phase == FirePhase::Out {
                0.0
            } else {
                i
            },
            fuel: i,
            ember_heat: if phase == FirePhase::Ember {
                0.85
            } else {
                0.2_f64.max(i * 0.9)
            },
            phase,
            spark_burst: i,
            tier,
            color_mix: self.displayed_color_mix,
        });
    }

    pub fn return_to_live(&mut self) {
        self.preview_style = None;
        self.custom_preview = None;
    }

    /// Update today's totals (called by the data layer).
    pub fn update_today_tokens(&mut self, _tokens: i64, by_source: [i64; 7]) {
        self.today_by_source = by_source;
        self.live = self.compose(self.live);
    }

    /// Inject new token usage into the fire.
    pub fn ingest(&mut self, tokens: f64, source: Option<UsageSource>, at: Instant, animate: bool) {
        if tokens <= 0.0 {
            return;
        }

        // Historical events restore fuel and ember warmth on startup. Only
        // events detected during this run may drive the live flame intensity
        // and hover rate; otherwise startup history briefly creates a large
        // flame that shrinks as the rolling window expires.
        if animate {
            self.recent_inflows.push((at, tokens, source));
            self.prune_inflows(at);
            self.recent_live_rate_inflows.push((at, tokens));
            self.refresh_tokens_per_second(at);
        }

        let compressed = soft_compress(tokens);
        let fuel_gain = (compressed / self.tuning.fuel_token_scale).min(0.55);

        let mut next = self.live;
        next.fuel = (next.fuel + fuel_gain).min(1.0);
        next.ember_heat = next
            .ember_heat
            .max(
                next.fuel * self.tuning.ember_from_fuel_gain
                    + self.instantaneous_rate_push(tokens) * 0.25,
            )
            .min(1.0);

        if animate && self.preview_style.is_none() {
            let burst = self.instantaneous_rate_push(tokens).min(1.0);
            next.spark_burst =
                (next.spark_burst + 0.2 + burst * 0.9).min(self.tuning.max_spark_burst);
        }

        let target = self.target_intensity(at);
        self.burn_intensity = self
            .burn_intensity
            .max((self.burn_intensity + (target - self.burn_intensity) * 0.55).min(1.0));
        next.intensity = self.burn_intensity;
        next.phase = FirePhase::Flame;

        self.smooth_color_mix(0.9);
        next.color_mix = self.displayed_color_mix;
        self.live = self.compose(next);
    }

    /// Advance time by `dt` seconds.
    pub fn tick(&mut self, dt: f64) {
        if dt <= 0.0 {
            return;
        }
        if self.animation_paused {
            self.expire_tokens_per_second(Instant::now());
            return;
        }
        let dt = dt.min(1.0); // cap per-tick advance
        self.advance_by(dt);
    }

    /// Estimated live tokens/sec from events detected during this run.
    pub fn tokens_per_second(&self) -> f64 {
        self.displayed_tokens_per_second
    }

    // ── internal ────────────────────────────────────────────────

    fn advance_by(&mut self, dt: f64) {
        let now = Instant::now();
        self.prune_inflows_at(now);
        self.expire_tokens_per_second(now);

        let target = self.target_intensity_at(now);
        let mut fuel = self.live.fuel;
        let mut ember = self.live.ember_heat;
        let mut spark = self.live.spark_burst;

        // Exponential smoothing toward target intensity
        let tau = if target > self.burn_intensity {
            self.tuning.intensity_rise_seconds
        } else {
            self.tuning.intensity_fall_seconds
        };
        let alpha = 1.0 - (-dt / tau.max(0.05)).exp();
        self.burn_intensity += (target - self.burn_intensity) * alpha;
        self.burn_intensity = self.burn_intensity.clamp(0.0, 1.0);

        // Fuel burns down
        if fuel > 0.0 {
            let burn_rate = self.tuning.fuel_burn_per_second_idle
                + (self.tuning.fuel_burn_per_second_full - self.tuning.fuel_burn_per_second_idle)
                    * (self.burn_intensity.max(target)).powf(1.1);
            fuel = (fuel - burn_rate * dt).max(0.0);
            ember = ember.max(fuel * 0.55 + self.burn_intensity * 0.25);
        } else {
            ember = (ember - self.tuning.ember_decay_per_second * dt).max(0.0);
        }

        spark = (spark - self.tuning.spark_decay_per_second * dt).max(0.0);

        // Smoothly transition color mix
        self.smooth_color_mix(dt);

        self.live = self.compose(FireSnapshot {
            intensity: self.burn_intensity,
            fuel,
            ember_heat: ember,
            phase: FirePhase::Flame,
            spark_burst: spark,
            tier: FireTier::Hush,
            color_mix: self.displayed_color_mix,
        });
    }

    fn compose(&self, snap: FireSnapshot) -> FireSnapshot {
        let mut next = snap;
        let shown = self.burn_intensity;

        next.intensity = shown;
        next.tier = FireTier::from_intensity(shown);
        next.color_mix = self.displayed_color_mix;

        if shown >= 0.035 {
            next.phase = FirePhase::Flame;
            next.ember_heat = next.ember_heat.max(shown * 0.4);
        } else if next.ember_heat > 0.04 || next.fuel > 0.03 {
            next.phase = FirePhase::Ember;
            next.intensity = 0.0;
            next.tier = FireTier::Hush;
        } else if next.phase == FirePhase::Unlit {
            // keep unlit
        } else {
            next.phase = FirePhase::Out;
            next.intensity = 0.0;
            next.fuel = 0.0;
            next.ember_heat = 0.0;
            next.spark_burst = 0.0;
        }

        next
    }

    fn prune_inflows(&mut self, now: Instant) {
        self.prune_inflows_at(now);
    }

    fn prune_inflows_at(&mut self, now: Instant) {
        let cutoff = now - Duration::from_secs_f64(self.tuning.intensity_window_seconds);
        self.recent_inflows.retain(|(t, _, _)| *t >= cutoff);
    }

    fn target_intensity(&self, now: Instant) -> f64 {
        self.target_intensity_at(now)
    }

    fn target_intensity_at(&self, now: Instant) -> f64 {
        let window = self.tuning.intensity_window_seconds.max(1.0);
        let credited: f64 = self
            .recent_inflows
            .iter()
            .filter(|(t, _, _)| *t >= now - Duration::from_secs_f64(window))
            .map(|(_, tokens, _)| credit(*tokens, self.tuning.event_credit_tokens))
            .sum();
        let tpm = credited * (60.0 / window);
        intensity_from_tpm(tpm, &self.tuning.tpm_anchors)
    }

    fn instantaneous_rate_push(&self, tokens: f64) -> f64 {
        let credited = credit(tokens, self.tuning.event_credit_tokens);
        let tpm = credited * (60.0 / self.tuning.intensity_window_seconds.max(1.0));
        intensity_from_tpm(tpm, &self.tuning.tpm_anchors)
    }

    fn target_color_weights(&self, now: Instant) -> [f64; 7] {
        let mut live = [0.0; 7];
        let window = self.tuning.intensity_window_seconds;
        for (t, tokens, source) in &self.recent_inflows {
            if *t < now - Duration::from_secs_f64(window) {
                continue;
            }
            if let Some(src) = source {
                let idx = src.index();
                live[idx] += credit(*tokens, self.tuning.event_credit_tokens);
            }
        }
        let live_total: f64 = live.iter().sum();
        if live_total > 50.0 {
            return live.map(|v| v / live_total);
        }
        // Quiet window → fall back to today's share
        let day_total: i64 = self.today_by_source.iter().sum();
        if day_total <= 0 {
            return [0.0; 7];
        }
        self.today_by_source.map(|v| {
            if v > 0 {
                v as f64 / day_total as f64
            } else {
                0.0
            }
        })
    }

    fn smooth_color_mix(&mut self, dt: f64) {
        let target = self.target_color_weights(Instant::now());
        let tau: f64 = 2.8;
        let alpha = 1.0 - (-dt / tau.max(0.05)).exp();

        for (weight, target_weight) in self.smoothed_weights.iter_mut().zip(target) {
            *weight += (target_weight - *weight) * alpha;
            if *weight < 0.004 {
                *weight = 0.0;
            }
        }

        let sum: f64 = self.smoothed_weights.iter().sum();
        let weights = if sum > 0.02 {
            self.smoothed_weights.map(|v| v / sum)
        } else {
            [0.0; 7]
        };

        self.displayed_color_mix = FlameColorMix::from_weights_array(weights);
    }

    fn refresh_tokens_per_second(&mut self, now: Instant) {
        // Usage sources usually persist token totals only after a response or
        // tool step completes. Build the estimate from the latest minute, but
        // update it only when a real event arrives so it does not drift while
        // the source is idle.
        const LIVE_RATE_SAMPLE_SECONDS: f64 = 60.0;
        let rate_event_credit = 15_000.0;
        let cutoff = now
            .checked_sub(Duration::from_secs_f64(LIVE_RATE_SAMPLE_SECONDS))
            .unwrap_or(now);
        self.recent_live_rate_inflows
            .retain(|(at, _)| *at >= cutoff && *at <= now);
        let credited: f64 = self
            .recent_live_rate_inflows
            .iter()
            .map(|(_, tokens)| tokens.min(rate_event_credit))
            .sum();
        self.displayed_tokens_per_second = if credited > 0.0 {
            ((credited / 60.0).min(320.0) * 10.0).round() / 10.0
        } else {
            0.0
        };
    }

    fn expire_tokens_per_second(&mut self, now: Instant) {
        // Completed steps can be tens of seconds apart even while an agent is
        // active. Keep the last real estimate across those gaps, then switch
        // directly to zero after sustained inactivity.
        const LIVE_RATE_IDLE_TIMEOUT_SECONDS: f64 = 90.0;
        let Some((last_event_at, _)) = self.recent_live_rate_inflows.last() else {
            self.displayed_tokens_per_second = 0.0;
            return;
        };
        if now
            .checked_duration_since(*last_event_at)
            .is_some_and(|idle| idle.as_secs_f64() >= LIVE_RATE_IDLE_TIMEOUT_SECONDS)
        {
            self.recent_live_rate_inflows.clear();
            self.displayed_tokens_per_second = 0.0;
        }
    }
}

// ── helpers ────────────────────────────────────────────────────

fn credit(tokens: f64, cap: f64) -> f64 {
    tokens.min(cap)
}

fn intensity_from_tpm(tpm: f64, anchors: &[(f64, f64); 6]) -> f64 {
    if tpm <= anchors[0].0 {
        return anchors[0].1;
    }
    if tpm >= anchors[anchors.len() - 1].0 {
        return anchors[anchors.len() - 1].1;
    }
    for i in 0..anchors.len() - 1 {
        let a = anchors[i];
        let b = anchors[i + 1];
        if tpm <= b.0 {
            let span = (b.0 - a.0).max(1.0);
            let t = (tpm - a.0) / span;
            // smoothstep
            let s = t * t * (3.0 - 2.0 * t);
            return a.1 + (b.1 - a.1) * s;
        }
    }
    anchors[anchors.len() - 1].1
}

/// Mild compression for fuel gains only (intensity uses raw rate).
pub fn soft_compress(tokens: f64) -> f64 {
    if tokens <= 0.0 {
        return 0.0;
    }
    tokens / (1.0 + tokens / 140_000.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tier_thresholds_match_the_design_bands() {
        assert_eq!(FireTier::from_intensity(0.0), FireTier::Hush);
        assert_eq!(FireTier::from_intensity(0.159), FireTier::Hush);
        assert_eq!(FireTier::from_intensity(0.16), FireTier::Glow);
        assert_eq!(FireTier::from_intensity(0.56), FireTier::Roar);
        assert_eq!(FireTier::from_intensity(1.0), FireTier::Blaze);
    }

    #[test]
    fn ingestion_raises_live_fire_and_snapshot_is_copyable() {
        let mut fire = FireStateMachine::new();
        assert_eq!(fire.snapshot(), FireSnapshot::EXTINGUISHED);
        fire.ingest(2_500.0, Some(UsageSource::Codex), Instant::now(), true);
        let snapshot = fire.snapshot();
        assert!(snapshot.fuel > 0.0);
        assert_eq!(snapshot.phase, FirePhase::Flame);
        assert!(snapshot.intensity > 0.0);
    }

    #[test]
    fn live_fire_eventually_extinguishes_when_usage_stops() {
        let mut fire = FireStateMachine::new();
        fire.ingest(2_500.0, Some(UsageSource::Codex), Instant::now(), true);
        fire.recent_inflows.clear();

        for _ in 0..600 {
            fire.advance_by(1.0);
        }

        let snapshot = fire.snapshot();
        assert_eq!(snapshot.phase, FirePhase::Out);
        assert_eq!(snapshot.intensity, 0.0);
        assert_eq!(snapshot.ember_heat, 0.0);
    }
}
