use std::fs::File;
use std::io;
use std::os::fd::AsRawFd;
use std::path::Path;
use std::ptr::NonNull;
use std::sync::atomic::{fence, AtomicU32, Ordering};
use std::time::{Duration, Instant};

pub const DEFAULT_RING_PATH: &str = "/tmp/open-xiaoai-pcm.ring";
pub const PCM_MAGIC: u32 = 0x4f_58_50_31;
pub const PCM_VERSION: u32 = 1;
pub const PCM_RATE: u32 = 16_000;
pub const PCM_SAMPLES: usize = 160;
pub const PCM_SLOTS: usize = 256;
const HEADER_WORDS: usize = 10;
const HEADER_SIZE: usize = HEADER_WORDS * size_of::<u32>();
const BLOCK_SIZE: usize = size_of::<u32>() + PCM_SAMPLES * size_of::<i16>();
pub const PCM_RING_SIZE: usize = HEADER_SIZE + PCM_SLOTS * BLOCK_SIZE;

const MAGIC: usize = 0;
const VERSION: usize = 1;
const RATE: usize = 2;
const BLOCK_SAMPLES: usize = 3;
const SLOTS: usize = 4;
const PRODUCER_PID: usize = 5;
const PUBLISHED: usize = 6;
const ERRORS: usize = 7;
const INPUT_BYTES: usize = 8;
const OUTPUT_BYTES: usize = 9;

pub struct PcmTapReader {
    mapping: NonNull<u8>,
    mapping_len: usize,
    producer_pid: u32,
    read_frame: u32,
    last_progress: Instant,
    stall_timeout: Duration,
}

unsafe impl Send for PcmTapReader {}

impl PcmTapReader {
    pub fn open(path: impl AsRef<Path>) -> io::Result<Self> {
        Self::open_with_timeout(path, Duration::from_secs(2))
    }

    fn open_with_timeout(path: impl AsRef<Path>, stall_timeout: Duration) -> io::Result<Self> {
        let file = File::open(path)?;
        let metadata = file.metadata()?;
        if !metadata.is_file() || metadata.len() != PCM_RING_SIZE as u64 {
            return Err(invalid_data("unexpected PCM tap ring size"));
        }

        let address = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                PCM_RING_SIZE,
                libc::PROT_READ,
                libc::MAP_SHARED,
                file.as_raw_fd(),
                0,
            )
        };
        if address == libc::MAP_FAILED {
            return Err(io::Error::last_os_error());
        }
        let mapping = NonNull::new(address.cast::<u8>()).expect("mmap returned null");

        let mut reader = Self {
            mapping,
            mapping_len: PCM_RING_SIZE,
            producer_pid: 0,
            read_frame: 0,
            last_progress: Instant::now(),
            stall_timeout,
        };
        if let Err(error) = reader.validate_header() {
            drop(reader);
            return Err(error);
        }

        reader.producer_pid = reader.load_header(PRODUCER_PID, Ordering::Acquire);
        if reader.producer_pid == 0 || !process_exists(reader.producer_pid) {
            drop(reader);
            return Err(invalid_data("PCM tap producer is not alive"));
        }
        reader.read_frame = reader.load_header(PUBLISHED, Ordering::Acquire);
        Ok(reader)
    }

    pub fn producer_pid(&self) -> u32 {
        self.producer_pid
    }

    pub fn observed_shape(&self) -> (u32, u32) {
        (
            self.load_header(INPUT_BYTES, Ordering::Acquire),
            self.load_header(OUTPUT_BYTES, Ordering::Acquire),
        )
    }

    pub fn error_count(&self) -> u32 {
        self.load_header(ERRORS, Ordering::Acquire)
    }

    pub fn next_block(&mut self) -> io::Result<Option<Vec<u8>>> {
        self.validate_runtime()?;
        let published = self.load_header(PUBLISHED, Ordering::Acquire);
        let queued = published.wrapping_sub(self.read_frame);
        if queued > PCM_SLOTS as u32 {
            return Err(invalid_data("PCM tap consumer was overwritten"));
        }
        if queued == 0 {
            if self.last_progress.elapsed() > self.stall_timeout {
                return Err(io::Error::new(io::ErrorKind::TimedOut, "PCM tap stalled"));
            }
            return Ok(None);
        }

        let block = self.read_frame as usize % PCM_SLOTS;
        let block_offset = HEADER_SIZE + block * BLOCK_SIZE;
        let expected = self.read_frame.wrapping_mul(2).wrapping_add(2);
        let sequence = self.load_u32(block_offset, Ordering::Acquire);
        if sequence != expected {
            return Err(invalid_data("PCM tap block sequence mismatch"));
        }

        let sample_offset = block_offset + size_of::<u32>();
        let mut bytes = vec![0u8; PCM_SAMPLES * size_of::<i16>()];
        unsafe {
            std::ptr::copy_nonoverlapping(
                self.mapping.as_ptr().add(sample_offset),
                bytes.as_mut_ptr(),
                bytes.len(),
            );
        }
        fence(Ordering::SeqCst);
        if self.load_u32(block_offset, Ordering::Acquire) != expected {
            return Err(invalid_data("PCM tap block changed while reading"));
        }

        self.read_frame = self.read_frame.wrapping_add(1);
        self.last_progress = Instant::now();
        Ok(Some(bytes))
    }

    fn validate_header(&self) -> io::Result<()> {
        if self.load_header(MAGIC, Ordering::Acquire) != PCM_MAGIC
            || self.load_header(VERSION, Ordering::Relaxed) != PCM_VERSION
            || self.load_header(RATE, Ordering::Relaxed) != PCM_RATE
            || self.load_header(BLOCK_SAMPLES, Ordering::Relaxed) != PCM_SAMPLES as u32
            || self.load_header(SLOTS, Ordering::Relaxed) != PCM_SLOTS as u32
        {
            return Err(invalid_data("unsupported PCM tap header"));
        }
        Ok(())
    }

    fn validate_runtime(&self) -> io::Result<()> {
        self.validate_header()?;
        let producer_pid = self.load_header(PRODUCER_PID, Ordering::Acquire);
        if producer_pid != self.producer_pid || !process_exists(producer_pid) {
            return Err(invalid_data("PCM tap producer changed or exited"));
        }
        Ok(())
    }

    fn load_header(&self, word: usize, ordering: Ordering) -> u32 {
        self.load_u32(word * size_of::<u32>(), ordering)
    }

    fn load_u32(&self, offset: usize, ordering: Ordering) -> u32 {
        debug_assert!(offset + size_of::<u32>() <= self.mapping_len);
        let atomic = unsafe { &*self.mapping.as_ptr().add(offset).cast::<AtomicU32>() };
        atomic.load(ordering)
    }
}

impl Drop for PcmTapReader {
    fn drop(&mut self) {
        unsafe {
            libc::munmap(self.mapping.as_ptr().cast(), self.mapping_len);
        }
    }
}

fn process_exists(pid: u32) -> bool {
    let result = unsafe { libc::kill(pid as libc::pid_t, 0) };
    result == 0 || io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)
}

fn invalid_data(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::OpenOptions;
    use std::os::unix::fs::FileExt;
    use std::sync::atomic::AtomicU64;

    static NEXT_FILE: AtomicU64 = AtomicU64::new(0);

    struct RingFixture {
        path: std::path::PathBuf,
        file: File,
    }

    impl RingFixture {
        fn new() -> Self {
            let id = NEXT_FILE.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "open-xiaoai-pcm-ring-{}-{}",
                std::process::id(),
                id
            ));
            let file = OpenOptions::new()
                .read(true)
                .write(true)
                .create_new(true)
                .open(&path)
                .unwrap();
            file.set_len(PCM_RING_SIZE as u64).unwrap();
            let fixture = Self { path, file };
            fixture.header(MAGIC, PCM_MAGIC);
            fixture.header(VERSION, PCM_VERSION);
            fixture.header(RATE, PCM_RATE);
            fixture.header(BLOCK_SAMPLES, PCM_SAMPLES as u32);
            fixture.header(SLOTS, PCM_SLOTS as u32);
            fixture.header(PRODUCER_PID, std::process::id());
            fixture
        }

        fn header(&self, word: usize, value: u32) {
            self.file
                .write_all_at(&value.to_ne_bytes(), (word * size_of::<u32>()) as u64)
                .unwrap();
        }

        fn publish(&self, frame: u32, sample: i16) {
            let block = frame as usize % PCM_SLOTS;
            let offset = HEADER_SIZE + block * BLOCK_SIZE;
            self.file
                .write_all_at(
                    &frame.wrapping_mul(2).wrapping_add(1).to_ne_bytes(),
                    offset as u64,
                )
                .unwrap();
            let samples = vec![sample; PCM_SAMPLES];
            let bytes = unsafe {
                std::slice::from_raw_parts(
                    samples.as_ptr().cast::<u8>(),
                    samples.len() * size_of::<i16>(),
                )
            };
            self.file
                .write_all_at(bytes, (offset + size_of::<u32>()) as u64)
                .unwrap();
            self.file
                .write_all_at(
                    &frame.wrapping_mul(2).wrapping_add(2).to_ne_bytes(),
                    offset as u64,
                )
                .unwrap();
            self.header(PUBLISHED, frame.wrapping_add(1));
        }
    }

    impl Drop for RingFixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.path);
        }
    }

    #[test]
    fn ring_size_is_bounded() {
        assert_eq!(PCM_RING_SIZE, 82_984);
        assert!(PCM_RING_SIZE < 128 * 1024);
    }

    #[test]
    fn reader_starts_at_latest_frame() {
        let fixture = RingFixture::new();
        fixture.publish(0, 1);
        let mut reader = PcmTapReader::open(&fixture.path).unwrap();
        assert!(reader.next_block().unwrap().is_none());
        fixture.publish(1, 0x1234);
        let block = reader.next_block().unwrap().unwrap();
        assert_eq!(&block[..2], &0x1234i16.to_ne_bytes());
    }

    #[test]
    fn rejects_invalid_header() {
        let fixture = RingFixture::new();
        fixture.header(RATE, 48_000);
        assert!(PcmTapReader::open(&fixture.path).is_err());
    }

    #[test]
    fn detects_overwrite() {
        let fixture = RingFixture::new();
        let mut reader = PcmTapReader::open(&fixture.path).unwrap();
        fixture.header(PUBLISHED, PCM_SLOTS as u32 + 1);
        assert!(reader.next_block().is_err());
    }

    #[test]
    fn detects_stall() {
        let fixture = RingFixture::new();
        let mut reader = PcmTapReader::open_with_timeout(&fixture.path, Duration::ZERO).unwrap();
        assert_eq!(
            reader.next_block().unwrap_err().kind(),
            io::ErrorKind::TimedOut
        );
    }
}
