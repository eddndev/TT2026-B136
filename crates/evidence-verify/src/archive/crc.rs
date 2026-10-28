//! CRC-32 as ZIP requires it: IEEE 802.3, reflected polynomial 0xEDB88320,
//! initial value and final xor all ones.

const POLYNOMIAL: u32 = 0xedb8_8320;

/// One remainder per byte value, computed at compile time.
const TABLE: [u32; 256] = build_table();

const fn build_table() -> [u32; 256] {
    let mut table = [0u32; 256];
    let mut index = 0;
    while index < 256 {
        let mut crc = index as u32;
        let mut bit = 0;
        while bit < 8 {
            crc = if crc & 1 == 1 {
                (crc >> 1) ^ POLYNOMIAL
            } else {
                crc >> 1
            };
            bit += 1;
        }
        table[index] = crc;
        index += 1;
    }
    table
}

/// The CRC-32 of `data`.
pub(crate) fn crc32(data: &[u8]) -> u32 {
    let mut crc = u32::MAX;
    for byte in data {
        let slot = (crc ^ u32::from(*byte)) & 0xff;
        crc = (crc >> 8) ^ TABLE[slot as usize];
    }
    !crc
}
