//! The one place that knows how a point in the model's coordinate space maps to a real pixel.
//!
//! **Model space** is Qwen-VL's convention: both axes run `0..=1000` across the full width and
//! height of the monitor image the model was shown. It does not depend on how far the screenshot
//! was downscaled, only on the monitor's shape.
//!
//! **Pixels** are physical desktop pixels (the unit `SendInput` and the cursor use), which can be
//! negative for a monitor placed left of or above the primary one.
//!
//! Nothing else in the app converts between the two.

use serde::Serialize;

/// Both model axes run from 0 to this value.
pub const MODEL_MAX: f64 = 1000.0;

/// A monitor's placement on the virtual desktop, in physical pixels.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Monitor {
    pub id: u32,
    /// Left edge on the virtual desktop. Negative when the monitor sits left of the primary one.
    pub x: i32,
    /// Top edge on the virtual desktop. Negative when the monitor sits above the primary one.
    pub y: i32,
    pub width: u32,
    pub height: u32,
    /// OS scale factor (1.0 at 100%, 1.25 at 125%...). Informational: geometry is already physical.
    pub scale_factor: f32,
    pub primary: bool,
}

impl Monitor {
    /// Whether a physical desktop pixel lies on this monitor.
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn contains(&self, px: i32, py: i32) -> bool {
        px >= self.x
            && py >= self.y
            && px < self.x + self.width as i32
            && py < self.y + self.height as i32
    }
}

/// Model-space point to the physical desktop pixel it denotes on `monitor`.
///
/// Out-of-range input is clamped, so a slightly wrong model answer (`1003`, `-4`) still lands on
/// the monitor edge instead of on another monitor or off screen.
pub fn model_to_pixels(monitor: &Monitor, x: f64, y: f64) -> (i32, i32) {
    (
        monitor.x + axis_to_pixel(x, monitor.width),
        monitor.y + axis_to_pixel(y, monitor.height),
    )
}

/// The model-space point at the centre of a physical desktop pixel on `monitor`.
/// The exact inverse of [`model_to_pixels`] for every pixel of the monitor.
/// Used by tests now and by UI Automation element ids from M4.
#[cfg_attr(not(test), allow(dead_code))]
pub fn pixels_to_model(monitor: &Monitor, px: i32, py: i32) -> (f64, f64) {
    (
        pixel_to_axis(px - monitor.x, monitor.width),
        pixel_to_axis(py - monitor.y, monitor.height),
    )
}

fn axis_to_pixel(value: f64, extent: u32) -> i32 {
    let clamped = if value.is_nan() { 0.0 } else { value.clamp(0.0, MODEL_MAX) };
    let pixel = (clamped / MODEL_MAX * f64::from(extent)).floor() as i64;
    pixel.clamp(0, i64::from(extent) - 1) as i32
}

#[cfg_attr(not(test), allow(dead_code))]
fn pixel_to_axis(offset: i32, extent: u32) -> f64 {
    (f64::from(offset) + 0.5) / f64::from(extent) * MODEL_MAX
}

/// Size of a screenshot after downscaling so that its longer edge is at most `max_edge`.
/// Never upscales, and keeps the aspect ratio.
pub fn fit_within(width: u32, height: u32, max_edge: u32) -> (u32, u32) {
    let longest = width.max(height);
    if longest <= max_edge || longest == 0 {
        return (width, height);
    }
    let scale = f64::from(max_edge) / f64::from(longest);
    let scaled = |edge: u32| ((f64::from(edge) * scale).round() as u32).max(1);
    (scaled(width), scaled(height))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn monitor(x: i32, y: i32, width: u32, height: u32, scale_factor: f32) -> Monitor {
        Monitor { id: 1, x, y, width, height, scale_factor, primary: x == 0 && y == 0 }
    }

    /// Representative layouts: plain 1080p, then physical sizes at 125% and 150% scaling,
    /// a 4K panel, and monitors placed left of and above the primary one.
    fn layouts() -> Vec<Monitor> {
        vec![
            monitor(0, 0, 1920, 1080, 1.0),
            monitor(0, 0, 2560, 1440, 1.25),
            monitor(0, 0, 3840, 2160, 1.5),
            monitor(-1920, 0, 1920, 1080, 1.0),
            monitor(1920, -300, 2560, 1440, 1.25),
            monitor(-2560, -1440, 2560, 1440, 1.5),
        ]
    }

    #[test]
    fn corners_map_to_the_monitor_corners() {
        for m in layouts() {
            assert_eq!(model_to_pixels(&m, 0.0, 0.0), (m.x, m.y), "{m:?}");
            assert_eq!(
                model_to_pixels(&m, 1000.0, 1000.0),
                (m.x + m.width as i32 - 1, m.y + m.height as i32 - 1),
                "{m:?}"
            );
        }
    }

    #[test]
    fn centre_lands_on_the_middle_pixel() {
        for m in layouts() {
            let (px, py) = model_to_pixels(&m, 500.0, 500.0);
            assert_eq!(px, m.x + (m.width / 2) as i32, "{m:?}");
            assert_eq!(py, m.y + (m.height / 2) as i32, "{m:?}");
        }
    }

    #[test]
    fn every_pixel_round_trips() {
        for m in layouts() {
            // Every column and every row, checked independently (the axes do not interact).
            for dx in 0..m.width as i32 {
                let (mx, _) = pixels_to_model(&m, m.x + dx, m.y);
                assert_eq!(model_to_pixels(&m, mx, 0.0).0, m.x + dx, "column {dx} on {m:?}");
            }
            for dy in 0..m.height as i32 {
                let (_, my) = pixels_to_model(&m, m.x, m.y + dy);
                assert_eq!(model_to_pixels(&m, 0.0, my).1, m.y + dy, "row {dy} on {m:?}");
            }
        }
    }

    #[test]
    fn a_click_never_leaves_its_monitor() {
        let probes = [-50.0, -0.0001, 0.0, 0.5, 333.3, 999.999, 1000.0, 1000.0001, 5000.0, f64::NAN];
        for m in layouts() {
            for &x in &probes {
                for &y in &probes {
                    let (px, py) = model_to_pixels(&m, x, y);
                    assert!(m.contains(px, py), "({x}, {y}) escaped {m:?} as ({px}, {py})");
                }
            }
        }
    }

    #[test]
    fn a_monitor_to_the_left_gets_negative_pixels() {
        let left = monitor(-1920, 0, 1920, 1080, 1.0);
        let (px, py) = model_to_pixels(&left, 500.0, 500.0);
        assert!(px < 0 && py >= 0, "({px}, {py})");
        assert!(left.contains(px, py));
        assert!(!monitor(0, 0, 1920, 1080, 1.0).contains(px, py));
    }

    #[test]
    fn mapping_is_monotonic() {
        let m = monitor(0, 0, 2560, 1440, 1.25);
        let mut last = -1;
        for step in 0..=1000 {
            let (px, _) = model_to_pixels(&m, f64::from(step), 0.0);
            assert!(px >= last, "x went backwards at {step}");
            last = px;
        }
    }

    #[test]
    fn downscale_keeps_aspect_ratio_and_never_upscales() {
        assert_eq!(fit_within(1920, 1080, 1280), (1280, 720));
        assert_eq!(fit_within(2560, 1440, 1280), (1280, 720));
        assert_eq!(fit_within(3840, 2160, 1280), (1280, 720));
        assert_eq!(fit_within(1080, 1920, 1280), (720, 1280));
        assert_eq!(fit_within(1000, 600, 1280), (1000, 600));
        assert_eq!(fit_within(1280, 800, 1280), (1280, 800));
        assert_eq!(fit_within(0, 0, 1280), (0, 0));
    }

    #[test]
    fn model_coordinates_do_not_depend_on_the_downscale() {
        // The same model point must hit the same physical pixel whatever size the model saw.
        let m = monitor(0, 0, 3840, 2160, 1.5);
        let (sw, sh) = fit_within(m.width, m.height, 1280);
        // A point at 40% across, 70% down in the *scaled* image, expressed in model space...
        let model = (0.4 * 1000.0, 0.7 * 1000.0);
        // ...is 40% / 70% of the *physical* monitor too.
        let (px, py) = model_to_pixels(&m, model.0, model.1);
        assert_eq!((px, py), ((0.4 * f64::from(m.width)) as i32, (0.7 * f64::from(m.height)) as i32));
        assert_eq!((sw, sh), (1280, 720));
    }
}
