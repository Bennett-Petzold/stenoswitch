use std::{
    ops::{Deref, DerefMut},
    simd::Simd,
};

#[derive(Debug)]
struct GeminiPr([u8; 6]);

impl GeminiPr {
    /// Creates a new empty packet.
    pub const fn new() -> Self {
        let mut inner = [0; 6];
        inner[0] = 0b1;
        Self(inner)
    }
}

impl Default for GeminiPr {
    fn default() -> Self {
        Self::new()
    }
}

impl Deref for GeminiPr {
    type Target = [u8; 6];
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for GeminiPr {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// 3 rows
/// (5 * 2) columns
/// 30 SIMD elements, two blank
const RAW_SCAN_LEN: usize = 32;

/// First 30 elements are scan results.
///
/// Last elements are guaranteed to be 0 (false).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct RawScan(Simd<u8, RAW_SCAN_LEN>);

impl RawScan {
    #[inline]
    pub fn new() -> Self {
        let scan_values = Simd::splat(0);
        Self(scan_values)
    }

    #[inline]
    pub fn update_gemini(self, packet: &mut GeminiPr) {
        const RELOCS: Simd<u8, RAW_SCAN_LEN> = Simd::from_array([
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0,
        ]);

        // Change positions for GeminiPR splits
        // Each row is put in reverse order of final GeminiPR.
        let bits = self.0.swizzle_dyn(RELOCS);
        let bits = bits.as_array();

        // Split into GeminiPR rows.
        let (row1, rem) = bits.split_at(7);
        let (row2, rem) = rem.split_at(7);
        let (row3, rem) = rem.split_at(4);
        let (row4, z) = rem.split_at(7);

        let byte_conv = |row: &[u8]| {
            let mut byte = 0;
            for (idx, bit) in row.iter().enumerate() {
                byte |= bit << idx;
            }
            byte
        };

        // Convert to single bytes and write in.
        // First row is left as default, the keyboard never changes those
        // values.
        packet[1] = byte_conv(row1);
        packet[2] = byte_conv(row2);
        packet[3] = byte_conv(row3);
        packet[4] = byte_conv(row4);
        packet[5] = z[0];
    }
}
