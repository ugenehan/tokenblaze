//! Flame colors — per-source tints and the live mix.

use crate::data::UsageSource;
use once_cell::sync::Lazy;
use std::sync::RwLock;

/// Weighted mix of source colors — what tints the live flame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FlameColorMix {
    pub weights: [f64; 7], // indexed by UsageSource discriminant
}

impl FlameColorMix {
    pub const CLASSIC: FlameColorMix = FlameColorMix { weights: [0.0; 7] };

    pub fn is_classic(self) -> bool {
        self.weights.iter().all(|&w| w < 0.01)
    }

    pub fn from_weights_array(weights: [f64; 7]) -> Self {
        FlameColorMix { weights }
    }

    /// Compute the mixed RGB color at a given heat level (0…1).
    ///
    /// Classic orange when no sources are active,
    /// otherwise a weighted mix of the per-source palettes.
    pub fn heat_to_rgb(self, heat: f32) -> [u8; 3] {
        if self.is_classic() {
            classic_palette(heat)
        } else {
            mixed_palette(heat, &self.weights)
        }
    }
}

impl Default for FlameColorMix {
    fn default() -> Self {
        Self::CLASSIC
    }
}

/// Per-source flame color presets.
///
/// Each tool gets a distinct "flavor" of flame so users can tell
/// at a glance what's been burning.
pub struct SourceFlameColors;

static SOURCE_COLORS: Lazy<RwLock<[[u8; 3]; 7]>> =
    Lazy::new(|| RwLock::new(SourceFlameColors::BUILT_IN));

impl SourceFlameColors {
    /// Accent colors used until the user customizes a source.
    pub const BUILT_IN: [[u8; 3]; 7] = [
        [232, 120, 46],  // Claude Code: orange
        [71, 184, 107],  // Codex: green
        [82, 143, 235],  // Cursor: blue
        [184, 107, 242], // Grok: violet
        [242, 158, 56],  // Pi: amber
        [51, 199, 199],  // Amp: teal
        [236, 72, 153],  // OpenCode: rose
    ];

    pub fn set_all(colors: [[u8; 3]; 7]) {
        if let Ok(mut current) = SOURCE_COLORS.write() {
            *current = colors;
        }
    }

    pub fn set(source: UsageSource, color: [u8; 3]) {
        if let Ok(mut colors) = SOURCE_COLORS.write() {
            colors[source.index()] = color;
        }
    }

    pub fn accent_color(source: UsageSource) -> [u8; 3] {
        SOURCE_COLORS
            .read()
            .map(|colors| colors[source.index()])
            .unwrap_or(Self::BUILT_IN[source.index()])
    }

    /// Base color for a source — the "hottest" point of its flame.
    pub fn hot_color(source: UsageSource) -> [u8; 3] {
        let [r, g, b] = Self::accent_color(source);
        [lift(r, 0.58), lift(g, 0.52), lift(b, 0.42)]
    }

    /// Mid color for a source — the body of the flame.
    pub fn mid_color(source: UsageSource) -> [u8; 3] {
        Self::accent_color(source)
    }

    /// Cool color for a source — the base near the logs.
    pub fn cool_color(source: UsageSource) -> [u8; 3] {
        let [r, g, b] = Self::accent_color(source);
        [scale(r, 0.48), scale(g, 0.40), scale(b, 0.43)]
    }
}

fn scale(channel: u8, factor: f32) -> u8 {
    (channel as f32 * factor).round().clamp(0.0, 255.0) as u8
}

fn lift(channel: u8, amount: f32) -> u8 {
    (channel as f32 + (255.0 - channel as f32) * amount)
        .round()
        .clamp(0.0, 255.0) as u8
}

// ── palette computation ────────────────────────────────────────

/// Classic orange flame palette — the default look.
fn classic_palette(heat: f32) -> [u8; 3] {
    let h = heat.clamp(0.0, 1.0);
    if h < 0.15 {
        // Dark ember → deep red
        lerp_rgb([20, 5, 0], [80, 20, 5], h / 0.15)
    } else if h < 0.35 {
        // Deep red → orange-red
        lerp_rgb([80, 20, 5], [200, 70, 15], (h - 0.15) / 0.20)
    } else if h < 0.60 {
        // Orange-red → bright orange
        lerp_rgb([200, 70, 15], [255, 160, 40], (h - 0.35) / 0.25)
    } else if h < 0.85 {
        // Orange → yellow
        lerp_rgb([255, 160, 40], [255, 230, 120], (h - 0.60) / 0.25)
    } else {
        // Yellow → white hot
        lerp_rgb([255, 230, 120], [255, 255, 240], (h - 0.85) / 0.15)
    }
}

/// Weighted mix of all active source palettes.
fn mixed_palette(heat: f32, weights: &[f64; 7]) -> [u8; 3] {
    let mut total_weight = 0.0f64;
    let mut r = 0.0f64;
    let mut g = 0.0f64;
    let mut b = 0.0f64;

    for (i, &w) in weights.iter().enumerate() {
        if w <= 0.001 {
            continue;
        }
        let src = UsageSource::from_index(i);
        let col = source_palette_at(src, heat);
        r += col[0] as f64 * w;
        g += col[1] as f64 * w;
        b += col[2] as f64 * w;
        total_weight += w;
    }

    if total_weight < 0.001 {
        return classic_palette(heat);
    }

    let inv = 1.0 / total_weight;
    [
        (r * inv).clamp(0.0, 255.0) as u8,
        (g * inv).clamp(0.0, 255.0) as u8,
        (b * inv).clamp(0.0, 255.0) as u8,
    ]
}

fn source_palette_at(source: UsageSource, heat: f32) -> [u8; 3] {
    let cool = SourceFlameColors::cool_color(source);
    let mid = SourceFlameColors::mid_color(source);
    let hot = SourceFlameColors::hot_color(source);

    let h = heat.clamp(0.0, 1.0);
    if h < 0.4 {
        lerp_rgb(cool, mid, h / 0.4)
    } else if h < 0.8 {
        lerp_rgb(mid, hot, (h - 0.4) / 0.4)
    } else {
        // white-hot tip
        lerp_rgb(hot, [255, 255, 245], (h - 0.8) / 0.2)
    }
}

fn lerp_rgb(a: [u8; 3], b: [u8; 3], t: f32) -> [u8; 3] {
    let t = t.clamp(0.0, 1.0);
    [
        (a[0] as f32 + (b[0] as f32 - a[0] as f32) * t) as u8,
        (a[1] as f32 + (b[1] as f32 - a[1] as f32) * t) as u8,
        (a[2] as f32 + (b[2] as f32 - a[2] as f32) * t) as u8,
    ]
}
