use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub dictionaries: [u32; 3],
    pub program_directory: Vec<ProgramDirectory>,
    pub size_high: u32,
    pub size_low: u32,
}

#[derive(Debug, Deserialize)]
pub struct ProgramDirectory {
    pub comp: u32,
    pub exc_hi: u32,
    pub exc_lo_small: u32,
    pub exc_lo: u32,
}
