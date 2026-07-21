# ma5-native

An independently written Rust interface for dynamically loading a
user-supplied MA-5 software emulator DLL pair.

This repository contains only wrapper code. It does **not** contain, download,
or redistribute Yamaha DLLs, SDK files, headers, sound banks, samples, or other
vendor assets. Users are responsible for obtaining and using the DLL under
terms that apply to them.

The public API loads `M5_EmuSmw5.dll`. That DLL locates and loads
`M5_EmuHw.dll`; this crate does not expose the hardware-register API.

## Platform and build

The observed vendor DLL is a 32-bit Windows PE library. Build consumers for
`i686-pc-windows-msvc` or `i686-pc-windows-gnu`; a 64-bit process cannot load it
directly.

```sh
rustup target add i686-pc-windows-msvc
cargo build --target i686-pc-windows-msvc
```

The DLL path is supplied at runtime and is never embedded in this crate. Both
DLLs normally need to be in the process DLL search directory:

```rust,no_run
use ma5_native::MaSound;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    std::env::set_current_dir(r"C:\path\you\control")?;
    let mmf = std::fs::read(r"C:\input\song.mmf")?;
    let mut sound = MaSound::load("M5_EmuSmw5.dll")?;
    let pcm = sound.render_mmf(&mmf, 48_000, 610_556, 960)?;
    assert_eq!(pcm.samples.len(), pcm.frames() * 2);
    Ok(())
}
```

## Optional render CLI

The crate is library-first: no binary is built by default. Enable `cli` to
build the validation renderer:

```sh
cargo build --release --target i686-pc-windows-msvc \
  --features cli --bin ma5-render

ma5-render.exe \
  --mmf input.mmf \
  --dll-dir C:\path\to\user-supplied-dlls \
  --output output.pcm \
  --frames 610556
```

The output is headerless interleaved stereo signed 16-bit little-endian PCM.
The default sample rate is 48 kHz and the default generation block is 960
frames.

## Compatibility verification

The wrapper lifecycle follows the observed `MaSound_EmuInitialize`,
`MaSound_Initialize`, `Create`, `Load`, `Open`, `Standby`, `Start`, `Generate`,
and reverse cleanup sequence. In the local Wine compatibility test, the Rust
CLI and the independently built C++ oracle rendered the same MMF for 120,000
frames with all 240,000 samples matching exactly.

## Distribution boundary

- Publish the Rust sources, Cargo metadata, documentation, and tests created
  for this project.
- Do not commit or attach vendor binaries, import libraries, SDK headers,
  documentation, sample content, extracted tables, traces, or decompiled code.
- Do not make crates.io build scripts fetch vendor software.
- Keep compatibility tests dependent on a local environment variable and skip
  them when no user-supplied DLL is present.
- Use Yamaha and MA-5 names only to describe compatibility; do not imply
  endorsement or affiliation.

This is an engineering boundary, not legal advice. Before public release,
review the license/EULA under which the DLL and developer kit were obtained.

## License

The wrapper code in this repository is licensed under the MIT License. This
license does not grant rights to any third-party software.
