/// Encode an image as Sixel graphics.
/// This is a simplified encoder; a production version would use a proper library.
#[allow(dead_code)]
pub fn encode_sixel(img: &image::DynamicImage, max_width: u16, max_height: u16) -> String {
    let resized = img.resize(
        max_width as u32 * 8, // rough char-to-pixel
        max_height as u32 * 16,
        image::imageops::FilterType::Triangle,
    );

    let rgb = resized.to_rgb8();
    let (w, h) = rgb.dimensions();

    let mut output = String::new();

    // Sixel start sequence: DCS P1 ; P2 ; P3 q
    output.push_str("\x1bPq");

    // Quantize to 16 colors for simplicity.
    // Define color palette.
    let palette = [
        (0, 0, 0),
        (100, 0, 0),
        (0, 100, 0),
        (100, 100, 0),
        (0, 0, 100),
        (100, 0, 100),
        (0, 100, 100),
        (100, 100, 100),
        (50, 50, 50),
        (100, 50, 50),
        (50, 100, 50),
        (100, 100, 50),
        (50, 50, 100),
        (100, 50, 100),
        (50, 100, 100),
        (100, 100, 100),
    ];

    for (i, (r, g, b)) in palette.iter().enumerate() {
        output.push_str(&format!("#{};2;{};{};{}", i, r, g, b));
    }

    // Encode pixels in sixel rows (6 pixels high each).
    for row_start in (0..h).step_by(6) {
        for color_idx in 0..16u8 {
            output.push_str(&format!("#{}", color_idx));
            for x in 0..w {
                let mut sixel_byte = 0u8;
                for bit in 0..6u32 {
                    let y = row_start + bit;
                    if y < h {
                        let pixel = rgb.get_pixel(x, y);
                        let nearest = nearest_palette_color(pixel[0], pixel[1], pixel[2], &palette);
                        if nearest == color_idx {
                            sixel_byte |= 1 << bit;
                        }
                    }
                }
                output.push((sixel_byte + 63) as char);
            }
            output.push('$'); // carriage return
        }
        output.push('-'); // newline
    }

    // Sixel end sequence: ST
    output.push_str("\x1b\\");

    output
}

fn nearest_palette_color(r: u8, g: u8, b: u8, palette: &[(i32, i32, i32)]) -> u8 {
    let r = r as i32 * 100 / 255;
    let g = g as i32 * 100 / 255;
    let b = b as i32 * 100 / 255;

    let mut best_idx = 0u8;
    let mut best_dist = i32::MAX;

    for (i, &(pr, pg, pb)) in palette.iter().enumerate() {
        let dist = (r - pr).pow(2) + (g - pg).pow(2) + (b - pb).pow(2);
        if dist < best_dist {
            best_dist = dist;
            best_idx = i as u8;
        }
    }

    best_idx
}
