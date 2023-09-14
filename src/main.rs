mod gen;
mod wav;

pub fn lerp(t: f64, s: f64, e: f64) -> f64 {
    s * (1. - t) + e * t
}

fn main() -> std::io::Result<()> {
    let channels = 1;
    let samplerate = 44100;
    let bytes_per_sample = 2;

    let data = generate_pcm(channels, samplerate, bytes_per_sample);

    wav::WaveDesc::from_data(
        channels as u16,
        samplerate as u32,
        (bytes_per_sample * 8) as u16,
    )
    .write(&data, &mut std::fs::File::create("test.wav")?)
}

fn generate_pcm(channels: usize, samplerate: usize, bytes_per_sample: usize) -> Vec<u8> {
    let seconds = 5;
    let samples = samplerate * seconds;

    let mut data = vec![0; samples * channels * bytes_per_sample];
    for i in 0..samples {
        let offs = i * channels * bytes_per_sample;
        let t = i as f64 / samplerate as f64;

        for channel in 0..channels {
            let gi = gen::GenerationInfo::new(channel, t, Some(seconds as f64), channels);

            let sample: i16 = gen::get_sample(gi);

            data[offs + channel * bytes_per_sample..offs + (channel + 1) * bytes_per_sample]
                .copy_from_slice(&sample.to_le_bytes());
        }
    }

    data
}
