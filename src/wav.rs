use std::io::Write;

pub struct WaveDesc {
    channels: u16,
    samplerate: u32,
    bits_per_sample: u16,
}

impl WaveDesc {
    pub fn from_data(channels: u16, samplerate: u32, bits_per_sample: u16) -> Self {
        Self {
            channels,
            samplerate,
            bits_per_sample,
        }
    }

    pub fn write(&self, data: &[u8], mut w: impl Write) -> std::io::Result<()> {
        let samples = data.len() as u32 / self.channels as u32 * self.bits_per_sample as u32;

        let subchunk1_size: u32 = 16;
        let subchunk2_size: u32 = samples * self.channels as u32 * self.bits_per_sample as u32 / 8;

        let chunk_size: u32 = 4 + (8 + subchunk1_size) + (8 + subchunk2_size);

        let byterate: u32 =
            self.samplerate * self.channels as u32 * self.bits_per_sample as u32 / 8;
        let block_align: u16 = self.channels * self.bits_per_sample / 8;

        // ---------- RIFF descriptor ----------
        w.write_all(b"RIFF")?;

        w.write_all(&chunk_size.to_le_bytes())?;
        w.write_all(b"WAVE")?;

        // ---------- fmt chunk ----------
        w.write_all(b"fmt ")?;

        w.write_all(&subchunk1_size.to_le_bytes())?;

        // format = pcm
        w.write_all(&1u16.to_le_bytes())?;
        w.write_all(&self.channels.to_le_bytes())?;

        w.write_all(&self.samplerate.to_le_bytes())?;
        w.write_all(&byterate.to_le_bytes())?;
        w.write_all(&block_align.to_le_bytes())?;
        w.write_all(&self.bits_per_sample.to_le_bytes())?;

        // ---------- data chunk ----------
        w.write_all(b"data")?;
        w.write_all(&subchunk2_size.to_le_bytes())?;

        w.write_all(data)
    }
}
