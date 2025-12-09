use alloc::vec::Vec;

use super::{chunk::SeaChunk, dqt::SeaDequantTab};

pub struct Decoder {
    channels: usize,
    scale_factor_bits: usize,

    dequant_tab: SeaDequantTab,
}

impl Decoder {
    pub fn init(channels: usize, scale_factor_bits: usize) -> Self {
        Self {
            channels,
            scale_factor_bits,

            dequant_tab: SeaDequantTab::init(scale_factor_bits),
        }
    }

    pub fn decode_cbr(&self, chunk: &SeaChunk, output: &mut Vec<i16>) {
        assert_eq!(chunk.scale_factor_bits as usize, self.scale_factor_bits);

        output.reserve(chunk.frames_per_chunk * self.channels);

        let mut lms = chunk.lms.clone();

        let dqts: &Vec<Vec<i32>> = self.dequant_tab.get_dqt(chunk.residual_size as usize);

        let mut base_scale_factors = chunk.scale_factors.iter();

        let mut scale_factors = Vec::with_capacity(self.channels);

        for (i, residual) in chunk.residuals.iter().enumerate() {
            let scale_factor_index = i % (self.channels * chunk.scale_factor_frames as usize);
            // get next subchunk data
            if scale_factor_index == 0 {
                scale_factors.clear();
                scale_factors.extend(base_scale_factors.by_ref().take(self.channels))
            }
            let channel_index = i % self.channels;

            let scale_factor = scale_factors[channel_index] as usize;
            let quantized = residual as usize;
            let dequantized = dqts[scale_factor][quantized];

            let reconstructed = lms[channel_index].predict_update(dequantized);

            output.push(reconstructed);
        }
    }

    pub fn decode_vbr(&self, chunk: &SeaChunk, output: &mut Vec<i16>) {
        assert_eq!(chunk.scale_factor_bits as usize, self.scale_factor_bits);

        output.reserve(chunk.frames_per_chunk * self.channels);

        let mut lms = chunk.lms.clone();

        let dqts: &Vec<Vec<Vec<i32>>> = &(1..=8)
            .map(|i| self.dequant_tab.get_dqt(i).clone())
            .collect();

        let mut base_scale_factors = chunk.scale_factors.iter();
        let mut base_vbr_residual_sizes = chunk.vbr_residual_sizes.iter();

        let mut scale_factors = Vec::with_capacity(self.channels);
        let mut vbr_residual_sizes = Vec::with_capacity(self.channels);

        for (i, residual) in chunk.residuals.iter().enumerate() {
            let scale_factor_index = i % (self.channels * chunk.scale_factor_frames as usize);
            // get next subchunk data
            if scale_factor_index == 0 {
                scale_factors.clear();
                scale_factors.extend(base_scale_factors.by_ref().take(self.channels));
                vbr_residual_sizes.clear();
                vbr_residual_sizes.extend(base_vbr_residual_sizes.by_ref().take(self.channels))
            }
            let channel_index = i % self.channels;

            let residual_size = vbr_residual_sizes[channel_index] as usize;
            let scale_factor = scale_factors[channel_index] as usize;
            let quantized = residual as usize;
            let dequantized = dqts[residual_size - 1][scale_factor][quantized];

            let reconstructed = lms[channel_index].predict_update(dequantized);

            output.push(reconstructed);
        }
    }
}
