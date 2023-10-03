use crate::{
    gen::{self, GenInfo, Song},
    parse::ExpressionError,
};

pub fn generate_pcm(
    song: &mut Song,
    samplerate: usize,
    bytes_per_sample: usize,
) -> Result<Vec<u8>, ExpressionError> {
    let samples = (samplerate as f64 * song.length_s) as usize;

    let mut data = vec![0; samples * song.channels * bytes_per_sample];
    for i in 0..samples {
        let offs = i * song.channels * bytes_per_sample;
        let t = i as f64 / samplerate as f64;

        for channel in 0..song.channels {
            let gi = GenInfo {
                channel,
                t: t / song.length_s,
            };

            let sample = gen::get_sample(song, gi)?;
            let sample = (sample * i16::MAX as f64) as i16;

            let data_start = offs + channel * bytes_per_sample;
            let data_end = data_start + bytes_per_sample;
            let data_pos = data_start..data_end;

            data[data_pos].copy_from_slice(&sample.to_le_bytes());
        }
    }

    Ok(data)
}
