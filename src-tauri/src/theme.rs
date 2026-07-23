use tauri::{image::Image, Theme, WebviewWindow};

fn icon_for_surface(dark_surface: bool) -> Image<'static> {
    let base = tauri::include_image!("../assets/icon/png/transparent/edentic-icon-128x128.png");
    let source = base.rgba();
    let width = base.width() as usize;
    let height = base.height() as usize;
    let mut rgba = source.to_vec();

    let outline = if dark_surface {
        [255, 246, 224, 220]
    } else {
        [28, 29, 32, 178]
    };
    let radius: isize = if dark_surface { 2 } else { 1 };

    for y in 0..height {
        for x in 0..width {
            let index = (y * width + x) * 4;
            if source[index + 3] >= 18 {
                continue;
            }

            let mut touches_logo = false;
            'neighbours: for dy in -radius..=radius {
                for dx in -radius..=radius {
                    if dx == 0 && dy == 0 {
                        continue;
                    }

                    let nx = x as isize + dx;
                    let ny = y as isize + dy;
                    if nx < 0 || ny < 0 || nx >= width as isize || ny >= height as isize {
                        continue;
                    }

                    let neighbour = (ny as usize * width + nx as usize) * 4;
                    if source[neighbour + 3] >= 48 {
                        touches_logo = true;
                        break 'neighbours;
                    }
                }
            }

            if touches_logo {
                rgba[index] = outline[0];
                rgba[index + 1] = outline[1];
                rgba[index + 2] = outline[2];
                rgba[index + 3] = outline[3];
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
