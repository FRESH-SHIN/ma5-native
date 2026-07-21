#[cfg(not(all(target_os = "windows", target_arch = "x86")))]
fn main() {
    eprintln!("ma5-render must be built for 32-bit Windows (i686-pc-windows-*)");
    std::process::exit(2);
}

#[cfg(all(target_os = "windows", target_arch = "x86"))]
fn main() {
    if let Err(error) = app::run() {
        eprintln!("ma5-render: {error}");
        std::process::exit(1);
    }
}

#[cfg(all(target_os = "windows", target_arch = "x86"))]
mod app {
    use std::{env, error::Error, fs, path::PathBuf};

    use ma5_native::{MaSound, RenderedPcm};

    struct Args {
        mmf: PathBuf,
        dll_dir: PathBuf,
        output: PathBuf,
        sample_rate: u32,
        frames: u64,
        chunk_frames: usize,
    }

    pub fn run() -> Result<(), Box<dyn Error>> {
        let args = parse_args(env::args().skip(1))?;
        let dll_dir = args.dll_dir.canonicalize()?;
        let dll_path = dll_dir.join("M5_EmuSmw5.dll");
        if !dll_dir.join("M5_EmuHw.dll").is_file() {
            return Err("M5_EmuHw.dll is not present in --dll-dir".into());
        }
        env::set_current_dir(&dll_dir)?;
        let mmf = fs::read(&args.mmf)?;
        let mut sound = MaSound::load(&dll_path)?;
        let pcm = sound.render_mmf(&mmf, args.sample_rate, args.frames, args.chunk_frames)?;
        write_pcm(&args.output, &pcm)?;
        println!(
            "rendered: mmf={} output={} frames={} sample_rate={} peak={}",
            args.mmf.display(),
            args.output.display(),
            pcm.frames(),
            pcm.sample_rate,
            pcm.peak()
        );
        Ok(())
    }

    fn write_pcm(path: &PathBuf, pcm: &RenderedPcm) -> Result<(), Box<dyn Error>> {
        let mut bytes = Vec::with_capacity(pcm.samples.len() * 2);
        for sample in &pcm.samples {
            bytes.extend_from_slice(&sample.to_le_bytes());
        }
        fs::write(path, bytes)?;
        Ok(())
    }

    fn parse_args<I>(mut args: I) -> Result<Args, Box<dyn Error>>
    where
        I: Iterator<Item = String>,
    {
        let mut mmf = None;
        let mut dll_dir = None;
        let mut output = None;
        let mut sample_rate = 48_000u32;
        let mut frames = 610_556u64;
        let mut chunk_frames = 960usize;
        while let Some(arg) = args.next() {
            let value = |args: &mut I, name: &str| {
                args.next()
                    .ok_or_else(|| format!("{name} requires a value"))
            };
            match arg.as_str() {
                "--mmf" => mmf = Some(PathBuf::from(value(&mut args, "--mmf")?)),
                "--dll-dir" => dll_dir = Some(PathBuf::from(value(&mut args, "--dll-dir")?)),
                "--output" | "--pcm-out" => {
                    output = Some(PathBuf::from(value(&mut args, "--output")?))
                }
                "--sample-rate" => sample_rate = value(&mut args, "--sample-rate")?.parse()?,
                "--frames" => frames = value(&mut args, "--frames")?.parse()?,
                "--chunk-frames" => chunk_frames = value(&mut args, "--chunk-frames")?.parse()?,
                "-h" | "--help" => {
                    print_usage();
                    std::process::exit(0);
                }
                _ => return Err(format!("unknown argument: {arg}").into()),
            }
        }
        if chunk_frames == 0 {
            return Err("--chunk-frames must be greater than zero".into());
        }
        Ok(Args {
            mmf: mmf.ok_or("--mmf is required")?,
            dll_dir: dll_dir.ok_or("--dll-dir is required")?,
            output: output.ok_or("--output is required")?,
            sample_rate,
            frames,
            chunk_frames,
        })
    }

    fn print_usage() {
        println!(
            "Usage: ma5-render --mmf INPUT.mmf --dll-dir DIR --output OUT.pcm \
             [--frames N] [--chunk-frames N] [--sample-rate HZ]"
        );
    }
}
