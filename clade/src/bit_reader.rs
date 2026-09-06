pub struct BitReader<'a> {
    pub words: &'a [u32],
    pub idx: usize,
    pub curr_word: u64,
    pub bits: i32,
}

impl<'a> BitReader<'a> {
    pub fn from_words(w: &'a [u32]) -> Self {
        BitReader {
            words: w,
            idx: 1,
            curr_word: w[0] as u64,
            bits: 0x20,
        }
    }

    pub fn refill(&mut self) {
        if self.bits < 0x20 {
            self.curr_word |= (self.words[self.idx] as u64) << (self.bits & 0x3f);
            self.bits += 0x20;
            self.idx += 1;
        }
    }
    pub fn read(&mut self, n: i32) -> u32 {
        if self.bits < n {
            self.refill();
        }
        let v = (self.curr_word & ((1u64 << n) - 1u64)) as u32;
        self.curr_word >>= n;
        self.bits -= n;
        if self.bits < 0 {
            self.refill();
        }
        v
    }
}
