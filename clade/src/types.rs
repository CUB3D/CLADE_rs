#[derive(Debug, Default, Copy, Clone)]
pub struct PdParams {
    pub comp: u32,
    pub exc_hi: u32,
    pub exc_lo_small: u32,
    pub exc_lo: u32,
    pub region_hi: u32,
    pub region_hi_len: u32,
    pub region_lo: u32,
    pub region_lo_len: u32,
}

#[derive(Debug, Clone, Copy)]
pub struct MemRequest {
    pub addr: u32,
    pub len: u32,
}
