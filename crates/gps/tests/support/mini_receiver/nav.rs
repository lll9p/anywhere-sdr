use gps::{Error, GpsTime};

use super::{PromptEpoch, WordDiagnostics};

#[derive(Clone, Debug)]
pub struct RecoveredSubframe {
    pub subframe_id: u8,
    pub tow_count: u32,
    pub data_words: [u32; 10],
    pub start_time: GpsTime,
}

#[derive(Clone, Debug)]
pub struct RecoveredNavigation {
    pub prn: usize,
    pub diagnostics: WordDiagnostics,
    pub subframes: Vec<RecoveredSubframe>,
}

struct Candidate {
    diagnostics: WordDiagnostics,
    subframes: Vec<RecoveredSubframe>,
    bit_offset_score: f64,
}

pub fn decode_navigation(
    prn: usize, prompt_epochs: &[PromptEpoch],
) -> Result<RecoveredNavigation, Error> {
    if prompt_epochs.len() < 620 {
        return Err(Error::msg(format!(
            "PRN {prn} has insufficient prompt epochs: {}",
            prompt_epochs.len()
        )));
    }

    let mut best_candidate: Option<Candidate> = None;
    for bit_offset in 0..20 {
        let bit_chunks: Vec<(f64, f64)> = prompt_epochs[bit_offset..]
            .chunks_exact(20)
            .map(|chunk| {
                chunk.iter().fold((0.0, 0.0), |(acc_re, acc_im), epoch| {
                    (acc_re + epoch.value_re, acc_im + epoch.value_im)
                })
            })
            .collect();
        if bit_chunks.len() < 330 {
            continue;
        }

        // Prefer bit boundaries that maximize coherent 20ms accumulation.
        // Parity/preamble checks can still succeed with slightly shifted
        // windows in high SNR, but timing-sensitive steps (pseudorange/PVT)
        // need a stable bit edge.
        let bit_offset_score = bit_chunks
            .iter()
            .map(|(sum_re, sum_im)| (sum_re * sum_re + sum_im * sum_im).sqrt())
            .sum::<f64>()
            / bit_chunks.len() as f64;

        for inverted_polarity in [false, true] {
            let bit_values: Vec<u8> = bit_chunks
                .iter()
                .map(|(sum_re, sum_im)| {
                    let phase = (2.0 * sum_re * sum_im)
                        .atan2(sum_re * sum_re - sum_im * sum_im)
                        / 2.0;
                    let aligned_re =
                        sum_re * phase.cos() + sum_im * phase.sin();
                    let aligned_re = if inverted_polarity {
                        -aligned_re
                    } else {
                        aligned_re
                    };
                    u8::from(aligned_re >= 0.0)
                })
                .collect();
            let Some(candidate) = find_candidate(
                prn,
                bit_offset,
                inverted_polarity,
                &bit_values,
                prompt_epochs,
            ) else {
                continue;
            };

            let mut candidate = candidate;
            candidate.bit_offset_score = bit_offset_score;

            let should_replace = best_candidate.as_ref().is_none_or(|best| {
                candidate.diagnostics.valid_word_count
                    > best.diagnostics.valid_word_count
                    || (candidate.diagnostics.valid_word_count
                        == best.diagnostics.valid_word_count
                        && candidate.subframes.len() > best.subframes.len())
                    || (candidate.diagnostics.valid_word_count
                        == best.diagnostics.valid_word_count
                        && candidate.subframes.len() == best.subframes.len()
                        && candidate.bit_offset_score > best.bit_offset_score)
            });
            if should_replace {
                best_candidate = Some(candidate);
            }
        }
    }

    let candidate = best_candidate.ok_or_else(|| {
        Error::msg(format!("failed to recover navigation words for PRN {prn}"))
    })?;

    Ok(RecoveredNavigation {
        prn,
        diagnostics: candidate.diagnostics,
        subframes: candidate.subframes,
    })
}

fn find_candidate(
    prn: usize, bit_offset: usize, inverted_polarity: bool, bit_values: &[u8],
    prompt_epochs: &[PromptEpoch],
) -> Option<Candidate> {
    let mut best_candidate: Option<Candidate> = None;

    for preamble_bit_index in 30..bit_values.len().saturating_sub(300) {
        let Some(candidate) = recover_subframes(
            prn,
            bit_offset,
            inverted_polarity,
            preamble_bit_index,
            bit_values,
            prompt_epochs,
        ) else {
            continue;
        };

        let should_replace = best_candidate.as_ref().is_none_or(|best| {
            candidate.diagnostics.valid_word_count
                > best.diagnostics.valid_word_count
                || (candidate.diagnostics.valid_word_count
                    == best.diagnostics.valid_word_count
                    && candidate.subframes.len() > best.subframes.len())
                || (candidate.diagnostics.valid_word_count
                    == best.diagnostics.valid_word_count
                    && candidate.subframes.len() == best.subframes.len()
                    && candidate.bit_offset_score > best.bit_offset_score)
        });
        if should_replace {
            best_candidate = Some(candidate);
        }
    }

    best_candidate
}

fn recover_subframes(
    _prn: usize, bit_offset: usize, _inverted_polarity: bool,
    preamble_bit_index: usize, bit_values: &[u8],
    prompt_epochs: &[PromptEpoch],
) -> Option<Candidate> {
    let mut position = preamble_bit_index;
    let mut previous_word = pack_word(&bit_values[position - 30..position]);
    let mut valid_word_count = 0usize;
    let mut previous_tow_count: Option<u32> = None;
    let mut previous_subframe_id: Option<u8> = None;
    let mut subframes = Vec::new();

    while position + 300 <= bit_values.len() {
        let mut data_words = [0u32; 10];
        let mut rolling_previous_word = previous_word;
        let mut word_parity_ok = [false; 10];

        for word_index in 0..10 {
            let word = pack_word(
                &bit_values[position + word_index * 30
                    ..position + (word_index + 1) * 30],
            );
            let parity_ok =
                validate_word(word, rolling_previous_word, word_index);
            word_parity_ok[word_index] = parity_ok;
            if parity_ok {
                valid_word_count += 1;
            }

            data_words[word_index] =
                recover_data_bits(word, rolling_previous_word);
            rolling_previous_word = word;
        }

        // Word 1 (TLM) and word 2 (HOW) parity must be valid; otherwise this
        // alignment is too ambiguous for timing-sensitive steps like PVT.
        if !word_parity_ok[0] || !word_parity_ok[1] {
            break;
        }

        if data_words[0] >> 16 != 0x8b {
            break;
        }

        let tow_count = (data_words[1] >> 7) & 0x1ffff;
        let subframe_id = ((data_words[1] >> 2) & 0x7) as u8;
        if !(1..=5).contains(&subframe_id) {
            break;
        }
        if let Some(previous_tow_count) = previous_tow_count {
            if tow_count != previous_tow_count + 1 {
                break;
            }
        }
        if let Some(previous_subframe_id) = previous_subframe_id {
            let expected_subframe_id = (previous_subframe_id % 5) + 1;
            if subframe_id != expected_subframe_id {
                break;
            }
        }

        let start_epoch_index = bit_offset + position * 20;
        if start_epoch_index >= prompt_epochs.len() {
            break;
        }

        previous_tow_count = Some(tow_count);
        previous_subframe_id = Some(subframe_id);
        subframes.push(RecoveredSubframe {
            subframe_id,
            tow_count,
            data_words,
            start_time: prompt_epochs[start_epoch_index].start_time.clone(),
        });

        previous_word = rolling_previous_word;
        position += 300;
    }

    if subframes.is_empty() {
        return None;
    }

    Some(Candidate {
        diagnostics: WordDiagnostics { valid_word_count },
        subframes,
        bit_offset_score: 0.0,
    })
}

fn pack_word(bits: &[u8]) -> u32 {
    bits.iter()
        .fold(0u32, |word, bit| (word << 1) | u32::from(*bit))
}

fn recover_data_bits(word: u32, previous_word: u32) -> u32 {
    let mut data = (word >> 6) & 0x00ff_ffff;
    if (previous_word & 0x1) != 0 {
        data ^= 0x00ff_ffff;
    }
    data
}

fn validate_word(word: u32, previous_word: u32, word_index: usize) -> bool {
    let source = ((previous_word & 0x3) << 30) | (word & 0x3fff_ffc0);
    let nib = i32::from(word_index == 1 || word_index == 9);
    compute_checksum(source, nib) == word
}

fn compute_checksum(source: u32, nib: i32) -> u32 {
    let parity_masks = [
        0x3b1f_3480,
        0x1d8f_9a40,
        0x2ec7_cd00,
        0x1763_e680,
        0x2bb1_f340,
        0x0b7a_89c0,
    ];
    let mut data = source & 0x3fff_ffc0;
    let d29 = (source >> 31) & 0x1;
    let d30 = (source >> 30) & 0x1;
    if nib != 0 {
        if (d30 + (parity_masks[4] & data).count_ones()) % 2 != 0 {
            data ^= 0x1 << 6;
        }
        if (d29 + (parity_masks[5] & data).count_ones()) % 2 != 0 {
            data ^= 0x1 << 7;
        }
    }

    let mut transmitted = data;
    if d30 != 0 {
        transmitted ^= 0x3fff_ffc0;
    }
    transmitted |= ((d29 + (parity_masks[0] & data).count_ones()) % 2) << 5;
    transmitted |= ((d30 + (parity_masks[1] & data).count_ones()) % 2) << 4;
    transmitted |= ((d29 + (parity_masks[2] & data).count_ones()) % 2) << 3;
    transmitted |= ((d30 + (parity_masks[3] & data).count_ones()) % 2) << 2;
    transmitted |= ((d30 + (parity_masks[4] & data).count_ones()) % 2) << 1;
    transmitted |= (d29 + (parity_masks[5] & data).count_ones()) % 2;
    transmitted & 0x3fff_ffff
}
