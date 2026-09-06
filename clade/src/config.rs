use crate::{DICT_SIZE_WORDS, types::PdParams};

#[derive(Debug, Clone)]
pub struct CladeConfig {
    pub region: u32,
    pub pd_params: Vec<PdParams>,
    pub dicts: [[u32; DICT_SIZE_WORDS]; 3],
}

impl CladeConfig {
    pub fn init(&mut self) {
        for (i, p) in self.pd_params.iter_mut().enumerate() {
            p.region_hi_len = 0x4000000;
            p.region_lo_len = 0x4000000;
            let hi = self.region + (((i as u32) << 0x15) & 0x3fe00000);
            p.region_hi = hi;
            p.region_lo = hi + 0x4000000;
        }
    }
}
