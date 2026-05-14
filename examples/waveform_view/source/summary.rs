#[cfg(test)]
const SUMMARY_BLOCK_FRAMES: usize = 128;

#[derive(Clone, Debug)]
pub(crate) struct WaveformBand {
    pub(super) samples: Vec<f32>,
    #[cfg(test)]
    summary: WaveformSummary,
}

#[cfg(test)]
#[derive(Clone, Debug)]
pub(crate) struct WaveformSummary {
    blocks: Vec<SummaryBlock>,
}

#[cfg(test)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(super) struct SummaryBlock {
    peak: f32,
    energy: f32,
    count: usize,
}

#[cfg(test)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct BandStats {
    pub(crate) peak: f32,
    pub(crate) rms: f32,
}

#[cfg(test)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct StatsAccumulator {
    peak: f32,
    energy: f32,
    count: usize,
}

impl WaveformBand {
    pub(crate) fn new(samples: Vec<f32>) -> Self {
        #[cfg(test)]
        let summary = WaveformSummary::from_samples(&samples);
        Self {
            samples,
            #[cfg(test)]
            summary,
        }
    }

    #[cfg(test)]
    pub(super) fn stats(&self, start: usize, end: usize) -> BandStats {
        self.summary.stats(&self.samples, start, end)
    }
}

#[cfg(test)]
impl WaveformSummary {
    pub(crate) fn from_samples(samples: &[f32]) -> Self {
        let blocks = samples
            .chunks(SUMMARY_BLOCK_FRAMES)
            .map(SummaryBlock::from_samples)
            .collect();
        Self { blocks }
    }

    pub(crate) fn stats(&self, samples: &[f32], start: usize, end: usize) -> BandStats {
        let start = start.min(samples.len());
        let end = end.min(samples.len()).max(start + 1).min(samples.len());
        if end <= start {
            return BandStats {
                peak: 0.0,
                rms: 0.0,
            };
        }
        if end - start <= SUMMARY_BLOCK_FRAMES * 2 {
            return SummaryBlock::from_samples(&samples[start..end]).into_stats();
        }

        let first_full_block = start.div_ceil(SUMMARY_BLOCK_FRAMES);
        let last_full_block = end / SUMMARY_BLOCK_FRAMES;
        let mut stats = StatsAccumulator::default();
        let left_end = (first_full_block * SUMMARY_BLOCK_FRAMES).min(end);
        stats.add_samples(&samples[start..left_end]);
        for block in &self.blocks[first_full_block..last_full_block] {
            stats.add_block(*block);
        }
        let right_start = (last_full_block * SUMMARY_BLOCK_FRAMES).max(left_end);
        stats.add_samples(&samples[right_start..end]);
        stats.into_stats()
    }
}

#[cfg(test)]
impl SummaryBlock {
    pub(super) fn from_samples(samples: &[f32]) -> Self {
        let mut block = Self::default();
        for sample in samples {
            block.peak = block.peak.max(sample.abs());
            block.energy += sample * sample;
            block.count += 1;
        }
        block
    }

    fn into_stats(self) -> BandStats {
        StatsAccumulator {
            peak: self.peak,
            energy: self.energy,
            count: self.count,
        }
        .into_stats()
    }
}

#[cfg(test)]
impl StatsAccumulator {
    fn add_samples(&mut self, samples: &[f32]) {
        for sample in samples {
            self.peak = self.peak.max(sample.abs());
            self.energy += sample * sample;
            self.count += 1;
        }
    }

    fn add_block(&mut self, block: SummaryBlock) {
        self.peak = self.peak.max(block.peak);
        self.energy += block.energy;
        self.count += block.count;
    }

    fn into_stats(self) -> BandStats {
        BandStats {
            peak: self.peak,
            rms: if self.count == 0 {
                0.0
            } else {
                self.energy / self.count as f32
            },
        }
    }
}

#[cfg(test)]
pub(crate) fn band_stats(samples: &[f32], start: usize, end: usize) -> BandStats {
    let start = start.min(samples.len());
    let end = end.min(samples.len()).max(start + 1).min(samples.len());
    SummaryBlock::from_samples(&samples[start..end]).into_stats()
}
