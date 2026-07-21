use std::{
    cell::Cell,
    marker::PhantomData,
    path::Path,
    sync::{Mutex, MutexGuard},
};

use libloading::Library;

use crate::{Error, Result};

type Word = u32;
type GenericFn = unsafe extern "C" fn(Word, Word, Word, Word, Word, Word) -> Word;

struct Api {
    emu_initialize: GenericFn,
    emu_terminate: GenericFn,
    initialize: GenericFn,
    terminate: GenericFn,
    create: GenericFn,
    delete: GenericFn,
    load: GenericFn,
    unload: GenericFn,
    open: GenericFn,
    close: GenericFn,
    standby: GenericFn,
    start: GenericFn,
    stop: GenericFn,
    generate: GenericFn,
}

/// Interleaved signed 16-bit stereo PCM returned by [`MaSound::render_mmf`].
#[derive(Debug, Clone)]
pub struct RenderedPcm {
    pub sample_rate: u32,
    pub samples: Vec<i16>,
}

impl RenderedPcm {
    pub fn frames(&self) -> usize {
        self.samples.len() / 2
    }

    pub fn peak(&self) -> u16 {
        self.samples
            .iter()
            .map(|sample| sample.unsigned_abs())
            .max()
            .unwrap_or(0)
    }
}

/// Safe owner for the process-global `M5_EmuSmw5.dll` runtime.
///
/// The DLL is 32-bit and loads `M5_EmuHw.dll` itself. Keep both user-supplied
/// DLLs discoverable by the Windows loader, normally in the same directory.
pub struct MaSound {
    api: Api,
    _library: Library,
    _guard: MutexGuard<'static, ()>,
    emu_initialized: bool,
    initialized: bool,
    _not_sync: PhantomData<Cell<()>>,
}

static INSTANCE_LOCK: Mutex<()> = Mutex::new(());

impl MaSound {
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let guard = INSTANCE_LOCK
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let library = unsafe { Library::new(path.as_ref()) }.map_err(Error::LoadLibrary)?;
        let api = unsafe { Api::load(&library)? };
        Ok(Self {
            api,
            _library: library,
            _guard: guard,
            emu_initialized: false,
            initialized: false,
            _not_sync: PhantomData,
        })
    }

    /// Renders an MMF buffer through the vendor Smw5/Hw DLL pair.
    pub fn render_mmf(
        &mut self,
        mmf: &[u8],
        sample_rate: u32,
        frames: u64,
        chunk_frames: usize,
    ) -> Result<RenderedPcm> {
        let mmf_len = u32::try_from(mmf.len()).map_err(|_| Error::MmfTooLarge(mmf.len()))?;
        let mut session = self.start_session(mmf.as_ptr() as Word, mmf_len, sample_rate)?;
        let mut samples = Vec::with_capacity(
            usize::try_from(frames)
                .unwrap_or(usize::MAX / 2)
                .saturating_mul(2),
        );
        let chunk_frames = chunk_frames.max(1);
        let mut rendered = 0u64;
        while rendered < frames {
            let count = (frames - rendered).min(chunk_frames as u64) as usize;
            let mut left = vec![0i16; count];
            let mut right = vec![0i16; count];
            session.generate(&mut left, &mut right)?;
            for (left, right) in left.into_iter().zip(right) {
                samples.push(left);
                samples.push(right);
            }
            rendered += count as u64;
        }
        drop(session);
        Ok(RenderedPcm {
            sample_rate,
            samples,
        })
    }

    fn start_session(
        &mut self,
        mmf_ptr: Word,
        mmf_len: Word,
        sample_rate: u32,
    ) -> Result<Session<'_>> {
        self.ensure_initialized(sample_rate)?;

        let slot = 1u32;
        let created = call(&self.api, self.api.create, [slot, 0, 0, 0, 0, 0]);
        if created == 0 || created == u32::MAX {
            return Err(Error::ApiCallFailed {
                name: "MaSound_Create",
                code: created,
            });
        }
        let handle = call(&self.api, self.api.load, [slot, mmf_ptr, mmf_len, 1, 0, 0]);
        if handle == 0 || handle == u32::MAX {
            let _ = call(&self.api, self.api.delete, [slot, 0, 0, 0, 0, 0]);
            return Err(Error::ApiCallFailed {
                name: "MaSound_Load",
                code: handle,
            });
        }
        let mut session = Session {
            owner: self,
            slot,
            handle,
            started: false,
            opened: false,
            loaded: true,
            created: true,
        };
        call_zero(
            &session.owner.api,
            "MaSound_Open",
            session.owner.api.open,
            [slot, handle, 0, 0, 0, 0],
        )?;
        session.opened = true;
        call_zero(
            &session.owner.api,
            "MaSound_Standby",
            session.owner.api.standby,
            [slot, handle, 0, 0, 0, 0],
        )?;
        call_zero(
            &session.owner.api,
            "MaSound_Start",
            session.owner.api.start,
            [slot, handle, 1, 0, 0, 0],
        )?;
        session.started = true;
        Ok(session)
    }

    fn ensure_initialized(&mut self, sample_rate: u32) -> Result<()> {
        if self.initialized {
            return Ok(());
        }
        call_zero(
            &self.api,
            "MaSound_EmuInitialize",
            self.api.emu_initialize,
            [sample_rate, 0, 0, 0, 0, 0],
        )?;
        self.emu_initialized = true;
        call_zero(
            &self.api,
            "MaSound_Initialize",
            self.api.initialize,
            [
                callback_a as *const () as usize as Word,
                callback_b as *const () as usize as Word,
                0,
                0,
                0,
                0,
            ],
        )?;
        self.initialized = true;
        Ok(())
    }
}

impl Drop for MaSound {
    fn drop(&mut self) {
        if self.initialized {
            let _ = call(&self.api, self.api.terminate, [0; 6]);
        }
        if self.emu_initialized {
            let _ = call(&self.api, self.api.emu_terminate, [0; 6]);
        }
    }
}

struct Session<'a> {
    owner: &'a mut MaSound,
    slot: u32,
    handle: u32,
    started: bool,
    opened: bool,
    loaded: bool,
    created: bool,
}

impl Session<'_> {
    fn generate(&mut self, left: &mut [i16], right: &mut [i16]) -> Result<()> {
        if left.len() != right.len() {
            return Err(Error::InvalidBufferLengths {
                left: left.len(),
                right: right.len(),
            });
        }
        let frames =
            u32::try_from(left.len()).map_err(|_| Error::FrameCountTooLarge(left.len()))?;
        let _generated = call(
            &self.owner.api,
            self.owner.api.generate,
            [
                left.as_mut_ptr() as Word,
                right.as_mut_ptr() as Word,
                frames,
                0,
                0,
                0,
            ],
        );
        Ok(())
    }
}

impl Drop for Session<'_> {
    fn drop(&mut self) {
        let api = &self.owner.api;
        if self.started {
            let _ = call(api, api.stop, [self.slot, self.handle, 0, 0, 0, 0]);
        }
        if self.opened {
            let _ = call(api, api.close, [self.slot, self.handle, 0, 0, 0, 0]);
        }
        if self.loaded {
            let _ = call(api, api.unload, [self.slot, self.handle, 0, 0, 0, 0]);
        }
        if self.created {
            let _ = call(api, api.delete, [self.slot, 0, 0, 0, 0, 0]);
        }
    }
}

unsafe extern "C" fn callback_a() {}
unsafe extern "C" fn callback_b() {}

fn call(_api: &Api, function: GenericFn, args: [Word; 6]) -> Word {
    unsafe { function(args[0], args[1], args[2], args[3], args[4], args[5]) }
}

fn call_zero(api: &Api, name: &'static str, function: GenericFn, args: [Word; 6]) -> Result<()> {
    let code = call(api, function, args);
    if code == 0 {
        Ok(())
    } else {
        Err(Error::ApiCallFailed { name, code })
    }
}

impl Api {
    unsafe fn load(library: &Library) -> Result<Self> {
        macro_rules! symbol {
            ($name:literal) => {{
                let symbol = unsafe { library.get::<GenericFn>(concat!($name, "\0").as_bytes()) }
                    .map_err(|source| Error::MissingExport {
                    name: $name,
                    source,
                })?;
                *symbol
            }};
        }
        Ok(Self {
            emu_initialize: symbol!("MaSound_EmuInitialize"),
            emu_terminate: symbol!("MaSound_EmuTerminate"),
            initialize: symbol!("MaSound_Initialize"),
            terminate: symbol!("MaSound_Terminate"),
            create: symbol!("MaSound_Create"),
            delete: symbol!("MaSound_Delete"),
            load: symbol!("MaSound_Load"),
            unload: symbol!("MaSound_Unload"),
            open: symbol!("MaSound_Open"),
            close: symbol!("MaSound_Close"),
            standby: symbol!("MaSound_Standby"),
            start: symbol!("MaSound_Start"),
            stop: symbol!("MaSound_Stop"),
            generate: symbol!("MaSound_Generate"),
        })
    }
}
