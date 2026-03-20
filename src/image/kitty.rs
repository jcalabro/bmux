use base64::Engine;

/// Encode an image for the Kitty graphics protocol.
#[allow(dead_code)]
pub fn encode_kitty(img: &image::DynamicImage, max_width: u16, max_height: u16) -> String {
    let resized = img.resize(
        max_width as u32 * 8,
        max_height as u32 * 16,
        image::imageops::FilterType::Triangle,
    );

    let rgba = resized.to_rgba8();
    let (w, h) = rgba.dimensions();

    let raw_data = rgba.into_raw();
    let encoded = base64::engine::general_purpose::STANDARD.encode(&raw_data);

    // Kitty protocol: split into 4096-byte chunks.
    let mut output = String::new();
    let chunks: Vec<&str> = encoded
        .as_bytes()
        .chunks(4096)
        .map(|c| std::str::from_utf8(c).unwrap_or(""))
        .collect();

    for (i, chunk) in chunks.iter().enumerate() {
        let is_last = i == chunks.len() - 1;
        let more = if is_last { 0 } else { 1 };

        if i == 0 {
            // First chunk includes image metadata.
            output.push_str(&format!(
                "\x1b_Gf=32,s={},v={},m={};{}\x1b\\",
                w, h, more, chunk
            ));
        } else {
            output.push_str(&format!("\x1b_Gm={};{}\x1b\\", more, chunk));
        }
    }

    output
}
