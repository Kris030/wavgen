#![feature(try_trait_v2)]

pub mod gen;
pub mod parse;
pub mod pcm;
pub mod wav;

fn main() -> anyhow::Result<()> {
    let mut args = std::env::args();
    let _ = args.next();
    let source_file = args.next().unwrap_or_else(|| "test_format.txt".to_string());
    let output_file = args.next().unwrap_or_else(|| "test.wav".to_string());

    let sample_rate = 44100;
    let bytes_per_sample = 2;

    let source = std::fs::read_to_string(&source_file)?;
    let mut song = parse::get_song(&source_file, &source)?;

    let data = pcm::generate_pcm(&mut song, sample_rate, bytes_per_sample);

    wav::write_to_wav(
        song.channels,
        sample_rate,
        bytes_per_sample,
        &data,
        std::fs::File::create(output_file)?,
    )?;

    Ok(())
}
