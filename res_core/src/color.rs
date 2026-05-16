pub fn rgb_to_ycocg_r(r: u8, g: u8, b: u8) -> (i16, i16, i16) {
    let r = r as i16; let g = g as i16; let b = b as i16;
    let co = r - b; let t = b + (co >> 1); let cg = g - t; let y = t + (cg >> 1);
    (y, co, cg)
}

pub fn ycocg_r_to_rgb(y: i16, co: i16, cg: i16) -> (u8, u8, u8) {
    let t = y - (cg >> 1); let g = cg + t; let b = t - (co >> 1); let r = b + co;
    (r as u8, g as u8, b as u8)
}
