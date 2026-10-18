//! The mouse pointer while the agent is in control.
//!
//! The agent moves the real pointer, so the pointer is what the user watches. While it is in
//! control the arrow becomes a small dot orb and the text cursor a line of dots, in the app's look
//! (white dots with a dark halo, one red dot at the point that acts); everything else about the
//! pointer is untouched, and the user's own cursors come back the moment control ends.

#[cfg(windows)]
mod imp {
    use std::sync::atomic::{AtomicBool, Ordering};

    use windows::Win32::{
        Graphics::Gdi::{BI_RGB, BITMAPINFO, BITMAPINFOHEADER, CreateBitmap, CreateDIBSection, DIB_RGB_COLORS, DeleteObject, GetDC, ReleaseDC},
        UI::WindowsAndMessaging::{
            CreateIconIndirect, HCURSOR, ICONINFO, OCR_IBEAM, OCR_NORMAL, SPI_SETCURSORS, SYSTEM_CURSOR_ID, SetSystemCursor, SystemParametersInfoW,
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS,
        },
    };

    const SIZE: i32 = 32;
    const MIDDLE: f32 = SIZE as f32 / 2.0;
    const DOT: f32 = 1.3;
    const HALO: f32 = 2.2;
    const RED: [u8; 3] = [0xd7, 0x19, 0x21];
    const WHITE: [u8; 3] = [0xf1, 0xf1, 0xef];

    static APPLIED: AtomicBool = AtomicBool::new(false);

    /// One dot: where it is in the 32 px image, and whether it is the red one.
    type Dot = (f32, f32, bool);

    /// The pointer is a small orb, like the app's: a red dot in the middle (the point that
    /// clicks) inside two rings of white dots.
    fn arrow() -> Vec<Dot> {
        let mut dots = vec![(MIDDLE, MIDDLE, true)];
        for (radius, count) in [(3.6_f32, 6), (7.2, 12)] {
            for index in 0..count {
                let angle = (index as f32 / count as f32) * std::f32::consts::TAU - std::f32::consts::FRAC_PI_2;
                dots.push((MIDDLE + radius * angle.cos(), MIDDLE + radius * angle.sin(), false));
            }
        }
        dots
    }

    /// The text cursor is a line of five dots with a red one in the middle, where the text lands.
    fn beam() -> Vec<Dot> {
        (-2..=2).map(|index| (MIDDLE, MIDDLE + index as f32 * 4.5, index == 0)).collect()
    }

    /// How much of the pixel at (px, py) a dot of `radius` centred at (cx, cy) covers.
    fn coverage(px: f32, py: f32, cx: f32, cy: f32, radius: f32) -> f32 {
        let distance = ((px + 0.5 - cx).powi(2) + (py + 0.5 - cy).powi(2)).sqrt();
        (radius + 0.5 - distance).clamp(0.0, 1.0)
    }

    /// Paints the dots into a 32x32 BGRA buffer (straight alpha).
    fn paint(dots: &[Dot]) -> Vec<u8> {
        let mut pixels = vec![0u8; (SIZE * SIZE * 4) as usize];
        for py in 0..SIZE {
            for px in 0..SIZE {
                let (mut alpha, mut colour) = (0.0f32, [0.0f32; 3]);
                for &(cx, cy, red) in dots {
                    // A soft dark halo under each dot keeps the pointer visible on white pages.
                    let halo = coverage(px as f32, py as f32, cx, cy, HALO) * 0.55;
                    if halo > 0.0 {
                        let out = halo + alpha * (1.0 - halo);
                        for channel in colour.iter_mut() {
                            *channel = *channel * alpha * (1.0 - halo) / out.max(f32::EPSILON);
                        }
                        alpha = out;
                    }
                    let dot = coverage(px as f32, py as f32, cx, cy, DOT);
                    if dot > 0.0 {
                        let source = if red { RED } else { WHITE };
                        let out = dot + alpha * (1.0 - dot);
                        for (index, channel) in colour.iter_mut().enumerate() {
                            *channel = (f32::from(source[index]) * dot + *channel * alpha * (1.0 - dot)) / out.max(f32::EPSILON);
                        }
                        alpha = out;
                    }
                }
                let at = ((py * SIZE + px) * 4) as usize;
                pixels[at] = colour[2].round() as u8;
                pixels[at + 1] = colour[1].round() as u8;
                pixels[at + 2] = colour[0].round() as u8;
                pixels[at + 3] = (alpha * 255.0).round() as u8;
            }
        }
        pixels
    }

    #[cfg(test)]
    pub fn arrow_pixels() -> Vec<u8> {
        paint(&arrow())
    }

    #[cfg(test)]
    pub fn beam_pixels() -> Vec<u8> {
        paint(&beam())
    }

    /// Builds a cursor from dots. The system takes ownership of what it is given.
    fn build(dots: &[Dot]) -> Option<HCURSOR> {
        let pixels = paint(dots);
        unsafe {
            let info = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: SIZE,
                    biHeight: -SIZE,
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB.0,
                    ..Default::default()
                },
                ..Default::default()
            };
            let screen = GetDC(None);
            let mut bits = std::ptr::null_mut();
            let colour = CreateDIBSection(Some(screen), &info, DIB_RGB_COLORS, &mut bits, None, 0);
            ReleaseDC(None, screen);
            let colour = colour.ok()?;
            std::ptr::copy_nonoverlapping(pixels.as_ptr(), bits.cast::<u8>(), pixels.len());
            let mask = CreateBitmap(SIZE, SIZE, 1, 1, None);
            let icon = ICONINFO {
                fIcon: false.into(),
                xHotspot: MIDDLE as u32,
                yHotspot: MIDDLE as u32,
                hbmMask: mask,
                hbmColor: colour,
            };
            let made = CreateIconIndirect(&icon);
            let _ = DeleteObject(colour.into());
            let _ = DeleteObject(mask.into());
            made.ok().map(|icon| HCURSOR(icon.0))
        }
    }

    pub fn apply() {
        if APPLIED.swap(true, Ordering::SeqCst) {
            return;
        }
        for (id, dots) in [(OCR_NORMAL, arrow()), (OCR_IBEAM, beam())] {
            match build(&dots) {
                Some(cursor) => unsafe {
                    if let Err(err) = SetSystemCursor(cursor, SYSTEM_CURSOR_ID(id.0)) {
                        log::warn!("could not set the agent cursor: {err}");
                    }
                },
                None => log::warn!("could not build the agent cursor"),
            }
        }
    }

    /// Puts the user's own cursors back. With `force` it also runs when nothing was applied, which
    /// heals a pointer left changed by an earlier run that did not shut down cleanly.
    pub fn restore(force: bool) {
        if !APPLIED.swap(false, Ordering::SeqCst) && !force {
            return;
        }
        unsafe {
            if let Err(err) = SystemParametersInfoW(SPI_SETCURSORS, 0, None, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0)) {
                log::warn!("could not restore the cursors: {err}");
            }
        }
    }
}

#[cfg(not(windows))]
mod imp {
    pub fn apply() {}
    pub fn restore(_force: bool) {}
}

/// Swaps in the agent's pointer. Does nothing if it is already in.
pub fn apply() {
    imp::apply();
}

/// Gives the user's pointer back. Does nothing if the agent's was never in.
pub fn restore() {
    imp::restore(false);
}

/// Called once at start-up and at exit: always puts the user's pointer back.
pub fn reset() {
    imp::restore(true);
}

#[cfg(all(test, windows))]
mod tests {
    use super::imp::*;

    #[test]
    fn the_arrow_has_a_red_tip_and_the_beam_a_red_middle() {
        let (arrow_pixels, beam_pixels) = (arrow_pixels(), beam_pixels());
        for pixels in [arrow_pixels, beam_pixels] {
            let red = pixels.chunks(4).filter(|p| p[3] > 200 && p[2] > 180 && p[1] < 60).count();
            let white = pixels.chunks(4).filter(|p| p[3] > 200 && p[0] > 200 && p[1] > 200).count();
            assert!(red >= 4, "a red dot");
            assert!(white >= 12, "white dots");
        }
    }
}
