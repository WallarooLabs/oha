use std::fs;
use std::io;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::OnceLock;

use http_body_util::Full;

static DATA_RING: OnceLock<DataRing> = OnceLock::new();

#[derive(Debug)]
pub(crate) struct BodyBuilder;

impl BodyBuilder {
    pub(crate) fn new() -> Self {
        Self
    }

    pub(crate) fn next_body(&self) -> (Id, Full<Body>) {
        DATA_RING
            .get()
            .map(|data| data.next_item())
            .map(|(idx, bytes)| (Id(idx), Full::new(Body::from(bytes))))
            .unwrap_or_default()
    }

    pub(crate) fn body_by_index(&self, idx: Id) -> Full<Body> {
        DATA_RING
            .get()
            .and_then(|data| data.get(idx.0))
            .map(Body::from)
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
    items: Vec<Vec<u8>>,
    idx: AtomicUsize,
}

impl DataRing {
    fn from_vec(buf: Vec<u8>) -> Self {
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
            .map(|line| line.to_string().into_bytes())
            .collect();
        let idx = AtomicUsize::from(0);
        Self { items, idx }
    }

    fn next_item(&self) -> (usize, &[u8]) {
        let idx = self.idx.fetch_add(1, Ordering::SeqCst) % self.items.len();
        (idx, &self.items[idx])
    }

    fn get(&self, index: usize) -> Option<&[u8]> {
        self.items.get(index).map(|bytes| bytes.as_slice())
    }
}

#[derive(Debug)]
pub struct Body {
    buf: hyper::body::Bytes,
}

impl From<&'static [u8]> for Body {
    fn from(bytes: &'static [u8]) -> Self {
        let buf = hyper::body::Bytes::from_static(bytes);
        Self { buf }
    }
}

impl hyper::body::Buf for Body {
    fn remaining(&self) -> usize {
        self.buf.remaining()
    }

    fn chunk(&self) -> &[u8] {
        self.buf.chunk()
    }

    fn advance(&mut self, cnt: usize) {
        self.buf.advance(cnt);
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Id(usize);

impl rusqlite::ToSql for Id {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        let idx = self.0 as i64;
        Ok(idx.into())
    }
}
