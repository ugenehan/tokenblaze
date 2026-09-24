//! Shared RGBA compositor used by the native and Tauri flame windows.

use super::FireSnapshot;
use super::{CANVAS_PADDING, FIRE_HEIGHT, FIRE_WIDTH, LOG_HEIGHT, LOG_WIDTH};

/// Compose the flame, its ambient glow, and the crossed log bed into a canvas.
pub fn compose_campfire_frame(
    fire_pixels: &[u8],
    render_width: usize,
    render_height: usize,
    snapshot: FireSnapshot,
) -> Vec<u8> {
    let pixel_count = render_width.saturating_mul(render_height);
    let mut frame = vec![0; pixel_count.saturating_mul(4)];
    if render_width == 0
        || render_height == 0
        || fire_pixels.len() < FIRE_WIDTH.saturating_mul(FIRE_HEIGHT).saturating_mul(4)
    {
        return frame;
    }

    let base_w = (FIRE_WIDTH.max(LOG_WIDTH) + CANVAS_PADDING) as f32;
    let base_h = (FIRE_HEIGHT + LOG_HEIGHT + CANVAS_PADDING) as f32;
    let scale = (render_width as f32 / base_w).min(render_height as f32 / base_h);
    let content_w = base_w * scale;
    let content_h = base_h * scale;
    let offset_x = (render_width as f32 - content_w) * 0.5;
    let offset_y = (render_height as f32 - content_h) * 0.5;
    let flame_x = offset_x + (base_w - FIRE_WIDTH as f32) * 0.5 * scale;
    let log_y = offset_y + (base_h - LOG_HEIGHT as f32) * scale;
    let flame_y = offset_y
        + (base_h - LOG_HEIGHT as f32 - FIRE_HEIGHT as f32 + LOG_HEIGHT as f32 * 0.5) * scale;
    let flame_w = FIRE_WIDTH as f32 * scale;
    let flame_h = FIRE_HEIGHT as f32 * scale;

    let glow_color = representative_fire_color(fire_pixels);
    let glow_strength =
        (snapshot.intensity as f32 * 0.22 + snapshot.ember_heat as f32 * 0.12).clamp(0.0, 0.36);
    draw_radial_glow(
        &mut frame,
        render_width,
        render_height,
        render_width as f32 * 0.5,
        log_y - LOG_HEIGHT as f32 * scale * 0.1,
        flame_w * 0.64,
        flame_h * 0.48,
        glow_color,
        glow_strength,
    );
    draw_smooth_logs(
        &mut frame,
        render_width,
        render_height,
        render_width as f32 * 0.5,
        log_y + LOG_HEIGHT as f32 * scale * 0.45,
        LOG_WIDTH as f32 * scale,
        LOG_HEIGHT as f32 * scale,
        snapshot.ember_heat as f32,
    );
    draw_scaled_fire(
        &mut frame,
        render_width,
        render_height,
        fire_pixels,
        FIRE_WIDTH,
        FIRE_HEIGHT,
        flame_x,
        flame_y,
        flame_w,
        flame_h,
    );
    frame
}

fn representative_fire_color(fire_pixels: &[u8]) -> [u8; 3] {
    let mut color = [255_u64, 120, 40];
    let mut weight = 1_u64;
    for pixel in fire_pixels.as_chunks::<4>().0 {
        let alpha = u64::from(pixel[3]);
        if alpha < 32 {
            continue;
        }
        color[0] += u64::from(pixel[0]) * alpha;
        color[1] += u64::from(pixel[1]) * alpha;
        color[2] += u64::from(pixel[2]) * alpha;
        weight += alpha;
    }
    [
        (color[0] / weight).min(255) as u8,
        (color[1] / weight).min(255) as u8,
        (color[2] / weight).min(255) as u8,
    ]
}

#[allow(clippy::too_many_arguments)]
fn draw_radial_glow(
    frame: &mut [u8],
    width: usize,
    height: usize,
    center_x: f32,
    center_y: f32,
    radius_x: f32,
    radius_y: f32,
    color: [u8; 3],
    strength: f32,
) {
    if radius_x <= 0.0 || radius_y <= 0.0 || strength <= 0.0 {
        return;
    }
    let min_x = (center_x - radius_x).floor().max(0.0) as usize;
    let max_x = (center_x + radius_x).ceil().min(width as f32 - 1.0) as usize;
    let min_y = (center_y - radius_y).floor().max(0.0) as usize;
    let max_y = (center_y + radius_y).ceil().min(height as f32 - 1.0) as usize;
    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let dx = (x as f32 + 0.5 - center_x) / radius_x;
            let dy = (y as f32 + 0.5 - center_y) / radius_y;
            let falloff = (1.0 - (dx * dx + dy * dy).sqrt()).clamp(0.0, 1.0);
            blend_rgba(
                frame,
                width,
                height,
                x as i32,
                y as i32,
                [color[0], color[1], color[2], 255],
                falloff * falloff * strength,
            );
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_smooth_logs(
    frame: &mut [u8],
    width: usize,
    height: usize,
    center_x: f32,
    center_y: f32,
    log_width: f32,
    log_height: f32,
    ember_heat: f32,
) {
    let half = log_width * 0.36;
    let rise = log_height * 0.34;
    let radius = (log_height * 0.34).max(2.0);
    let ember = ember_heat.clamp(0.0, 1.0);
    let edge = [55, 24, 14, 238];
    let wood = [
        (112.0 + ember * 32.0) as u8,
        (48.0 + ember * 18.0) as u8,
        (24.0 + ember * 5.0) as u8,
        250,
    ];
    let highlight = [
        (174.0 + ember * 54.0) as u8,
        (72.0 + ember * 52.0) as u8,
        (30.0 + ember * 18.0) as u8,
        (150.0 + ember * 70.0) as u8,
    ];

    for (start_y, end_y) in [
        (center_y + rise, center_y - rise),
        (center_y - rise, center_y + rise),
    ] {
        draw_capsule(
            frame,
            width,
            height,
            center_x - half,
            start_y,
            center_x + half,
            end_y,
            radius + 1.4,
            edge,
        );
        draw_capsule(
            frame,
            width,
            height,
            center_x - half,
            start_y,
            center_x + half,
            end_y,
            radius,
            wood,
        );
    }

    let streak_half = half * 0.62;
    draw_capsule(
        frame,
        width,
        height,
        center_x - streak_half,
        center_y - rise * 0.62,
        center_x + streak_half,
        center_y + rise * 0.62,
        (radius * 0.16).max(0.7),
        highlight,
    );
}

#[allow(clippy::too_many_arguments)]
fn draw_capsule(
    frame: &mut [u8],
    width: usize,
    height: usize,
    start_x: f32,
    start_y: f32,
    end_x: f32,
    end_y: f32,
    radius: f32,
    color: [u8; 4],
) {
    let min_x = (start_x.min(end_x) - radius - 1.0).floor().max(0.0) as usize;
    let max_x = (start_x.max(end_x) + radius + 1.0)
        .ceil()
        .min(width as f32 - 1.0) as usize;
    let min_y = (start_y.min(end_y) - radius - 1.0).floor().max(0.0) as usize;
    let max_y = (start_y.max(end_y) + radius + 1.0)
        .ceil()
        .min(height as f32 - 1.0) as usize;
    let segment_x = end_x - start_x;
    let segment_y = end_y - start_y;
    let segment_len_sq = segment_x * segment_x + segment_y * segment_y;

    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let point_x = x as f32 + 0.5;
            let point_y = y as f32 + 0.5;
            let projection = if segment_len_sq > 0.0 {
                (((point_x - start_x) * segment_x + (point_y - start_y) * segment_y)
                    / segment_len_sq)
                    .clamp(0.0, 1.0)
            } else {
                0.0
            };
            let nearest_x = start_x + segment_x * projection;
            let nearest_y = start_y + segment_y * projection;
            let distance = (point_x - nearest_x).hypot(point_y - nearest_y);
            let coverage = (radius + 0.5 - distance).clamp(0.0, 1.0);
            blend_rgba(frame, width, height, x as i32, y as i32, color, coverage);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_scaled_fire(
    frame: &mut [u8],
    frame_width: usize,
    frame_height: usize,
    source: &[u8],
    source_width: usize,
    source_height: usize,
    dest_x: f32,
    dest_y: f32,
    dest_width: f32,
    dest_height: f32,
) {
    let start_x = dest_x.floor().max(0.0) as usize;
    let end_x = (dest_x + dest_width).ceil().min(frame_width as f32) as usize;
    let start_y = dest_y.floor().max(0.0) as usize;
    let end_y = (dest_y + dest_height).ceil().min(frame_height as f32) as usize;

    for y in start_y..end_y {
        for x in start_x..end_x {
            let source_x = ((x as f32 + 0.5 - dest_x) / dest_width * source_width as f32 - 0.5)
                .clamp(0.0, source_width.saturating_sub(1) as f32);
            let source_y = ((y as f32 + 0.5 - dest_y) / dest_height * source_height as f32 - 0.5)
                .clamp(0.0, source_height.saturating_sub(1) as f32);
            let color =
                sample_bilinear_rgba(source, source_width, source_height, source_x, source_y);
            blend_rgba(
                frame,
                frame_width,
                frame_height,
                x as i32,
                y as i32,
                color,
                1.0,
            );
        }
    }
}

fn sample_bilinear_rgba(source: &[u8], width: usize, height: usize, x: f32, y: f32) -> [u8; 4] {
    let x0 = x.floor() as usize;
    let y0 = y.floor() as usize;
    let x1 = (x0 + 1).min(width - 1);
    let y1 = (y0 + 1).min(height - 1);
    let tx = x - x0 as f32;
    let ty = y - y0 as f32;
    let samples = [
        (x0, y0, (1.0 - tx) * (1.0 - ty)),
        (x1, y0, tx * (1.0 - ty)),
        (x0, y1, (1.0 - tx) * ty),
        (x1, y1, tx * ty),
    ];
    let mut alpha = 0.0_f32;
    let mut premultiplied = [0.0_f32; 3];
    for (sample_x, sample_y, weight) in samples {
        let index = (sample_y * width + sample_x) * 4;
        let sample_alpha = source[index + 3] as f32 / 255.0;
        alpha += sample_alpha * weight;
        for channel in 0..3 {
            premultiplied[channel] += source[index + channel] as f32 * sample_alpha * weight;
        }
    }
    if alpha <= f32::EPSILON {
        return [0, 0, 0, 0];
    }
    [
        (premultiplied[0] / alpha).round().clamp(0.0, 255.0) as u8,
        (premultiplied[1] / alpha).round().clamp(0.0, 255.0) as u8,
        (premultiplied[2] / alpha).round().clamp(0.0, 255.0) as u8,
        (alpha * 255.0).round().clamp(0.0, 255.0) as u8,
    ]
}

fn blend_rgba(
    frame: &mut [u8],
    width: usize,
    height: usize,
    x: i32,
    y: i32,
    color: [u8; 4],
    coverage: f32,
) {
    if x < 0 || y < 0 || x >= width as i32 || y >= height as i32 || coverage <= 0.0 {
        return;
    }
    let index = (y as usize * width + x as usize) * 4;
    let source_alpha = color[3] as f32 / 255.0 * coverage.clamp(0.0, 1.0);
    let dest_alpha = frame[index + 3] as f32 / 255.0;
    let output_alpha = source_alpha + dest_alpha * (1.0 - source_alpha);
    if output_alpha <= 0.0 {
        return;
    }
    for channel in 0..3 {
        frame[index + channel] = ((color[channel] as f32 * source_alpha
            + frame[index + channel] as f32 * dest_alpha * (1.0 - source_alpha))
            / output_alpha)
            .round()
            .clamp(0.0, 255.0) as u8;
    }
    frame[index + 3] = (output_alpha * 255.0).round().clamp(0.0, 255.0) as u8;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fire::{FirePhase, FireTier, FlameColorMix};

    #[test]
    fn compositor_preserves_canvas_size_and_transparency() {
        let fire = vec![0; FIRE_WIDTH * FIRE_HEIGHT * 4];
        let snapshot = FireSnapshot {
            intensity: 0.0,
            fuel: 0.0,
            ember_heat: 0.0,
            phase: FirePhase::Out,
            spark_burst: 0.0,
            tier: FireTier::Hush,
            color_mix: FlameColorMix::CLASSIC,
        };
        let frame = compose_campfire_frame(&fire, 160, 220, snapshot);
        assert_eq!(frame.len(), 160 * 220 * 4);
        assert_eq!(frame[3], 0);
    }
}
