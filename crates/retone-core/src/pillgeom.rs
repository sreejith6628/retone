//! Floating-button geometry: click regions, status width, keeping it on screen.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Region { Main, Menu }

/// The rightmost 20% of the pill is the ▼ (options) button.
pub fn region(x_in_pill: i32, width: i32) -> Region {
    if x_in_pill * 100 >= width * 80 { Region::Menu } else { Region::Main }
}

pub const PILL_W: u32 = 128;
pub const PILL_H: u32 = 30;
const MAX_W: u32 = 420;

/// Logical width needed to show a status message inside the pill.
pub fn status_width(text: &str) -> u32 {
    (text.chars().count() as u32 * 8 + 48).clamp(PILL_W, MAX_W)
}

/// Keep the pill on a connected screen: clamp it into the screen it overlaps most,
/// or move it to the top-centre of the first screen if it overlaps none.
/// `screens` are (x, y, w, h) work areas in the same coordinate space as `pos`.
pub fn clamp_to_visible(pos: (i32, i32), size: (i32, i32), screens: &[(i32, i32, i32, i32)]) -> (i32, i32) {
    let (x, y) = pos;
    let (w, h) = size;
    let overlap = |&(sx, sy, sw, sh): &(i32, i32, i32, i32)| {
        let ox = (x + w).min(sx + sw) - x.max(sx);
        let oy = (y + h).min(sy + sh) - y.max(sy);
        if ox > 0 && oy > 0 { ox as i64 * oy as i64 } else { 0 }
    };
    let Some(best) = screens.iter().max_by_key(|s| overlap(s)) else { return pos };
    if overlap(best) == 0 {
        let (sx, sy, sw, _) = screens[0];
        return (sx + (sw - w) / 2, sy + 8);
    }
    let (sx, sy, sw, sh) = *best;
    (x.clamp(sx, sx + sw - w), y.clamp(sy, sy + sh - h))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn region_split_at_80_percent() {
        assert_eq!(region(10, 128), Region::Main);
        assert_eq!(region(101, 128), Region::Main);
        assert_eq!(region(103, 128), Region::Menu);
        assert_eq!(region(127, 128), Region::Menu);
    }
    #[test]
    fn status_width_bounds() {
        assert_eq!(status_width("✨ Polish"), 128);
        let w = status_width("⚠ Signed out — open Settings to sign in again");
        assert!(w > 128 && w <= 420, "{w}");
        assert_eq!(status_width(&"x".repeat(500)), 420);
    }
    #[test]
    fn clamp_to_visible_area() {
        let screens = [(0, 0, 1920, 1080)];
        assert_eq!(clamp_to_visible((100, 100), (128, 30), &screens), (100, 100));
        assert_eq!(clamp_to_visible((1900, 1070), (128, 30), &screens), (1792, 1050));
        // centre on a monitor that no longer exists -> top-centre of first screen
        assert_eq!(clamp_to_visible((3000, 200), (128, 30), &screens), (896, 8));
        let two = [(0, 0, 1920, 1080), (1920, 0, 1280, 1024)];
        assert_eq!(clamp_to_visible((2000, 500), (128, 30), &two), (2000, 500));
        assert_eq!(clamp_to_visible((10, 10), (128, 30), &[]), (10, 10));
    }
}
