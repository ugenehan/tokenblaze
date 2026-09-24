//! Pixel-art campfire logs matching the original TokenBlaze campfire sprite.

use super::engine::{LOG_HEIGHT, LOG_WIDTH, VISUAL_DETAIL_SCALE};

const BASE_LOG_WIDTH: usize = 28;
const BASE_LOG_HEIGHT: usize = 12;
type Color = [u8; 4];

const LOG_DARK: Color = [62, 34, 14, 255];
const LOG_MID: Color = [110, 68, 28, 255];
const LOG_LIGHT: Color = [148, 98, 44, 255];
const LOG_END: Color = [186, 148, 88, 255];
const ASH: Color = [78, 74, 70, 255];
const COAL: Color = [28, 24, 20, 255];
const EMBER: Color = [220, 48, 8, 255];

pub struct PixelCampfireAtlas;

impl PixelCampfireAtlas {
    /// Render the original flat log bed at the simulation's detail scale.
    pub fn render_log_rgba(ember_brightness: f32) -> Vec<u8> {
        let mut base = [[0_u8; 4]; BASE_LOG_WIDTH * BASE_LOG_HEIGHT];
        stamp(&mut base, 8, 4..=23, ASH);
        stamp(&mut base, 9, 3..=24, COAL);
        stamp(&mut base, 10, 5..=22, ASH);
        stamp(&mut base, 11, 7..=20, COAL);
        put(&mut base, 8, 9, EMBER);
        put(&mut base, 14, 9, EMBER);
        put(&mut base, 19, 9, EMBER);

        stamp(&mut base, 6, 2..=25, LOG_DARK);
        stamp(&mut base, 7, 2..=25, LOG_MID);
        put(&mut base, 2, 6, LOG_END);
        put(&mut base, 2, 7, LOG_END);
        put(&mut base, 25, 6, LOG_END);
        put(&mut base, 25, 7, LOG_LIGHT);

        stamp(&mut base, 4, 3..=24, LOG_LIGHT);
        stamp(&mut base, 5, 3..=24, LOG_MID);
        put(&mut base, 3, 4, LOG_END);
        put(&mut base, 3, 5, LOG_END);
        put(&mut base, 24, 4, LOG_END);
        put(&mut base, 24, 5, LOG_LIGHT);

        let mut pixels = vec![0; LOG_WIDTH * LOG_HEIGHT * 4];
        for y in 0..LOG_HEIGHT {
            for x in 0..LOG_WIDTH {
                let mut color =
                    base[(y / VISUAL_DETAIL_SCALE) * BASE_LOG_WIDTH + x / VISUAL_DETAIL_SCALE];
                if color == EMBER {
                    let brightness = ember_brightness.clamp(0.0, 1.0);
                    color[0] = (color[0] as f32 * (0.65 + brightness * 0.35)) as u8;
                    color[1] = (color[1] as f32 * (0.35 + brightness * 0.65)) as u8;
                }
                let index = (y * LOG_WIDTH + x) * 4;
                pixels[index..index + 4].copy_from_slice(&color);
            }
        }
        pixels
    }
}

fn put(sprite: &mut [Color], x: usize, y: usize, color: Color) {
    if x < BASE_LOG_WIDTH && y < BASE_LOG_HEIGHT {
        sprite[y * BASE_LOG_WIDTH + x] = color;
    }
}

fn stamp(sprite: &mut [Color], row: usize, cols: std::ops::RangeInclusive<usize>, color: Color) {
    for x in cols {
        put(sprite, x, row, color);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_log_matches_original_sprite_dimensions() {
        let pixels = PixelCampfireAtlas::render_log_rgba(0.5);
        assert_eq!(LOG_WIDTH, BASE_LOG_WIDTH * VISUAL_DETAIL_SCALE);
        assert_eq!(LOG_HEIGHT, BASE_LOG_HEIGHT * VISUAL_DETAIL_SCALE);
        assert_eq!(pixels.len(), LOG_WIDTH * LOG_HEIGHT * 4);
        assert!(pixels
            .chunks_exact(4)
            .any(|pixel| pixel[0] > 140 && pixel[1] > 10 && pixel[2] == 8));
    }
}
