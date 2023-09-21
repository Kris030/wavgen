use crate::gen::{self, GenInfo, Song};

pub fn generate_pcm(
    song: &mut Song,
    channels: usize,
    samplerate: usize,
    bytes_per_sample: usize,
) -> Vec<u8> {
    let seconds = 5.;
    let samples = (samplerate as f64 * seconds) as usize;

    let mut data = vec![0; samples * channels * bytes_per_sample];
    for i in 0..samples {
        let offs = i * channels * bytes_per_sample;
        let t = i as f64 / samplerate as f64;

        for channel in 0..channels {
            let gi = GenInfo {
                channel,
                t: t / song.length,
            };

            let sample = gen::get_sample(song, gi);

            let sample = (sample * i16::MAX as f64).round() as i16;
            let sample = sample.clamp(i16::MIN, i16::MAX);

            data[offs + channel * bytes_per_sample..offs + (channel + 1) * bytes_per_sample]
                .copy_from_slice(&sample.to_le_bytes());
        }
    }

    data
}
