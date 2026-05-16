pub mod color;
pub mod predict;
pub mod encode;
pub mod decode;

use bytemuck::{Pod, Zeroable};

pub const RES_MAGIC: [u8; 4] = [b'R', b'E', b'S', b'\0'];

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct ResHeader {
    pub magic: [u8; 4],
    pub version: u8,
    pub color_space: u8,
    pub bit_depth: u8,
    pub _padding: u8,
    pub width: u32,
    pub height: u32,
}

impl ResHeader {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            magic: RES_MAGIC,
            version: 1,
            color_space: 1,
            bit_depth: 8,
            _padding: 0,
            width,
            height,
        }
    }
}
