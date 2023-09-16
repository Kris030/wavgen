pub mod gen;
pub mod wav;

fn main() -> std::io::Result<()> {
    let mut args = std::env::args();
    let _ = args.next();
    let file = args.next().unwrap_or_else(|| "test.wav".to_string());

    let channels = 1;
    let sample_rate = 44100;
    let bytes_per_sample = 2;

    let data = generate_pcm(channels, sample_rate, bytes_per_sample);

    write_to_wav(channels, sample_rate, bytes_per_sample, &data, &file)?;

    play(channels, sample_rate, data);

    Ok(())
}

fn play(channels: usize, sample_rate: usize, data: Vec<u8>) {
    let (_stream, stream_handle) = rodio::OutputStream::try_default().unwrap();
    let sink = rodio::Sink::try_new(&stream_handle).unwrap();

    sink.append(rodio::buffer::SamplesBuffer::new(
        channels as u16,
        sample_rate as u32,
        unsafe { std::slice::from_raw_parts(data.as_ptr() as *mut i16, data.len() / 2) },
    ));
    sink.sleep_until_end();
}

fn write_to_wav(
    channels: usize,
    sample_rate: usize,
    bytes_per_sample: usize,
    data: &[u8],
    file: &str,
) -> Result<(), std::io::Error> {
    let mut file = std::fs::File::create(file)?;

    let desc = &wav::WaveDesc::from_data(
        channels as u16,
        sample_rate as u32,
        (bytes_per_sample * 8) as u16,
    );

    desc.write(data, &mut file)
}

fn generate_pcm(channels: usize, samplerate: usize, bytes_per_sample: usize) -> Vec<u8> {
    let seconds = 5.;
    let samples = (samplerate as f64 * seconds) as usize;

    let mut data = vec![0; samples * channels * bytes_per_sample];
    for i in 0..samples {
        let offs = i * channels * bytes_per_sample;
        let t = i as f64 / samplerate as f64;

        for channel in 0..channels {
            let gi = gen::GenerationInfo::new(channel, t, Some(seconds), channels);

            let sample: i16 = gen::get_sample(gi);

            data[offs + channel * bytes_per_sample..offs + (channel + 1) * bytes_per_sample]
                .copy_from_slice(&sample.to_le_bytes());
        }
    }

    data
}
