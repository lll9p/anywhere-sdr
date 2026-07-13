use constants::{N_DWRD_SBF, N_SBF};

use super::Channel;
use crate::datetime::GpsTime;

impl Channel {
    /// Generates the full navigation message data words (DWRD) for a 30-second
    /// cycle.
    ///
    /// Populates the `self.dwrd` array with the 5 subframes (50 words total).
    /// It inserts the Time of Week (TOW) count and Week Number (WN) into the
    /// appropriate words and computes the parity checksum for each word.
    ///
    /// # Arguments
    /// * `time` - The current GPS time used to calculate TOW and WN.
    /// * `init` - Flag indicating if this is the initial generation (handles
    ///   subframe 5 differently).]
    pub fn generate_nav_msg(&mut self, time: &GpsTime, init: bool) {
        let time_init = time.floor_to_interval(30);
        let mut sbfwrd: u32;
        let mut prevwrd: u32 = 0;
        let mut nib: i32;

        let wn = (time_init.week % 1024) as u32;
        let mut tow = (time_init.sec as u32).wrapping_div(6);
        self.nav_message_start_time = time_init; // Data bit reference time

        if init {
            // Initialize subframe 5
            prevwrd = 0;
            for iwrd in 0..N_DWRD_SBF {
                sbfwrd = self.subframes[4][iwrd];
                // Add TOW-count message into HOW
                if iwrd == 1 {
                    sbfwrd |= (tow & 0x1ffff) << 13;
                }
                // Compute checksum
                sbfwrd |= prevwrd << 30 & 0xc000_0000; // 2 LSBs of the previous transmitted word
                nib = i32::from(iwrd == 1 || iwrd == 9); // Non-information bearing bits for word 2 and 10
                self.data_words[iwrd] = Self::compute_checksum(sbfwrd, nib);
                prevwrd = self.data_words[iwrd];
            }
        } else {
            // Save subframe 5
            for iwrd in 0..N_DWRD_SBF {
                self.data_words[iwrd] =
                    self.data_words[N_DWRD_SBF * N_SBF + iwrd];
                prevwrd = self.data_words[iwrd];
            }
            /*
            // Sanity check
            if (((chan->dwrd[1])&(0x1FFFFUL<<13)) != ((tow&0x1FFFFUL)<<13))
            {
                fprintf(stderr, "\nWARNING: Invalid TOW in subframe 5.\n");
                return(0);
            }
            */
        }
        for isbf in 0..N_SBF {
            tow = tow.wrapping_add(1);

            for iwrd in 0..N_DWRD_SBF {
                sbfwrd = self.subframes[isbf][iwrd];
                // Add transmission week number to Subframe 1
                if isbf == 0 && iwrd == 2 {
                    sbfwrd |= (wn & 0x3ff) << 20;
                }
                // Add TOW-count message into HOW
                if iwrd == 1 {
                    sbfwrd |= (tow & 0x1ffff) << 13;
                }
                // Compute checksum
                sbfwrd |= prevwrd << 30 & 0xc000_0000; // 2 LSBs of the previous transmitted word
                nib = i32::from(iwrd == 1 || iwrd == 9); // Non-information bearing bits for word 2 and 10
                self.data_words[(isbf + 1) * N_DWRD_SBF + iwrd] =
                    Self::compute_checksum(sbfwrd, nib);
                prevwrd = self.data_words[(isbf + 1) * N_DWRD_SBF + iwrd];
            }
        }
    }

    /// Computes the 6-bit parity checksum for a 30-bit navigation message word.
    ///
    /// Implements the parity algorithm defined in IS-GPS-200, using the
    /// previous word's last two bits (D29*, D30*) and the current word's 24
    /// data bits. Handles non-information bearing bits (NIB) adjustments
    /// for specific words.
    ///
    /// # Arguments
    /// * `source` - The 32-bit input word containing data bits (29-6) and
    ///   previous parity bits (31-30).
    /// * `nib` - Flag indicating if the word contains non-information-bearing
    ///   bits (usually words 2 and 10).
    ///
    /// # Returns
    /// The 32-bit word with the computed 6 parity bits (5-0) inserted.
    #[allow(non_snake_case)]
    // #[inline]
    pub fn compute_checksum(source: u32, nib: i32) -> u32 {
        /*
        Bits 31 to 30 = 2 LSBs of the previous transmitted word, D29* and D30*
        Bits 29 to  6 = Source data bits, d1, d2, ..., d24
        Bits  5 to  0 = Empty parity bits
        */

        /*
        Bits 31 to 30 = 2 LSBs of the previous transmitted word, D29* and D30*
        Bits 29 to  6 = Data bits transmitted by the SV, D1, D2, ..., D24
        Bits  5 to  0 = Computed parity bits, D25, D26, ..., D30
        */

        /*
                          1            2           3
        bit    12 3456 7890 1234 5678 9012 3456 7890
        ---    -------------------------------------
        D25    11 1011 0001 1111 0011 0100 1000 0000
        D26    01 1101 1000 1111 1001 1010 0100 0000
        D27    10 1110 1100 0111 1100 1101 0000 0000
        D28    01 0111 0110 0011 1110 0110 1000 0000
        D29    10 1011 1011 0001 1111 0011 0100 0000
        D30    00 1011 0111 1010 1000 1001 1100 0000
        */
        let bmask: [u32; 6] = [
            0x3b1f_3480,
            0x1d8f_9a40,
            0x2ec7_cd00,
            0x1763_e680,
            0x2bb1_f340,
            0x0b7a_89c0,
        ];
        let mut D: u32;
        let mut d: u32 = source & 0x3fff_ffc0;
        let D29: u32 = source >> 31 & 0x1;
        let D30: u32 = source >> 30 & 0x1;
        if nib != 0 {
            // Non-information bearing bits for word 2 and 10
            /*
            Solve bits 23 and 24 to preserve parity check
            with zeros in bits 29 and 30.
            */
            if D30
                .wrapping_add((bmask[4] & d).count_ones())
                .wrapping_rem(2)
                != 0
            {
                d ^= 0x1 << 6;
            }
            if D29
                .wrapping_add((bmask[5] & d).count_ones())
                .wrapping_rem(2)
                != 0
            {
                d ^= 0x1 << 7;
            }
        }
        D = d;
        if D30 != 0 {
            D ^= 0x3fff_ffc0;
        }
        D |= D29
            .wrapping_add((bmask[0] & d).count_ones())
            .wrapping_rem(2)
            << 5;
        D |= D30
            .wrapping_add((bmask[1] & d).count_ones())
            .wrapping_rem(2)
            << 4;
        D |= D29
            .wrapping_add((bmask[2] & d).count_ones())
            .wrapping_rem(2)
            << 3;
        D |= D30
            .wrapping_add((bmask[3] & d).count_ones())
            .wrapping_rem(2)
            << 2;
        D |= D30
            .wrapping_add((bmask[4] & d).count_ones())
            .wrapping_rem(2)
            << 1;
        D |= D29
            .wrapping_add((bmask[5] & d).count_ones())
            .wrapping_rem(2);
        D &= 0x3fff_ffff;

        //D |= (source & 0xC0000000UL); // Add D29* and D30* from source data
        // bits
        D
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn navigation_frame_start_uses_exact_floor() {
        let mut channel = Channel::default();
        channel.generate_nav_msg(
            &GpsTime {
                week: 2_000,
                sec: 29.999_999,
            },
            true,
        );
        assert_eq!(channel.nav_message_start_time, GpsTime {
            week: 2_000,
            sec: 0.0,
        });

        channel.generate_nav_msg(
            &GpsTime {
                week: 2_000,
                sec: 30.0,
            },
            true,
        );
        assert_eq!(channel.nav_message_start_time, GpsTime {
            week: 2_000,
            sec: 30.0,
        });
    }
}
