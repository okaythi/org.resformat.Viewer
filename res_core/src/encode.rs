use crate::{ResHeader, color::rgb_to_ycocg_r, predict::paeth_predict};
use std::io::Write;
use flate2::write::ZlibEncoder;
use flate2::Compression;

pub fn encode_rgb_to_res(width: u32, height: u32, rgb_data: &[u8]) -> std::io::Result<Vec<u8>> {
    let mut residuals = Vec::with_capacity((width * height * 6) as usize);
    let w = width as usize;
    let mut prev_row = vec![(0i16, 0i16, 0i16); w];
    let mut current_row = vec![(0i16, 0i16, 0i16); w];

    for y in 0..(height as usize) {
        let mut prev_pixel = (0i16, 0i16, 0i16);
        for x in 0..w {
            let idx = (y * w + x) * 3;
            let (cy, co, cg) = rgb_to_ycocg_r(rgb_data[idx], rgb_data[idx + 1], rgb_data[idx + 2]);
            current_row[x] = (cy, co, cg);
            let top = prev_row[x];
            let top_left = if x > 0 { prev_row[x - 1] } else { (0, 0, 0) };
            residuals.extend_from_slice(&(cy - paeth_predict(prev_pixel.0, top.0, top_left.0)).to_le_bytes());
            residuals.extend_from_slice(&(co - paeth_predict(prev_pixel.1, top.1, top_left.1)).to_le_bytes());
            residuals.extend_from_slice(&(cg - paeth_predict(prev_pixel.2, top.2, top_left.2)).to_le_bytes());
            prev_pixel = (cy, co, cg);
        }
        prev_row.copy_from_slice(&current_row);
    }

    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::best());
    encoder.write_all(&residuals)?;
    let compressed_data = encoder.finish()?;

    let mut final_file = Vec::new();
    final_file.write_all(bytemuck::bytes_of(&ResHeader::new(width, height)))?;
    final_file.write_all(&compressed_data)?;
    Ok(final_file)
}
