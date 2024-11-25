use std::fs;
use std::io;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::OnceLock;

use http_body_util::Full;

static DATA_RING: OnceLock<DataRing> = OnceLock::new();

#[derive(Debug)]
pub(crate) struct Body;

impl Body {
    pub(crate) fn new() -> Self {
        Self
    }

    pub(crate) fn next_body(&self) -> Full<&'static [u8]> {
        DATA_RING
            .get()
            .map(|data| data.next_item())
            .map(Full::new)
            .unwrap_or_default()
    }

    pub(crate) fn from_string(text: String) -> io::Result<Self> {
        let data = DataRing::from_string(text);
        Self::init(data)
    }

    pub(crate) fn from_file(path: impl AsRef<Path>) -> io::Result<Self> {
        fs::read(path).map(DataRing::from_vec).and_then(Self::init)
    }

    pub(crate) fn from_jsonline(path: impl AsRef<Path>) -> io::Result<Self> {
        fs::read_to_string(path)
            .map(DataRing::from_lines)
            .and_then(Self::init)
    }

    fn init(data: DataRing) -> io::Result<Self> {
        match DATA_RING.set(data) {
            Ok(()) => Ok(Self::new()),
            Err(_) => Err(io::Error::other("Double data init")),
        }
    }
}

struct DataRing {
    items: Vec<&'static mut [u8]>,
    idx: AtomicUsize,
}

impl DataRing {
    fn from_vec(buf: Vec<u8>) -> Self {
        let buf = buf.into_boxed_slice();
        let buf = Box::leak(buf);
        let items = vec![buf];
        let idx = AtomicUsize::from(0);
        Self { items, idx }
    }

    fn from_string(text: String) -> Self {
        Self::from_vec(text.into_bytes())
    }

    fn from_lines(text: String) -> Self {
        let items = text
            .lines()
            .map(|line| line.to_string())
            .map(|text| text.into_bytes().into_boxed_slice())
            .map(Box::leak)
            .collect();
        let idx = AtomicUsize::from(0);
        Self { items, idx }
    }

    fn next_item(&'static self) -> &'static [u8] {
        let idx = self.idx.fetch_add(1, Ordering::SeqCst) % self.items.len();
        self.items[idx]
    }
}
