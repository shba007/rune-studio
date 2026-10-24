#[cfg(windows)]
mod imp {
    use std::sync::atomic::{AtomicBool, Ordering};

    use windows::Win32::{
        Graphics::Gdi::{
            BI_RGB, BITMAPINFO, BITMAPINFOHEADER, CreateBitmap, CreateDIBSection, DIB_RGB_COLORS,
            DeleteObject, GetDC, ReleaseDC,
        },
        UI::WindowsAndMessaging::{
            CreateIconIndirect, HCURSOR, ICONINFO, OCR_IBEAM, OCR_NORMAL, SPI_SETCURSORS,
            SYSTEM_CURSOR_ID, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS, SetSystemCursor,
            SystemParametersInfoW,
        },
    };

    const SIZE: i32 = 32;
    static APPLIED: AtomicBool = AtomicBool::new(false);
    static IMAGES: std::sync::Mutex<Vec<super::CursorImage>> = std::sync::Mutex::new(Vec::new());

    pub fn set_images(images: Vec<super::CursorImage>) {
        *IMAGES
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = images;
    }

    fn build(image: &super::CursorImage) -> Option<HCURSOR> {
        let mut pixels = image.rgba.clone();
        for pixel in pixels.as_chunks_mut::<4>().0 {
            pixel.swap(0, 2);
            for channel in 0..3 {
                pixel[channel] = (u16::from(pixel[channel]) * u16::from(pixel[3]) / 255) as u8;
            }
        }
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
                xHotspot: image.hotspot[0],
                yHotspot: image.hotspot[1],
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
        let images = IMAGES
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if images.is_empty() || APPLIED.swap(true, Ordering::SeqCst) {
            return;
        }
        for image in images.iter() {
            let id = match image.kind.as_str() {
                "default" => OCR_NORMAL,
                "text" => OCR_IBEAM,
                "link" => windows::Win32::UI::WindowsAndMessaging::OCR_HAND,
                "wait" => windows::Win32::UI::WindowsAndMessaging::OCR_WAIT,
                "not-allowed" => windows::Win32::UI::WindowsAndMessaging::OCR_NO,
                _ => continue,
            };
            match build(image) {
                Some(cursor) => unsafe {
                    if let Err(err) = SetSystemCursor(cursor, SYSTEM_CURSOR_ID(id.0)) {
                        log::warn!("could not set the agent cursor: {err}");
                    }
                },
                None => log::warn!(
                    "could not build the agent cursor from public/cursor/{}",
                    image.kind
                ),
            }
        }
    }

    pub fn restore(force: bool) {
        if !APPLIED.swap(false, Ordering::SeqCst) && !force {
            return;
        }
        unsafe {
            if let Err(err) = SystemParametersInfoW(
                SPI_SETCURSORS,
                0,
                None,
                SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
            ) {
                log::warn!("could not restore the cursors: {err}");
            }
        }
    }
}

#[cfg(not(windows))]
mod imp {
    pub fn apply() {}
    pub fn set_images(_images: Vec<super::CursorImage>) {}
    pub fn restore(_force: bool) {}
}

pub fn apply() {
    imp::apply();
}

pub fn restore() {
    imp::restore(false);
}

pub fn reset() {
    imp::restore(true);
}

#[derive(serde::Deserialize)]
pub struct CursorImage {
    kind: String,
    hotspot: [u32; 2],
    rgba: Vec<u8>,
}

fn validate_images(images: &[CursorImage]) -> Result<(), String> {
    let kinds = ["default", "link", "text", "wait", "not-allowed"];
    if images.len() != kinds.len()
        || kinds
            .iter()
            .any(|kind| images.iter().filter(|i| i.kind == *kind).count() != 1)
        || images
            .iter()
            .any(|i| i.rgba.len() != 32 * 32 * 4 || i.hotspot.iter().any(|p| *p >= 32))
    {
        return Err("Invalid cursor assets".into());
    }
    Ok(())
}

#[tauri::command]
pub fn set_cursor_images(app: tauri::AppHandle, images: Vec<CursorImage>) -> Result<(), String> {
    validate_images(&images)?;
    imp::set_images(images);
    if crate::overlay::has_control(crate::events::agent_state(&app)) {
        imp::apply();
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cursor_assets_require_all_kinds_bounded_pixels_and_valid_hotspots() {
        let mut images: Vec<_> = ["default", "link", "text", "wait", "not-allowed"]
            .into_iter()
            .map(|kind| CursorImage {
                kind: kind.into(),
                hotspot: [1, 1],
                rgba: vec![0; 32 * 32 * 4],
            })
            .collect();
        assert!(validate_images(&images).is_ok());
        images[0].rgba.pop();
        assert!(validate_images(&images).is_err());
        images[0].rgba.push(0);
        images[0].hotspot = [32, 1];
        assert!(validate_images(&images).is_err());
        images[0].hotspot = [1, 1];
        images[0].kind = "text".into();
        assert!(validate_images(&images).is_err());
    }
}
