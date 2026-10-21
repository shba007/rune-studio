use serde::Serialize;

use crate::model::profile::PointingConvention;

pub const MODEL_MAX: f64 = 1000.0;

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Monitor {
    pub id: u32,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub scale_factor: f32,
    pub primary: bool,
}

impl Monitor {
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn contains(&self, px: i32, py: i32) -> bool {
        px >= self.x
            && py >= self.y
            && px < self.x + self.width as i32
            && py < self.y + self.height as i32
    }
}

pub fn model_to_pixels(monitor: &Monitor, x: f64, y: f64) -> (i32, i32) {
    (
        monitor.x + axis_to_pixel(x, monitor.width),
        monitor.y + axis_to_pixel(y, monitor.height),
    )
}

#[cfg_attr(not(test), allow(dead_code))]
pub fn pixels_to_model(monitor: &Monitor, px: i32, py: i32) -> (f64, f64) {
    (
        pixel_to_axis(px - monitor.x, monitor.width),
        pixel_to_axis(py - monitor.y, monitor.height),
    )
}

fn axis_to_pixel(value: f64, extent: u32) -> i32 {
    let clamped = if value.is_nan() {
        0.0
    } else {
        value.clamp(0.0, MODEL_MAX)
    };
    let pixel = (clamped / MODEL_MAX * f64::from(extent)).floor() as i64;
    pixel.clamp(0, i64::from(extent) - 1) as i32
}

#[cfg_attr(not(test), allow(dead_code))]
fn pixel_to_axis(offset: i32, extent: u32) -> f64 {
    (f64::from(offset) + 0.5) / f64::from(extent) * MODEL_MAX
}

pub fn fit_within(width: u32, height: u32, max_edge: u32) -> (u32, u32) {
    let longest = width.max(height);
    if longest <= max_edge || longest == 0 {
        return (width, height);
    }
    let scale = f64::from(max_edge) / f64::from(longest);
    let scaled = |edge: u32| ((f64::from(edge) * scale).round() as u32).max(1);
    (scaled(width), scaled(height))
}

pub fn adapt_point_to_norm1000(
    convention: PointingConvention,
    x: f64,
    y: f64,
    width: u32,
    height: u32,
) -> (f64, f64) {
    if x.is_nan() || y.is_nan() || width == 0 || height == 0 {
        return (0.0, 0.0);
    }
    match convention {
        PointingConvention::Norm1000 => (x.clamp(0.0, MODEL_MAX), y.clamp(0.0, MODEL_MAX)),
        PointingConvention::Norm1 => (
            (x * MODEL_MAX).clamp(0.0, MODEL_MAX),
            (y * MODEL_MAX).clamp(0.0, MODEL_MAX),
        ),
        PointingConvention::Pixels => {
            let nx = (x / f64::from(width) * MODEL_MAX).clamp(0.0, MODEL_MAX);
            let ny = (y / f64::from(height) * MODEL_MAX).clamp(0.0, MODEL_MAX);
            (nx, ny)
        }
        PointingConvention::Unknown => {
            if x <= 1.0 && y <= 1.0 && (x > 0.0 || y > 0.0) {
                (
                    (x * MODEL_MAX).clamp(0.0, MODEL_MAX),
                    (y * MODEL_MAX).clamp(0.0, MODEL_MAX),
                )
            } else if x <= MODEL_MAX && y <= MODEL_MAX {
                (x.clamp(0.0, MODEL_MAX), y.clamp(0.0, MODEL_MAX))
            } else {
                let nx = (x / f64::from(width) * MODEL_MAX).clamp(0.0, MODEL_MAX);
                let ny = (y / f64::from(height) * MODEL_MAX).clamp(0.0, MODEL_MAX);
                (nx, ny)
            }
        }
    }
}

pub fn adapt_box_to_norm1000(
    convention: PointingConvention,
    b: [f64; 4],
    width: u32,
    height: u32,
) -> [f64; 4] {
    let (x0, y0) = adapt_point_to_norm1000(convention, b[0], b[1], width, height);
    let (x1, y1) = adapt_point_to_norm1000(convention, b[2], b[3], width, height);
    [x0.min(x1), y0.min(y1), x0.max(x1), y0.max(y1)]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn monitor(x: i32, y: i32, width: u32, height: u32, scale_factor: f32) -> Monitor {
        Monitor {
            id: 1,
            x,
            y,
            width,
            height,
            scale_factor,
            primary: x == 0 && y == 0,
        }
    }

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
            for dx in 0..m.width as i32 {
                let (mx, _) = pixels_to_model(&m, m.x + dx, m.y);
                assert_eq!(
                    model_to_pixels(&m, mx, 0.0).0,
                    m.x + dx,
                    "column {dx} on {m:?}"
                );
            }
            for dy in 0..m.height as i32 {
                let (_, my) = pixels_to_model(&m, m.x, m.y + dy);
                assert_eq!(
                    model_to_pixels(&m, 0.0, my).1,
                    m.y + dy,
                    "row {dy} on {m:?}"
                );
            }
        }
    }

    #[test]
    fn a_click_never_leaves_its_monitor() {
        let probes = [
            -50.0,
            -0.0001,
            0.0,
            0.5,
            333.3,
            999.999,
            1000.0,
            1000.0001,
            5000.0,
            f64::NAN,
        ];
        for m in layouts() {
            for &x in &probes {
                for &y in &probes {
                    let (px, py) = model_to_pixels(&m, x, y);
                    assert!(
                        m.contains(px, py),
                        "({x}, {y}) escaped {m:?} as ({px}, {py})"
                    );
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
        let m = monitor(0, 0, 3840, 2160, 1.5);
        let (sw, sh) = fit_within(m.width, m.height, 1280);
        let model = (0.4 * 1000.0, 0.7 * 1000.0);
        let (px, py) = model_to_pixels(&m, model.0, model.1);
        assert_eq!(
            (px, py),
            (
                (0.4 * f64::from(m.width)) as i32,
                (0.7 * f64::from(m.height)) as i32
            )
        );
        assert_eq!((sw, sh), (1280, 720));
    }

    #[test]
    fn adapt_norm1000_keeps_fixed_numbers() {
        let (x, y) =
            adapt_point_to_norm1000(PointingConvention::Norm1000, 500.0, 250.0, 1920, 1080);
        assert_eq!((x, y), (500.0, 250.0));

        let clamped =
            adapt_point_to_norm1000(PointingConvention::Norm1000, -10.0, 1200.0, 1920, 1080);
        assert_eq!(clamped, (0.0, 1000.0));
    }

    #[test]
    fn adapt_norm1_scales_to_norm1000() {
        let (x, y) = adapt_point_to_norm1000(PointingConvention::Norm1, 0.5, 0.25, 1920, 1080);
        assert_eq!((x, y), (500.0, 250.0));

        let corners = adapt_point_to_norm1000(PointingConvention::Norm1, 1.0, 1.0, 2560, 1440);
        assert_eq!(corners, (1000.0, 1000.0));
    }

    #[test]
    fn adapt_pixels_on_non_square_screen_at_different_scales() {
        // 100% scale: 1920x1080 (16:9 non-square)
        let (x100, y100) =
            adapt_point_to_norm1000(PointingConvention::Pixels, 960.0, 540.0, 1920, 1080);
        assert_eq!((x100, y100), (500.0, 500.0));

        // 125% scale: 2560x1440 (physical pixels)
        let (x125, y125) =
            adapt_point_to_norm1000(PointingConvention::Pixels, 1280.0, 720.0, 2560, 1440);
        assert_eq!((x125, y125), (500.0, 500.0));

        // 150% scale: 3840x2160 (physical pixels)
        let (x150, y150) =
            adapt_point_to_norm1000(PointingConvention::Pixels, 1920.0, 1080.0, 3840, 2160);
        assert_eq!((x150, y150), (500.0, 500.0));

        // Non-center arbitrary point on 1920x1080: (480, 270) -> (250, 250)
        let (x_arb, y_arb) =
            adapt_point_to_norm1000(PointingConvention::Pixels, 480.0, 270.0, 1920, 1080);
        assert_eq!((x_arb, y_arb), (250.0, 250.0));
    }

    #[test]
    fn adapt_unknown_heuristic_selects_correct_space() {
        // 0..1 range
        assert_eq!(
            adapt_point_to_norm1000(PointingConvention::Unknown, 0.4, 0.6, 1920, 1080),
            (400.0, 600.0)
        );

        // 0..1000 range
        assert_eq!(
            adapt_point_to_norm1000(PointingConvention::Unknown, 500.0, 300.0, 1920, 1080),
            (500.0, 300.0)
        );

        // Pixel range > 1000
        assert_eq!(
            adapt_point_to_norm1000(PointingConvention::Unknown, 1440.0, 540.0, 1920, 1080),
            (750.0, 500.0)
        );
    }

    #[test]
    fn adapt_box_to_norm1000_works_for_all_conventions() {
        let b_pixels = [480.0, 270.0, 1440.0, 810.0];
        let adapted = adapt_box_to_norm1000(PointingConvention::Pixels, b_pixels, 1920, 1080);
        assert_eq!(adapted, [250.0, 250.0, 750.0, 750.0]);

        let b_norm1 = [0.1, 0.2, 0.8, 0.9];
        let adapted_norm1 = adapt_box_to_norm1000(PointingConvention::Norm1, b_norm1, 1920, 1080);
        assert_eq!(adapted_norm1, [100.0, 200.0, 800.0, 900.0]);
    }
}
