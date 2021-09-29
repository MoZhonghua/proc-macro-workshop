// Crates that have the "proc-macro" crate type are only allowed to export
// procedural macros. So we cannot have one crate that defines procedural macros
// alongside other types of public APIs like traits and structs.
//
// For this project we are going to need a #[bitfield] macro but also a trait
// and some structs. We solve this by defining the trait and structs in this
// crate, defining the attribute macro in a separate bitfield-impl crate, and
// then re-exporting the macro from this crate so that users only have one crate
// that they need to import.
//
// From the perspective of a user of this crate, they get all the necessary APIs
// (macro, trait, struct) through the one bitfield crate.
pub use bitfield_impl::bitfield;
pub use bitfield_impl::BitfieldSpecifier;

// TODO other things

pub mod checks;
pub mod specifiers;

pub use specifiers::*;

#[doc(hidden)]
#[inline]
pub fn read_bits(data: &[u8], start: usize, bits: usize) -> u64 {
    let end = start + bits;
    assert!(start <= data.len() * 8);
    assert!(end <= data.len() * 8);

    let mut v = 0_u64;
    for pos in (start - start % 8..end).step_by(8) {
        let bit_start = if pos < start { start - pos } else { 0_usize };
        let bit_end = if end - pos >= 8 { 8 } else { end - pos };

        let byte = data[pos / 8];
        if bit_start == 0 && bit_end == 8 {
            v = (v << 8) | byte as u64;
        } else {
            let mask = u8::MAX ^ ((1 << (8 - bit_end)) - 1);
            let mask = mask & (u8::MAX >> bit_start);
            let bits = (byte & mask) >> (8 - bit_end);
            v = (v << (bit_end - bit_start)) | bits as u64;
        }
    }

    v
}

#[doc(hidden)]
#[inline]
pub fn write_bits(data: &mut [u8], start: usize, bits: usize, val: u64) {
    // println!("write_bits: start={}, bit={}; val={}", start, bits, val);
    let end = start + bits;
    assert!(start <= data.len() * 8);
    assert!(end <= data.len() * 8);

    let mut v = val;
    // in reverse order
    for pos in (start - start % 8..end).step_by(8).rev() {
        let bit_start = if pos < start { start - pos } else { 0_usize };

        let bit_end = if (end - pos) >= 8 { 8 } else { end - pos };

        /*
        println!(
            "    pos: {} bit_start: {}; bit_end: {}",
            pos, bit_start, bit_end
            );
        */

        let bit_count = bit_end - bit_start;
        let bits = (v & ((1 << bit_count) - 1)) as u8;
        v = v >> bit_count;

        if bit_start == 0 && bit_end == 8 {
            data[pos / 8] = bits as u8;
        } else {
            let old_mask = u8::MAX ^ ((1 << (8 - bit_end)) - 1);
            let old_mask = old_mask & (u8::MAX >> bit_start);
            let old_mask = !old_mask;
            let old = data[pos / 8] & old_mask;

            // println!("    old mask: {:08b}", old_mask);
            // println!("    old: {:08b}", old);
            let bits = bits << (8 - bit_end);
            data[pos / 8] = old | bits;
        }
    }

    // println!("    result: {:08b}", data[0]);
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn read_whole_one_byte() {
        let data = vec![11_u8, 22, 33, 44, 55];
        assert_eq!(read_bits(&data, 0, 8), 11);
        assert_eq!(read_bits(&data, 8, 8), 22);
        assert_eq!(read_bits(&data, 16, 8), 33);
        assert_eq!(read_bits(&data, 24, 8), 44);
    }

    #[test]
    fn write_whole_one_byte() {
        let mut data = vec![0u8; 8];
        write_bits(&mut data, 0, 8, 11);
        assert_eq!(read_bits(&data, 0, 8), 11);

        write_bits(&mut data, 8, 8, 22);
        assert_eq!(read_bits(&data, 8, 8), 22);

        write_bits(&mut data, 16, 8, 33);
        assert_eq!(read_bits(&data, 16, 8), 33);

        write_bits(&mut data, 24, 8, 44);
        assert_eq!(read_bits(&data, 24, 8), 44);
    }

    #[test]
    fn read_whole_mutiple_bytes() {
        let data = vec![0x11_u8, 0x22, 0x33, 0x44, 0x55];
        assert_eq!(read_bits(&data, 0, 16), 0x1122);
        assert_eq!(read_bits(&data, 8, 24), 0x223344);
    }

    #[test]
    fn write_whole_mutiple_bytes() {
        let mut data = vec![0u8; 4];
        write_bits(&mut data, 0, 16, 0x1122);
        // println!("{:?}", data);
        assert_eq!(read_bits(&data, 0, 16), 0x1122);
        write_bits(&mut data, 8, 24, 0x223344);
        assert_eq!(read_bits(&data, 8, 24), 0x223344);
    }

    #[test]
    fn read_cross_byte_boundary() {
        let data = vec![0x11_u8, 0x22, 0x33, 0x44, 0x55];
        assert_eq!(read_bits(&data, 0, 12), 0x112);
        assert_eq!(read_bits(&data, 12, 12), 0x233);
    }

    #[test]
    fn write_cross_byte_boundary() {
        let mut data = vec![0u8; 4];
        write_bits(&mut data, 0, 12, 0x112);
        assert_eq!(read_bits(&data, 0, 12), 0x112);

        write_bits(&mut data, 7, 16, 0x3344);
        assert_eq!(read_bits(&data, 7, 16), 0x3344);
    }
}
