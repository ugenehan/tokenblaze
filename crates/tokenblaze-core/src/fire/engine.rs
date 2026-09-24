//! Heat-field fire engine.
//!
//! A compact heat field cools as it rises and spreads horizontally. The
//! renderer softens that field before applying the active color palette so
//! the floating flame keeps fluid edges at every window size.
//!
//! Reference id: https://fabiensanglard.net/doom_fire_psx/index.html

use rand::Rng;

use super::palette::FlameColorMix;
use super::{FirePhase, FireSnapshot, FireTier};

/// Internal detail multiplier. The panel keeps the same physical size while
/// the fire, logs, and overlay render with twice as many pixels per axis.
pub const VISUAL_DETAIL_SCALE: usize = 2;
/// Width of the fire simulation in pixels.
pub const FIRE_WIDTH: usize = 28 * VISUAL_DETAIL_SCALE;
/// Height of the fire simulation in pixels.
pub const FIRE_HEIGHT: usize = 36 * VISUAL_DETAIL_SCALE;
/// Width of the log sprite in pixels.
pub const LOG_WIDTH: usize = 28 * VISUAL_DETAIL_SCALE;
/// Height of the log sprite in pixels.
pub const LOG_HEIGHT: usize = 12 * VISUAL_DETAIL_SCALE;
/// Transparent padding around the composited campfire.
pub const CANVAS_PADDING: usize = 8 * VISUAL_DETAIL_SCALE;

pub struct FireEngine {
    width: usize,
    height: usize,
    /// Heat buffer — 0 = cold, 255 = hottest.
    heat: Vec<u8>,
    /// Color palette (256 RGBA entries, rebuilt when colors change).
    palette: Vec<[u8; 4]>,
    /// Output RGBA frame buffer.
    frame: Vec<u8>,

    // state
    intensity: f32,
    tier: FireTier,
    phase: FirePhase,
    ember_heat: f32,
    color_mix: FlameColorMix,
    palette_epoch: u64,

    // particles
    sparks: Vec<Spark>,
    spark_burst: f32,
    reduce_motion: bool,

    rng: rand::rngs::StdRng,
}

#[derive(Debug, Clone, Copy)]
struct Spark {
    x: f32,
    y: f32,
    vy: f32,
    vx: f32,
    life: f32,
    max_life: f32,
}

impl FireEngine {
    pub fn new() -> Self {
        let width = FIRE_WIDTH;
        let height = FIRE_HEIGHT;
        let heat = vec![0u8; width * height];
        let palette = build_palette(FlameColorMix::CLASSIC);
        let frame = vec![0u8; width * height * 4];
        let rng = rand::SeedableRng::from_rng(rand::thread_rng())
            .unwrap_or_else(|_| rand::SeedableRng::seed_from_u64(0xC0FFEE));

        FireEngine {
            width,
            height,
            heat,
            palette,
            frame,
            intensity: 0.0,
            tier: FireTier::Hush,
            phase: FirePhase::Unlit,
            ember_heat: 0.0,
            color_mix: FlameColorMix::CLASSIC,
            palette_epoch: 0,
            sparks: Vec::new(),
            spark_burst: 0.0,
            reduce_motion: false,
            rng,
        }
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    /// Update simulation state from a fire snapshot.
    pub fn apply(&mut self, snap: FireSnapshot, reduce_motion: bool, palette_epoch: u64) {
        self.intensity = snap.intensity as f32;
        self.tier = snap.tier;
        self.phase = snap.phase;
        self.ember_heat = snap.ember_heat as f32;
        self.spark_burst = snap.spark_burst as f32;
        self.reduce_motion = reduce_motion;

        if self.color_mix != snap.color_mix || self.palette_epoch != palette_epoch {
            self.color_mix = snap.color_mix;
            self.palette_epoch = palette_epoch;
            self.palette = build_palette(snap.color_mix);
        }
    }

    /// Advance the simulation by one step.
    pub fn step(&mut self) {
        match self.phase {
            FirePhase::Unlit | FirePhase::Out => {
                // Cool everything down
                for h in self.heat.iter_mut() {
                    *h = h.saturating_sub(8);
                }
                self.update_sparks();
            }
            FirePhase::Ember => {
                for _ in 0..VISUAL_DETAIL_SCALE {
                    self.propagate_up();
                    self.ignite_ember_base();
                    self.apply_height_cap();
                }
                self.update_sparks();
            }
            FirePhase::Flame => {
                for _ in 0..VISUAL_DETAIL_SCALE {
                    self.propagate_up();
                    self.ignite_base();
                    self.apply_height_cap();
                }
                self.spawn_sparks();
                self.update_sparks();
            }
        }
    }

    /// Render current state into the frame buffer as RGBA.
    ///
    /// Returns a reference to the internal RGBA pixel buffer.
    pub fn render(&mut self) -> &[u8] {
        let w = self.width;
        let h = self.height;

        for y in 0..h {
            for x in 0..w {
                // A small Gaussian kernel removes the grid-shaped edge left by
                // the discrete simulation without blurring away its motion.
                let mut weighted_heat = 0.0_f32;
                let mut total_weight = 0.0_f32;
                for offset_y in -1_i32..=1 {
                    let sample_y = (y as i32 + offset_y).clamp(0, h as i32 - 1) as usize;
                    for offset_x in -1_i32..=1 {
                        let sample_x = (x as i32 + offset_x).clamp(0, w as i32 - 1) as usize;
                        let weight = match (offset_x.abs(), offset_y.abs()) {
                            (0, 0) => 4.0,
                            (1, 1) => 1.0,
                            _ => 2.0,
                        };
                        weighted_heat += self.heat[sample_y * w + sample_x] as f32 * weight;
                        total_weight += weight;
                    }
                }
                let heat = weighted_heat / total_weight;
                let px_idx = (y * w + x) * 4;
                let lower = heat.floor().clamp(0.0, 255.0) as usize;
                let upper = (lower + 1).min(255);
                let mix = heat - lower as f32;
                for channel in 0..3 {
                    self.frame[px_idx + channel] = (self.palette[lower][channel] as f32
                        * (1.0 - mix)
                        + self.palette[upper][channel] as f32 * mix)
                        .round() as u8;
                }

                let normalized = heat / 255.0;
                let edge = smoothstep(0.012, 0.12, normalized).powf(0.72);
                self.frame[px_idx + 3] = (edge * 255.0).round().clamp(0.0, 255.0) as u8;
            }
        }

        // Draw sparks on top
        self.draw_sparks();

        &self.frame
    }

    // ── internal: heat field ──────────────────────────────────

    fn ignite_base(&mut self) {
        let w = self.width;
        let bottom = (self.height - 1) * w;
        self.heat[bottom..bottom + w].fill(0);

        let half_width = match self.tier {
            FireTier::Hush => 2,
            FireTier::Glow => 3,
            FireTier::Crackle => 5,
            FireTier::Roar => 7,
            FireTier::Blaze => 10,
        } * VISUAL_DETAIL_SCALE;
        let base_peak = match self.tier {
            FireTier::Hush => 18.0,
            FireTier::Glow => 22.0,
            FireTier::Crackle => 26.0,
            FireTier::Roar => 30.0,
            FireTier::Blaze => 32.0,
        };
        let peak = (base_peak * (0.55 + self.intensity * 0.55)).min(32.0);
        let peak = (peak * (255.0 / 32.0)) as i32;
        let center = (w / 2) as i32;

        for dx in -(half_width as i32)..=half_width as i32 {
            let x = center + dx;
            if !(0..w as i32).contains(&x) {
                continue;
            }
            let edge_penalty = if dx.unsigned_abs() as usize == half_width {
                24
            } else {
                0
            };
            let mut heat = peak - dx.abs() * 8 - edge_penalty;
            heat += self.rng.gen_range(-12..=12);
            if matches!(self.tier, FireTier::Roar | FireTier::Blaze)
                && self.rng.gen_range(0..=8) == 0
            {
                heat += 48;
            }
            self.heat[bottom + x as usize] = heat.clamp(0, 255) as u8;
        }

        // Give the higher-resolution flame a deep, solid base rather than a
        // single noisy row that reads as disconnected dots.
        for depth in 1..=(VISUAL_DETAIL_SCALE * 2 - 1) {
            if depth >= self.height || half_width <= depth {
                break;
            }
            let row = (self.height - 1 - depth) * w;
            let row_half_width = half_width - depth;
            let depth_falloff = depth as i32 * 18;
            for dx in -(row_half_width as i32)..=(row_half_width as i32) {
                let x = center + dx;
                if !(0..w as i32).contains(&x) {
                    continue;
                }
                let boost = (peak - depth_falloff - dx.abs() * 7).clamp(0, 255) as u8;
                self.heat[row + x as usize] = self.heat[row + x as usize].max(boost);
            }
        }
    }

    fn ignite_ember_base(&mut self) {
        let w = self.width;
        let bottom = (self.height - 1) * w;
        self.heat[bottom..bottom + w].fill(0);
        let count = (3 + (self.ember_heat * 4.0) as usize) * VISUAL_DETAIL_SCALE;
        let start = w / 2 - count / 2;

        for x in start..(start + count).min(w) {
            if self.rng.gen_range(0..=2) == 0 {
                let pulse = ((10.0 + self.ember_heat * 12.0) * (255.0 / 32.0)) as i32
                    + self.rng.gen_range(-24..=24);
                self.heat[bottom + x] = pulse.clamp(0, 255) as u8;
            }
        }
    }

    /// Classic Doom fire propagation: each pixel cools as it rises.
    fn propagate_up(&mut self) {
        let w = self.width;
        let h = self.height;

        // Update from the tip downwards. This ensures heat rises only one row
        // per step; walking bottom-up would repeatedly propagate freshly
        // written values through the entire field in a single frame.
        let mut next_row = vec![0u8; w];
        for y in 0..h - 1 {
            next_row.fill(0);
            for x in 0..w {
                let below = self.heat[(y + 1) * w + x] as i32;

                // Random horizontal spread
                let spread = if self.reduce_motion {
                    1
                } else {
                    self.rng.gen_range(0..3)
                };
                let target_x = match spread {
                    0 => x.saturating_sub(1),
                    1 => x,
                    _ => (x + 1).min(w - 1),
                };

                let decay = match self.tier {
                    FireTier::Hush => self.rng.gen_range(8..=16),
                    FireTier::Glow | FireTier::Crackle => self.rng.gen_range(4..=12),
                    FireTier::Roar => self.rng.gen_range(4..=8),
                    FireTier::Blaze => self.rng.gen_range(0..=8),
                };
                let new_heat = (below - decay).max(0) as u8;
                next_row[target_x] = next_row[target_x].max(new_heat);
                next_row[x] = next_row[x].max(new_heat.saturating_sub(8));
            }
            self.heat[y * w..(y + 1) * w].copy_from_slice(&next_row);
        }
    }

    fn apply_height_cap(&mut self) {
        let max_rows = match self.phase {
            FirePhase::Unlit | FirePhase::Out => 0,
            FirePhase::Ember => 4 * VISUAL_DETAIL_SCALE,
            FirePhase::Flame => match self.tier {
                FireTier::Hush => 8 * VISUAL_DETAIL_SCALE,
                FireTier::Glow => 14 * VISUAL_DETAIL_SCALE,
                FireTier::Crackle => 22 * VISUAL_DETAIL_SCALE,
                FireTier::Roar => 30 * VISUAL_DETAIL_SCALE,
                FireTier::Blaze => self.height,
            },
        };
        let cut = self.height.saturating_sub(max_rows);

        for y in 0..cut {
            let distance = cut - y;
            for x in 0..self.width {
                let heat = &mut self.heat[y * self.width + x];
                if distance > 2 {
                    *heat = 0;
                } else {
                    *heat /= (4 - distance) as u8;
                }
            }
        }
    }

    // ── internal: sparks ──────────────────────────────────────

    fn spawn_sparks(&mut self) {
        let base_count = match self.tier {
            FireTier::Hush => 0,
            FireTier::Glow => 1,
            FireTier::Crackle => 3,
            FireTier::Roar => 6,
            FireTier::Blaze => 10,
        };

        let target = if self.reduce_motion {
            base_count / 2
        } else {
            base_count + (self.spark_burst * 3.0) as usize
        };

        let target = target.min(20);

        while self.sparks.len() < target {
            let w = self.width as f32;
            let x = w / 2.0 + self.rng.gen_range(-w * 0.35..w * 0.35);
            self.sparks.push(Spark {
                x,
                y: self.height as f32 - 2.0,
                vy: self.rng.gen_range(-3.0..-1.6),
                vx: self.rng.gen_range(-0.45..0.45),
                life: 0.0,
                max_life: self.rng.gen_range(0.6..1.4),
            });
        }
    }

    fn update_sparks(&mut self) {
        let h = self.height as f32;
        let dt = if self.reduce_motion { 0.5 } else { 1.0 };

        self.sparks.retain_mut(|s| {
            s.x += s.vx * dt;
            s.y += s.vy * dt;
            s.life += (1.0 / 12.0) * dt;
            s.vy += if self.reduce_motion { 0.04 } else { 0.06 }; // gravity
            s.life < s.max_life && s.y < h && s.y > 0.0
        });
    }

    fn draw_sparks(&mut self) {
        let w = self.width;
        let h = self.height;

        for spark in &self.sparks {
            let px = spark.x as i32;
            let py = spark.y as i32;
            if px < 0 || px >= w as i32 || py < 0 || py >= h as i32 {
                continue;
            }

            let life_frac = 1.0 - spark.life / spark.max_life;
            let alpha = (life_frac * 255.0) as u8;

            // Bright yellow-white spark
            let idx = (py as usize * w + px as usize) * 4;
            if idx + 3 < self.frame.len() {
                // Blend additively
                let existing = &mut self.frame[idx..idx + 4];
                existing[0] = existing[0].saturating_add((255.0 * life_frac) as u8);
                existing[1] = existing[1].saturating_add((220.0 * life_frac) as u8);
                existing[2] = existing[2].saturating_add((120.0 * life_frac) as u8);
                existing[3] = existing[3].max(alpha);
            }

            // A dim trailing pixel keeps sparks readable at the finer
            // simulation resolution without turning them into large blocks.
            let tail_y = py + 1;
            if tail_y < h as i32 {
                let tail_idx = (tail_y as usize * w + px as usize) * 4;
                let tail = &mut self.frame[tail_idx..tail_idx + 4];
                tail[0] = tail[0].saturating_add((180.0 * life_frac) as u8);
                tail[1] = tail[1].saturating_add((90.0 * life_frac) as u8);
                tail[2] = tail[2].saturating_add((30.0 * life_frac) as u8);
                tail[3] = tail[3].max(alpha / 2);
            }
        }
    }
}

fn smoothstep(edge_start: f32, edge_end: f32, value: f32) -> f32 {
    let t = ((value - edge_start) / (edge_end - edge_start)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

impl Default for FireEngine {
    fn default() -> Self {
        Self::new()
    }
}

fn build_palette(mix: FlameColorMix) -> Vec<[u8; 4]> {
    let mut pal = Vec::with_capacity(256);
    for i in 0..=255u8 {
        let heat = i as f32 / 255.0;
        let rgb = mix.heat_to_rgb(heat);
        pal.push([rgb[0], rgb[1], rgb[2], 255]);
    }
    pal
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::UsageSource;
    use crate::fire::SourceFlameColors;

    #[test]
    fn palette_epoch_applies_edited_source_color() {
        let source = UsageSource::Codex;
        let original = SourceFlameColors::accent_color(source);
        let mut engine = FireEngine::new();
        let snapshot = FireSnapshot {
            intensity: 0.5,
            fuel: 0.5,
            ember_heat: 0.5,
            phase: FirePhase::Flame,
            spark_burst: 0.0,
            tier: FireTier::Crackle,
            color_mix: FlameColorMix::from_weights_array([0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0]),
        };

        engine.apply(snapshot, false, 0);
        let before = engine.palette[128];
        SourceFlameColors::set(source, [12, 220, 45]);
        engine.apply(snapshot, false, 1);
        let after = engine.palette[128];
        SourceFlameColors::set(source, original);

        assert_ne!(before, after);
    }
}
