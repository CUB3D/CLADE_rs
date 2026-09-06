#[derive(Default)]
pub struct BitWriter {
    pub words: [u32; 32],
    pub pos: i32,
}

impl BitWriter {
    pub fn add(&mut self, val: u32, n: i32) {
        for i in 0..n {
            if val & (1u32 << (i & 0x1f)) != 0 {
                self.words[(self.pos >> 5) as usize] |= 1u32 << (self.pos & 0x1f);
            }
            self.pos += 1;
        }
    }
}
