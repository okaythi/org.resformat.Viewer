use crate::{ResHeader, color::ycocg_r_to_rgb, predict::paeth_predict};
use std::io::Read;
use flate2::read::ZlibDecoder;

pub fn decode_res_to_rgb(res_data: &[u8]) -> std::io::Result<(u32, u32, Vec<u8>)> {
    let header_size = std::mem::size_of::<ResHeader>();
    if res_data.len() < header_size { return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "File too small")); }
    let header: &ResHeader = bytemuck::from_bytes(&res_data[..header_size]);
    
    let mut decompressed = Vec::new();
    ZlibDecoder::new(&res_data[header_size..]).read_to_end(&mut decompressed)?;

    let w = header.width as usize; let h = header.height as usize;
    let mut rgb_out = Vec::with_capacity(w * h * 3);
    let mut prev_row = vec![(0i16, 0i16, 0i16); w];
    let mut current_row = vec![(0i16, 0i16, 0i16); w];
    let mut res_idx = 0;

    for _ in 0..h {
        let mut prev_pixel = (0i16, 0i16, 0i16);
        for x in 0..w {
            let mut read_i16 = || -> i16 { let b = [decompressed[res_idx], decompressed[res_idx+1]]; res_idx += 2; i16::from_le_bytes(b) };
            let cy = read_i16() + paeth_predict(prev_pixel.0, prev_row[x].0, if x > 0 { prev_row[x-1].0 } else { 0 });
            let co = read_i16() + paeth_predict(prev_pixel.1, prev_row[x].1, if x > 0 { prev_row[x-1].1 } else { 0 });
            let cg = read_i16() + paeth_predict(prev_pixel.2, prev_row[x].2, if x > 0 { prev_row[x-1].2 } else { 0 });
            current_row[x] = (cy, co, cg); prev_pixel = (cy, co, cg);
            let (r, g, b) = ycocg_r_to_rgb(cy, co, cg);
            rgb_out.extend_from_slice(&[r, g, b]);
        }
        prev_row.copy_from_slice(&current_row);
    }
    Ok((header.width, header.height, rgb_out))  
}
