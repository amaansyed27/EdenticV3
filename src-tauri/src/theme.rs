use tauri::{image::Image, Theme, WebviewWindow};

fn icon_for_surface(dark_surface: bool) -> Image<'static> {
    let base = tauri::include_image!("../assets/icon/png/transparent/edentic-icon-128x128.png");
    let mut rgba = base.rgba().to_vec();

    if dark_surface {
        for pixel in rgba.chunks_exact_mut(4) {
            if pixel[3] < 12 {
                continue;
            }

            let red = u16::from(pixel[0]);
            let green = u16::from(pixel[1]);
            let blue = u16::from(pixel[2]);
            let luminance = (red * 54 + green * 183 + blue * 19) / 256;

            if luminance < 170 {
                if blue > red + 18 && blue > green + 10 {
                    pixel[0] = 78;
                    pixel[1] = 142;
                    pixel[2] = 255;
                } else if red > blue + 16 && green > blue + 8 {
                    pixel[0] = 239;
                    pixel[1] = 190;
                    pixel[2] = 72;
                } else {
                    pixel[0] = 244;
                    pixel[1] = 239;
                    pixel[2] = 226;
                }
            }
        }
    }

    Image::new_owned(rgba, base.width(), base.height())
}

pub fn apply_window_theme(window: &WebviewWindow, theme: &str) -> Result<(), String> {
    let dark_surface = match theme {
        "dark" => true,
        "light" => false,
        _ => return Err("Unknown window theme".into()),
    };

    window
        .set_theme(Some(if dark_surface { Theme::Dark } else { Theme::Light }))
        .map_err(|error| format!("Could not update the native window theme: {error}"))?;
    window
        .set_icon(icon_for_surface(dark_surface))
        .map_err(|error| format!("Could not update the Windows icon: {error}"))?;
    Ok(())
}

#[tauri::command]
pub fn sync_window_theme(theme: String, window: WebviewWindow) -> Result<(), String> {
    apply_window_theme(&window, &theme)
}
