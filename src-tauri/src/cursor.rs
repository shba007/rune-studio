//! The mouse pointer while the agent is in control.
//!
//! The agent moves the real pointer, so the pointer is what the user watches. While it is in
//! control the arrow and the text beam are swapped for dot-matrix ones in the app's look
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
    /// Distance between dots, and the margin before the first one.
    const PITCH: f32 = 3.0;
    const MARGIN: f32 = 2.5;
    const DOT: f32 = 1.4;
    const HALO: f32 = 2.5;
    const RED: [u8; 3] = [0xd7, 0x19, 0x21];
    const WHITE: [u8; 3] = [0xf1, 0xf1, 0xef];

    static APPLIED: AtomicBool = AtomicBool::new(false);

    /// One dot: its grid column and row, and whether it is the red one.
    type Dot = (i32, i32, bool);

    /// The arrow: a triangle of dots with a short tail. The red dot is its tip, the point that clicks.
    fn arrow() -> (Vec<Dot>, (i32, i32)) {
        let mut dots: Vec<Dot> = Vec::new();
        for row in 0..6 {
            for col in 0..=row {
                dots.push((col, row, col == 0 && row == 0));
            }
        }
        for col in 0..3 {
            dots.push((col, 6, false));
        }
        dots.push((2, 7, false));
        dots.push((3, 7, false));
        dots.push((3, 8, false));
        dots.push((4, 8, false));
        (dots, (0, 0))
    }

    /// The text beam: a column of dots with a cap at each end. The red dot marks where text lands.
    fn beam() -> (Vec<Dot>, (i32, i32)) {
        let mut dots: Vec<Dot> = (0..9).map(|row| (1, row, row == 4)).collect();
        for row in [0, 8] {
            dots.push((0, row, false));
            dots.push((2, row, false));
        }
        (dots, (1, 4))
    }

    fn centre(cell: i32) -> f32 {
        MARGIN + cell as f32 * PITCH
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
                for &(col, row, red) in dots {
                    let (cx, cy) = (centre(col), centre(row));
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
        paint(&arrow().0)
    }

    #[cfg(test)]
    pub fn beam_pixels() -> Vec<u8> {
        paint(&beam().0)
    }

    /// Builds a cursor from dots. The system takes ownership of what it is given.
    fn build(dots: &[Dot], hotspot: (i32, i32)) -> Option<HCURSOR> {
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
                xHotspot: centre(hotspot.0).round() as u32,
                yHotspot: centre(hotspot.1).round() as u32,
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
        for (id, (dots, hotspot)) in [(OCR_NORMAL, arrow()), (OCR_IBEAM, beam())] {
            match build(&dots, hotspot) {
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
            assert!(white >= 20, "white dots");
        }
    }
}
