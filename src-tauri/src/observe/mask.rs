//! Demo mode: with "Show Rune in screen recordings" on, Rune's windows are no longer excluded from
//! capture, so an external recorder (OBS, Game Bar) can see them. The model must still never see
//! Rune's own UI (it could read the pill or click it), so the rectangles of Rune's windows are
//! painted over in the screenshot **before** it is cropped or downscaled. The frame keeps its size,
//! so no coordinate shifts.
//!
//! Rectangles are in physical desktop pixels (the unit the window APIs report). Nothing here touches
//! Tauri; the source of the rectangles is registered once at startup (`set_source`).

use std::sync::Mutex;

use image::RgbaImage;

/// A rectangle on the virtual desktop, physical pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: u32,
    pub h: u32,
}

/// What a masked area is painted with: neutral grey, so it reads as "nothing here".
const FILL: [u8; 4] = [128, 128, 128, 255];

/// Where the overlay's pieces sit, in logical pixels (they scale with the display): the top bar
/// (`top-6`, `h-10`, centred; the width is a ceiling for a long action caption) and the dot frame
/// along every edge (`DEPTH` 4 rows of `STEP` 8). **Estimates from the stylesheet, not measured**:
/// a margin is added so a pixel of the pieces is never left uncovered.
const BAR_TOP: f64 = 24.0 - 4.0;
const BAR_HEIGHT: f64 = 40.0 + 8.0;
const BAR_WIDTH: f64 = 1000.0;
const EDGE_DEPTH: f64 = 4.0 * 8.0 + 4.0;

/// The pieces of the full-screen overlay on a monitor at `scale` (1.0 at 100%): the top bar and the
/// four edge strips. The overlay is click-through and covers the whole screen, so masking all of it
/// would blank the desktop.
pub fn overlay_rects(monitor: Rect, scale: f64) -> Vec<Rect> {
    let px = |logical: f64| (logical * scale).round().max(1.0) as u32;
    let (edge, bar_h, bar_w) = (px(EDGE_DEPTH), px(BAR_HEIGHT), px(BAR_WIDTH).min(monitor.w));
    let bar_x = monitor.x + ((monitor.w - bar_w) / 2) as i32;
    vec![
        Rect {
            x: bar_x,
            y: monitor.y + px(BAR_TOP) as i32,
            w: bar_w,
            h: bar_h,
        },
        Rect {
            x: monitor.x,
            y: monitor.y,
            w: monitor.w,
            h: edge.min(monitor.h),
        },
        Rect {
            x: monitor.x,
            y: monitor.y + monitor.h as i32 - edge.min(monitor.h) as i32,
            w: monitor.w,
            h: edge.min(monitor.h),
        },
        Rect {
            x: monitor.x,
            y: monitor.y,
            w: edge.min(monitor.w),
            h: monitor.h,
        },
        Rect {
            x: monitor.x + monitor.w as i32 - edge.min(monitor.w) as i32,
            y: monitor.y,
            w: edge.min(monitor.w),
            h: monitor.h,
        },
    ]
}

/// `rect` in the pixels of an image whose top-left is at (`origin_x`, `origin_y`) on the desktop,
/// clipped to the image. `None` when nothing of it is inside.
pub fn to_image(
    rect: Rect,
    origin_x: i32,
    origin_y: i32,
    width: u32,
    height: u32,
) -> Option<(u32, u32, u32, u32)> {
    let left = i64::from(rect.x - origin_x).clamp(0, i64::from(width));
    let top = i64::from(rect.y - origin_y).clamp(0, i64::from(height));
    let right = (i64::from(rect.x - origin_x) + i64::from(rect.w)).clamp(0, i64::from(width));
    let bottom = (i64::from(rect.y - origin_y) + i64::from(rect.h)).clamp(0, i64::from(height));
    (right > left && bottom > top).then_some((
        left as u32,
        top as u32,
        (right - left) as u32,
        (bottom - top) as u32,
    ))
}

/// Paints every rectangle grey. The image keeps its size.
pub fn apply(image: &mut RgbaImage, rects: &[Rect], origin_x: i32, origin_y: i32) {
    let (width, height) = (image.width(), image.height());
    for rect in rects {
        let Some((left, top, w, h)) = to_image(*rect, origin_x, origin_y, width, height) else {
            continue;
        };
        for y in top..top + h {
            for x in left..left + w {
                image.put_pixel(x, y, image::Rgba(FILL));
            }
        }
    }
}

type Source = Box<dyn Fn() -> Vec<Rect> + Send + Sync>;
static SOURCE: Mutex<Option<Source>> = Mutex::new(None);

/// Registers where the rectangles to mask come from (set once at startup, by the app).
pub fn set_source(source: impl Fn() -> Vec<Rect> + Send + Sync + 'static) {
    if let Ok(mut slot) = SOURCE.lock() {
        *slot = Some(Box::new(source));
    }
}

/// The rectangles to mask right now: none unless demo mode is on and the app registered a source.
pub fn current() -> Vec<Rect> {
    SOURCE
        .lock()
        .ok()
        .and_then(|slot| slot.as_ref().map(|source| source()))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x: i32, y: i32, w: u32, h: u32) -> Rect {
        Rect { x, y, w, h }
    }

    #[test]
    fn a_rectangle_is_clipped_to_the_image_and_shifted_to_its_origin() {
        assert_eq!(
            to_image(rect(100, 50, 40, 20), 0, 0, 1920, 1080),
            Some((100, 50, 40, 20))
        );
        // A monitor to the left of the primary one starts at -1920.
        assert_eq!(
            to_image(rect(-1900, 10, 100, 30), -1920, 0, 1920, 1080),
            Some((20, 10, 100, 30))
        );
        assert_eq!(
            to_image(rect(1900, 1070, 100, 100), 0, 0, 1920, 1080),
            Some((1900, 1070, 20, 10)),
            "clipped at the corner"
        );
        assert_eq!(
            to_image(rect(-50, -50, 100, 100), 0, 0, 1920, 1080),
            Some((0, 0, 50, 50))
        );
        assert_eq!(
            to_image(rect(2000, 0, 10, 10), 0, 0, 1920, 1080),
            None,
            "entirely outside"
        );
        assert_eq!(
            to_image(rect(0, 0, 0, 10), 0, 0, 1920, 1080),
            None,
            "no width"
        );
    }

    #[test]
    fn applying_paints_only_the_rectangle_and_keeps_the_size() {
        let mut image = RgbaImage::from_pixel(10, 10, image::Rgba([1, 2, 3, 255]));
        apply(&mut image, &[rect(2, 3, 4, 2)], 0, 0);
        assert_eq!((image.width(), image.height()), (10, 10));
        assert_eq!(image.get_pixel(2, 3).0, FILL);
        assert_eq!(image.get_pixel(5, 4).0, FILL);
        assert_eq!(
            image.get_pixel(6, 4).0,
            [1, 2, 3, 255],
            "one past the right edge is untouched"
        );
        assert_eq!(
            image.get_pixel(2, 5).0,
            [1, 2, 3, 255],
            "one past the bottom edge is untouched"
        );
        let painted = image.pixels().filter(|pixel| pixel.0 == FILL).count();
        assert_eq!(painted, 8);
    }

    #[test]
    fn the_overlay_masks_its_bar_and_edges_at_every_display_scale() {
        for (scale, edge, bar_h) in [(1.0, 36, 48), (1.25, 45, 60), (1.5, 54, 72)] {
            let monitor = rect(0, 0, 1920, 1080);
            let pieces = overlay_rects(monitor, scale);
            assert_eq!(pieces.len(), 5);
            assert_eq!(
                (pieces[1].h, pieces[3].w),
                (edge, edge),
                "edge depth at {scale}"
            );
            assert_eq!(pieces[0].h, bar_h, "bar height at {scale}");
            // The bar is centred.
            assert_eq!(pieces[0].x * 2 + pieces[0].w as i32, 1920);
            // The middle of the screen is not masked.
            let mut screen = RgbaImage::from_pixel(1920, 1080, image::Rgba([9, 9, 9, 255]));
            apply(&mut screen, &pieces, 0, 0);
            assert_eq!(screen.get_pixel(960, 540).0, [9, 9, 9, 255]);
            assert_eq!(screen.get_pixel(0, 540).0, FILL);
            assert_eq!(screen.get_pixel(1919, 540).0, FILL);
            assert_eq!(screen.get_pixel(960, 1079).0, FILL);
        }
    }

    #[test]
    fn a_small_monitor_never_gets_a_bar_wider_than_itself() {
        let pieces = overlay_rects(rect(0, 0, 800, 600), 1.0);
        assert_eq!(pieces[0].w, 800);
        assert_eq!(pieces[0].x, 0);
    }
}
