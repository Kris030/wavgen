use crate::{
    gen::{self, GenInfo, Song},
    parse,
};

pub fn generate_pcm(song: &mut Song, samplerate: usize) -> Result<Vec<u8>, parse::ExpressionError> {
    const BYTES_PER_SAMPLE: usize = std::mem::size_of::<i16>();

    let samples = (samplerate as f64 * song.length_s) as usize;

    let mut data = vec![0; samples * song.channels * BYTES_PER_SAMPLE];
    for i in 0..samples {
        let offs = i * song.channels * BYTES_PER_SAMPLE;
        let t = i as f64 / samplerate as f64;

        for channel in 0..song.channels {
            let gi = GenInfo {
                channel,
                t: t / song.length_s,
            };

            let sample = gen::get_sample(song, gi)?;
            let sample = (sample * i16::MAX as f64) as i16;

            let data_start = offs + channel * BYTES_PER_SAMPLE;
            let data_end = data_start + BYTES_PER_SAMPLE;

            data[data_start..data_end].copy_from_slice(&sample.to_le_bytes());
        }
    }

    Ok(data)
}
