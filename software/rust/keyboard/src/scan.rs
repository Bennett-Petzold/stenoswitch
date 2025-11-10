use std::{
    io,
    ops::{Deref, DerefMut},
    simd::{Simd, num::SimdUint},
    sync::{Condvar, Mutex},
    thread,
    time::{Duration, Instant},
};

use gpio_cdev::{Chip, EventRequestFlags, LineEventHandle, LineHandle, LineRequestFlags};
use nix::{
    sys::time::TimeValLike,
    time::{ClockId, clock_gettime},
};

const GEMINI_PR_BYTES: usize = 6;

/// GeminiPR Steno packet.
///
/// Reference is <https://docs.qmk.fm/features/stenography#geminipr>.
///
/// ```ignore
/// 1 Fn  #1  #2 #3 #4 #5   #6
/// 0 S1- S2- T- K- P- W-   H-
/// 0 R-  A-  O- *1 *2 res1 res2
/// 0 pwr *3  *4 -E -U -F   -R
/// 0 -P  -B  -L -G -T -S   -D
/// 0 #7  #8  #9 #A #B #C   -Z
/// ```
#[derive(Debug)]
pub struct GeminiPr([u8; GEMINI_PR_BYTES]);

impl GeminiPr {
    /// Creates a new empty packet.
    pub const fn new() -> Self {
        let mut inner = [0; GEMINI_PR_BYTES];
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

/// Size of a full keyboard scan.
///
/// 3 rows
/// (5 * 2) columns
const RAW_SCAN_LEN: usize = 30;

/// [`RAW_SCAN_LEN`] with SIMD pad, two blank.
const RAW_SCAN_LEN_SIMD: usize = 32;

/// First 30 elements are scan results.
///
/// Last elements are guaranteed to be 0 (false).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct RawScan(Simd<u8, RAW_SCAN_LEN_SIMD>);

impl RawScan {
    #[inline]
    pub const fn new() -> Self {
        let scan_values = Simd::splat(0);
        Self(scan_values)
    }

    #[inline]
    /// Fills from array with the expected scan order.
    ///
    /// Rows 0-2, columns right 0-4 then left 0-4 for each row.
    pub fn from_scan(scan: [u8; RAW_SCAN_LEN]) -> Self {
        let mut this = Self::new();
        this.0[..RAW_SCAN_LEN].copy_from_slice(&scan);
        this
    }

    #[inline]
    /// Add new values to the total scan.
    pub fn extend(&mut self, new: Self) {
        self.0 |= new.0;
    }

    #[inline]
    /// The number of scanned keys.
    pub fn len(&self) -> u8 {
        // All keys are 0 (not scanned) or 1 (scanned).
        self.0.reduce_sum()
    }

    #[inline]
    /// Returns true when no keys were scanned.
    pub fn is_empty(&self) -> bool {
        // All keys are 0 (not scanned) or 1 (scanned).
        self.0.reduce_or() == 0
    }

    #[inline]
    pub fn update_gemini(mut self, packet: &mut GeminiPr) {
        // Rows go 0 to 2 in order.
        // Columns are right 0 to 4, then left 0 to 4.
        // Produces this, concated in order:
        //
        // Row 0
        // [-T, -L, -P, -F, *3]
        // [S1-, T-, P-, H-, *1]
        //
        // Row 1
        // [-S, -G, -B, -R, *4]
        // [S2-, K-, W-, R-, *2]
        //
        // Row 2
        // [-Z, -D, #C, -U, -E]
        // [res2, res1, #B, A-, O-]
        //
        // Trailing:
        // [always 0, always 0]
        //
        // Output after reloc is
        // [S1-, S2-, T-, K-, P-, W-, H-]
        // [R-, A-, O-, *1, *2, res1, res2]
        // [_(always 0), *3, *4, -E, -U, -F, -R]
        // [-P, -B, -L, -G, -T, -S, -D]
        // [#B, #C, -Z]
        const RELOCS: Simd<u8, RAW_SCAN_LEN_SIMD> = Simd::from_array([
            3, 23, 21, 19, 15, // [-T, -L, -P, -F, *3]
            0, 2, 4, 6, 10, // [S1-, T-, P-, H-, *1]
            26, 24, 22, 20, 16, // [-S, -G, -B, -R, *4]
            1, 3, 5, 7, 11, // [S2-, K-, W-, R-, *2]
            30, 27, 29, 18, 17, // [-Z, -D, #C, -U, -E]
            13, 12, 29, 8, 9, // [res2, res1, #B, A-, O-]
            14, 31, // _(always 0), END
        ]);

        // MSB order with 4 chunks of 7, one chunk of 3, one trailing
        const SHIFTS: Simd<u8, RAW_SCAN_LEN_SIMD> = Simd::from_array([
            6, 5, 4, 3, 2, 1, 0, 6, 5, 4, 3, 2, 1, 0, 6, 5, 4, 3, 2, 1, 0, 6, 5, 4, 3, 2, 1, 0, 2,
            1, 0, 0,
        ]);

        // Change positions for GeminiPR splits
        // Organized in lines of 7 bits
        self.0 = self.0.swizzle_dyn(RELOCS);
        self.0 <<= SHIFTS;

        // Shrink into individual chunks
        let row1 = self.0.extract::<{ 7 * 0 }, 7>();
        let row2 = self.0.extract::<{ 7 * 1 }, 7>();
        let row3 = self.0.extract::<{ 7 * 2 }, 7>();
        let row4 = self.0.extract::<{ 7 * 3 }, 7>();
        let row5 = self.0.extract::<{ 7 * 4 }, 4>();
        let simd_split = [row1, row2, row3, row4];

        // Convert to single bytes and write in.
        // First row is left as default, the keyboard never changes those
        // values.
        for (packet, new) in packet.iter_mut().skip(1).zip(simd_split) {
            *packet = new.reduce_or();
        }

        // Final row is shorter than the rest.
        packet[5] = row5.reduce_or();
    }

    #[inline]
    pub fn create_gemini(self) -> GeminiPr {
        let mut packet = GeminiPr::new();
        self.update_gemini(&mut packet);
        packet
    }
}

impl Default for RawScan {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
pub struct KeyScanner {
    rows: [LineHandle; 3],
    // Right hand columns first
    columns: [[LineEventHandle; 5]; 2],
}

impl KeyScanner {
    pub fn new() -> Result<Self, gpio_cdev::Error> {
        const ROW_ENV: [&str; 3] = ["ROW0", "ROW1", "ROW2"];
        const RHS_COLUMNS: [&str; 5] = ["RCOL0", "RCOL1", "RCOL2", "RCOL3", "RCOL4"];
        const LHS_COLUMNS: [&str; 5] = ["LCOL0", "LCOL1", "LCOL2", "LCOL3", "LCOL4"];

        let mut chip = Chip::new(option_env!("GPIO_CHIP").unwrap_or("/dev/null"))?;

        let rows = [
            chip.get_line(str::parse(option_env!("ROW0").unwrap_or("/dev/null")).unwrap())?
                .request(LineRequestFlags::OUTPUT, 0, "keyboard_scan")?,
            chip.get_line(str::parse(option_env!("ROW1").unwrap_or("/dev/null")).unwrap())?
                .request(LineRequestFlags::OUTPUT, 0, "keyboard_scan")?,
            chip.get_line(str::parse(option_env!("ROW2").unwrap_or("/dev/null")).unwrap())?
                .request(LineRequestFlags::OUTPUT, 0, "keyboard_scan")?,
        ];

        let columns = [
            [
                chip.get_line(str::parse(option_env!("RCOL0").unwrap_or("/dev/null")).unwrap())?
                    .events(
                        LineRequestFlags::INPUT,
                        EventRequestFlags::RISING_EDGE,
                        "keyboard_scan",
                    )?,
                chip.get_line(str::parse(option_env!("RCOL1").unwrap_or("/dev/null")).unwrap())?
                    .events(
                        LineRequestFlags::INPUT,
                        EventRequestFlags::RISING_EDGE,
                        "keyboard_scan",
                    )?,
                chip.get_line(str::parse(option_env!("RCOL2").unwrap_or("/dev/null")).unwrap())?
                    .events(
                        LineRequestFlags::INPUT,
                        EventRequestFlags::RISING_EDGE,
                        "keyboard_scan",
                    )?,
                chip.get_line(str::parse(option_env!("RCOL3").unwrap_or("/dev/null")).unwrap())?
                    .events(
                        LineRequestFlags::INPUT,
                        EventRequestFlags::RISING_EDGE,
                        "keyboard_scan",
                    )?,
                chip.get_line(str::parse(option_env!("RCOL4").unwrap_or("/dev/null")).unwrap())?
                    .events(
                        LineRequestFlags::INPUT,
                        EventRequestFlags::RISING_EDGE,
                        "keyboard_scan",
                    )?,
            ],
            [
                chip.get_line(str::parse(option_env!("LCOL0").unwrap_or("/dev/null")).unwrap())?
                    .events(
                        LineRequestFlags::INPUT,
                        EventRequestFlags::RISING_EDGE,
                        "keyboard_scan",
                    )?,
                chip.get_line(str::parse(option_env!("LCOL1").unwrap_or("/dev/null")).unwrap())?
                    .events(
                        LineRequestFlags::INPUT,
                        EventRequestFlags::RISING_EDGE,
                        "keyboard_scan",
                    )?,
                chip.get_line(str::parse(option_env!("LCOL2").unwrap_or("/dev/null")).unwrap())?
                    .events(
                        LineRequestFlags::INPUT,
                        EventRequestFlags::RISING_EDGE,
                        "keyboard_scan",
                    )?,
                chip.get_line(str::parse(option_env!("LCOL3").unwrap_or("/dev/null")).unwrap())?
                    .events(
                        LineRequestFlags::INPUT,
                        EventRequestFlags::RISING_EDGE,
                        "keyboard_scan",
                    )?,
                chip.get_line(str::parse(option_env!("LCOL4").unwrap_or("/dev/null")).unwrap())?
                    .events(
                        LineRequestFlags::INPUT,
                        EventRequestFlags::RISING_EDGE,
                        "keyboard_scan",
                    )?,
            ],
        ];

        Ok(Self { rows, columns })
    }

    /// Busy waits until a key is pressed.
    pub fn wait_for_input(&mut self) -> Result<(), gpio_cdev::Error> {
        let notify = Condvar::new();
        let status_mut = Mutex::new(false);

        let start_time = clock_gettime(ClockId::CLOCK_MONOTONIC)
            .expect("Well defined Linux call")
            .num_milliseconds() as u64;

        // Turning on all rows means any key will trigger a press
        for row in &self.rows {
            row.set_value(1)?;
        }

        // Wait for any key to be pressed.
        // Since the events are blocking, each column needs its own thread.
        thread::scope(|s| {
            for column in self.columns.as_flattened_mut() {
                s.spawn(|| {
                    loop {
                        if let Ok(event) = column.get_event() {
                            if event.timestamp() > start_time {
                                *status_mut.lock().unwrap() = true;
                                notify.notify_all();
                            }
                        }
                    }
                });
            }

            while *notify.wait(status_mut.lock().unwrap()).unwrap() != true {}
        });

        Ok(())
    }

    /// Scans at an effective 133 KHz.
    pub fn scan(&self) -> Result<GeminiPr, gpio_cdev::Error> {
        // Assuming the lines can handle 400 KHz.
        const LINE_DELAY: Duration = Duration::from_nanos(2500);
        // Ten rounds of zero scans
        const EMPTY_TO_END: u8 = 10;

        let mut empty_count = 0;
        let mut data = RawScan::new();

        // Skip turning the first row off, that's redundant.
        for row in &self.rows[1..] {
            row.set_value(0)?;
        }

        while empty_count < EMPTY_TO_END {
            let mut new_data = [0; 30];
            for (row_num, row) in self.rows.iter().enumerate() {
                row.set_value(1)?;
                spin_sleep::sleep(LINE_DELAY);

                let base_idx = row_num * 10;
                for (dest, column) in new_data[base_idx..10]
                    .iter_mut()
                    .zip(self.columns.as_flattened())
                {
                    *dest = column.get_value()?;
                }
                row.set_value(0)?;
            }

            let new_data = RawScan::from_scan(new_data);

            // Branchless tomfoolery to update the empty count
            {
                // Bool definition gives 0 for false, 1 for true.
                let empty = new_data.is_empty() as u8;
                // Increases the count when empty
                empty_count += empty;
                // Resets to zero when nonempty
                empty_count *= empty;
            }

            data.extend(new_data);
        }

        Ok(data.create_gemini())
    }
}
