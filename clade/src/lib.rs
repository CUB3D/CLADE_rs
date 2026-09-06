use crate::bit_reader::BitReader;
use crate::bit_writer::BitWriter;
use crate::config::CladeConfig;
use crate::types::{MemRequest, PdParams};

pub mod bit_reader;
pub mod bit_writer;
pub mod config;
pub mod types;

pub const DICT_SIZE_BYTES: usize = 0x2000;
pub const DICT_SIZE_WORDS: usize = DICT_SIZE_BYTES >> 2;

const DICT_MISSING_BITS: [u32; 96] = [
    1, 15, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 1, 2, 4, 5, 6, 7, 8, 9, 16, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 1, 2, 4, 5, 7, 8, 9, 10, 15, 16, 17, 18, 19, 20, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0,
];

const MISSING_BITS_BY_DICT: [u32; 3] = [2, 9, 15];

// in lib this is var
const WORD_SIZE: u32 = 1;

fn extract_bits(v: u32, sz: u32, off: u32) -> u32 {
    1_u32.wrapping_shl(sz).wrapping_sub(1) & v.wrapping_shr(off)
}

fn reconstruct_word(mut base: u32, rem: u32, bits: &[u32], cnt: u32) -> u32 {
    for i in 0..cnt {
        let pos = bits[i as usize] & 0x1f;
        let mask = (1u32 << pos) - 1;

        let lo = base & mask;
        let hi = (base & !mask) << 1;
        let x = ((rem >> (i & 0x1f)) & 1) << pos;
        base = lo | hi | x;
    }
    base
}

fn extract_data_with_stream(r: &mut BitReader, nbits: i32, w: &mut BitWriter) {
    let cnt = (nbits + 0x1f) >> 5;
    let mut rem = nbits;
    for _ in 0..cnt {
        if rem < 0x20 {
            if rem > 0 {
                if r.bits < rem {
                    r.refill();
                }
                let v = (r.curr_word & ((1u32 << rem) - 1u32) as u64) as u32;
                w.add(v, rem);
                r.curr_word >>= rem;
                r.bits -= rem;
                if r.bits < 0 {
                    r.refill();
                }
            }
        } else {
            r.refill();
            w.add(r.curr_word as u32, 0x20);
            r.curr_word >>= 0x20;
            r.bits -= 0x20;
            if r.bits < 0 {
                r.refill();
            }
        }
        rem -= 0x20;
    }
}

fn clade_uncompress(
    out: &mut [u32],
    data: &[u32],
    dicts: [[u32; DICT_SIZE_WORDS]; 3],
) {
    let mut acc: u64 = data[0] as u64;
    let mut loc = 1usize;
    let mut tmp0: i32 = 0x20;
    let mut tmp1: u32 = 0;
    let mut out_amt: u32 = 0;
    let mut tmp2: i32 = 0;
    let num_out = out.len() * 4;
    let nout = num_out / 4;
    let mut o = 0usize;

    fn refill(acc: &mut u64, loc: &mut usize, d: &[u32], sh: i32) {
        *acc |= (d[*loc] as u64) << (sh & 0x3f);
        *loc += 1;
    }

    let mut dict_id = (acc & 0b11) as i32;
    while o < nout {
        if dict_id < 3 {
            let missing_bits_cnt = MISSING_BITS_BY_DICT[dict_id as usize];
            let dict = dicts[dict_id as usize];
            let idx0 = extract_bits(acc as u32, 0xb, 2);
            let rem = extract_bits(acc as u32, missing_bits_cnt, 0xd);
            let mut shift = -0xd - (missing_bits_cnt as i32) + tmp0;
            acc >>= (missing_bits_cnt + 0xd) & 0x3f;
            tmp0 = shift;
            if shift < 0x20 {
                tmp0 = shift + 0x20;
                refill(&mut acc, &mut loc, data, shift);
            }
            out_amt += 4;
            let bits = &DICT_MISSING_BITS[(0x20 * dict_id) as usize..][..32];
            out[o] = reconstruct_word(dict[idx0 as usize], rem, bits, missing_bits_cnt);
            o += 1;
            if out_amt >= num_out as u32 {
                break;
            }
            tmp2 += 1;
            tmp1 += 0xd + missing_bits_cnt;
            if tmp2 == 0x10 {
                tmp1 &= 7;
                if tmp1 != 0 {
                    shift = tmp0 - 8 + (tmp1 as i32);
                    acc >>= (8 - (tmp1 as i32)) & 0x3f;
                    tmp0 = shift;
                    if shift < 0x20 {
                        tmp1 = 0;
                        tmp0 = shift + 0x20;
                        tmp2 = 0;
                        refill(&mut acc, &mut loc, data, shift);
                    } else {
                        tmp1 = 0;
                        tmp2 = 0;
                    }
                } else {
                    tmp1 = 0;
                    tmp2 = 0;
                }
            }
        } else {
            let tmp3 = tmp0 - 2;
            let mut tmp4: u64 = acc >> 2;
            let mut tmp5 = tmp3;
            if tmp3 < 0x20 {
                tmp5 = tmp0 + 0x1e;
                tmp4 |= (data[loc] as u64) << (tmp3 & 0x3f);
                loc += 1;
            }
            tmp0 = tmp5 - 0x20;
            acc = tmp4 >> 0x20;
            if tmp0 < 0x20 {
                acc |= (data[loc] as u64) << (tmp0 & 0x3f);
                loc += 1;
                tmp0 = tmp5;
            }
            out_amt += 4;
            out[o] = tmp4 as u32;
            o += 1;
            if out_amt >= num_out as u32 {
                break;
            }
            tmp2 += 1;
            tmp1 += 0x22;
            if tmp2 == 0x10 {
                tmp1 &= 7;
                if tmp1 != 0 {
                    tmp5 = tmp0 + (tmp1 as i32 - 8);
                    acc >>= (8 - (tmp1 as i32)) & 0x3f;
                    tmp0 = tmp5;
                    if tmp5 < 0x20 {
                        tmp1 = 0;
                        tmp0 = tmp5 + 0x20;
                        tmp2 = 0;
                        refill(&mut acc, &mut loc, data, tmp5);
                    } else {
                        tmp1 = 0;
                        tmp2 = 0;
                    }
                } else {
                    tmp1 = 0;
                    tmp2 = 0;
                }
            }
        }
        dict_id = (acc & 3) as i32;
    }
}

fn decompress_high_line(pp: &PdParams, mem: &[u8], cfg: &CladeConfig, line: u32, out: &mut [u8]) {
    let mut slot = [0u32; 32];
    slot[..16].copy_from_slice(bytemuck::cast_slice::<u8, u32>(
        &mem[(pp.comp + line * 0x40) as usize..][..0x40],
    ));

    let mut r = BitReader::from_words(&slot);
    let bit = r.read(1);

    let mut w = BitWriter::default();

    if bit == 0 {
        let mut tmp = [0u32; 32];
        extract_data_with_stream(&mut r, 0x1ee, &mut w);
        tmp.copy_from_slice(&w.words);
        let mut dec = [0u32; 16];
        clade_uncompress(
            &mut dec,
            &tmp,
            cfg.dicts,
        );
        let dec = bytemuck::cast::<[u32; 0x10], [u8; 0x40]>(dec);
        out.copy_from_slice(&dec);
    } else {
        let x = r.read(12);
        extract_data_with_stream(&mut r, 0x1e0, &mut w);
        w.words[15] = u32::from_le_bytes(
            mem[(pp.exc_hi + (x) * 4) as usize..][..4]
                .try_into()
                .unwrap(),
        );
        let tmp = bytemuck::cast_slice::<u32, u8>(&w.words[..16]);
        out.copy_from_slice(tmp);
    }
}

fn flip_block_and_reverse_bits(b: &mut [u8; 0x20]) {
    b.reverse();
    for b in b.iter_mut() {
        *b = b.reverse_bits();
    }
}

fn decompress_low_line(pp: &PdParams, mem: &[u8], cfg: &CladeConfig, line: u32, out: &mut [u8]) {
    let mut buf = [0u32; 32];

    let mut a: [u8; 0x20] = mem[(pp.comp + line * 0x80 + 0x20) as usize..][..0x20]
        .try_into()
        .unwrap();
    flip_block_and_reverse_bits(&mut a);

    let a = bytemuck::cast::<[u8; 0x20], [u32; 0x8]>(a);
    buf[8..16].copy_from_slice(&a);

    let mut b: [u8; 0x20] = mem[(pp.comp + line * 0x80 + 0x60) as usize..][..0x20]
        .try_into()
        .unwrap();

    flip_block_and_reverse_bits(&mut b);
    let b = bytemuck::cast::<[u8; 0x20], [u32; 0x8]>(b);
    buf[24..32].copy_from_slice(&b);

    let mut r = BitReader::from_words(&buf[8..32]);
    let mut w = BitWriter::default();

    let l1 = r.read(5);
    let p1 = r.read(12);
    extract_data_with_stream(&mut r, (l1 * 8 - 9) as i32, &mut w);

    let mut r2 = BitReader::from_words(&buf[24..32]);
    let l2 = r2.read(5);
    let p2 = r2.read(12);
    extract_data_with_stream(&mut r2, (l2 * 8 - 9) as i32, &mut w);

    let ptr = (p2 << 12) | p1;
    if ptr != 0xffffff {
        let diff = ((pp.exc_lo_small as i64).wrapping_sub(pp.exc_lo as i64)) >> 2;
        let sd = (((diff & 0xffffffff) as i32).wrapping_add(0xf)) >> 4;
        let uvar7: u64 = if ((ptr >> 6) as i32) < sd {
            (ptr as u64 * 8 + 0x200) & 0xfffffe00
        } else {
            (ptr as u64 * 8 + 0x100) & 0xffffff00
        };
        let nbytes = (uvar7 - ptr as u64 * 8) as usize / 8;
        let exc_bytes = &mem[(pp.exc_lo + ptr) as usize..][..nbytes];
        for &byte in exc_bytes {
            w.add(byte as u32, 8);
        }
    }

    if l1 != 0 || l2 != 0 {
        let mut tmp = [0u32; 16];
        clade_uncompress(
            &mut tmp,
            &w.words,
            cfg.dicts
        );
        let dec = bytemuck::cast::<[u32; 0x10], [u8; 0x40]>(tmp);
        out.copy_from_slice(&dec);
    } else {
        let tmp = bytemuck::cast_slice::<u32, u8>(&w.words[..16]);
        out.copy_from_slice(tmp);
    }
}

pub fn clade_read(cfg: &CladeConfig, requests: &[MemRequest], mem: &[u8]) -> anyhow::Result<Vec<Vec<u8>>> {
    let mut out = Vec::new();

    for r in requests {
        let pd_idx = cfg.pd_params.iter().position(|p| {
            (p.region_hi..(p.region_hi + p.region_hi_len)).contains(&r.addr) ||
                (p.region_lo..(p.region_lo + p.region_lo_len)).contains(&r.addr)
        }).unwrap() as i64;

        if r.addr == 0 {
            out.push(Vec::new());
            continue;
        }

        let pp = cfg.pd_params[pd_idx as usize];
        let high =
            (r.addr < pp.region_lo) || (r.addr >= pp.region_lo + pp.region_lo_len);

        let mut rem = WORD_SIZE * r.len;
        let base: i64 = if high {
            pp.region_hi as i64
        } else {
            pp.region_lo as i64
        };
        let mut diff: u32 = (r.addr as i64 - base) as u32;
        let mut pd_out = vec![0u8; rem as usize];
        let mut outpos = 0usize;

        while rem > 0 {
            let line = diff >> 6;
            let off = diff & 0x3f;
            let mut n = rem.min(0x40);
            if off + n > 0x40 {
                n = 0x40 - off;
            }

            let mut tmp = [0u8; 0x40];
            if high {
                decompress_high_line(&pp, mem, cfg, line, &mut tmp);
            } else {
                decompress_low_line(&pp, mem, cfg, line, &mut tmp);
            }

            pd_out[outpos..outpos + n as usize]
                .copy_from_slice(&tmp[off as usize..(off + n) as usize]);
            rem -= n;
            diff += n;
            outpos += n as usize;
        }

        out.push(pd_out);
    }

    Ok(out)
}
