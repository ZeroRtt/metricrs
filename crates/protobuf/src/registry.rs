//! A poll-style and in-memory metrics collector implementation.

use std::{
    collections::HashMap,
    fmt::Debug,
    io::{Read, Result, Write as _},
    net::{SocketAddr, TcpListener, TcpStream, ToSocketAddrs},
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

use parking_lot::RwLock;
use protobuf::Message;

use metricrs::{
    Counter, CounterWrite, Gauge, GaugeWrite, Histogram, HistogramWrite, Registry, Token,
};

use crate::protos::memory::{Instrument, Label, Metadata, Query, QueryResult, Value};

#[allow(unused)]
struct Write(Arc<AtomicU64>);

#[allow(unused)]
impl CounterWrite for Write {
    fn increment(&self, step: u64) {
        self.0.fetch_add(step, Ordering::AcqRel);
    }

    fn absolute(&self, value: u64) {
        self.0.swap(value, Ordering::AcqRel);
    }
}

#[allow(unused)]
impl HistogramWrite for Write {
    fn record(&self, value: f64) {
        self.0.swap(value.to_bits(), Ordering::AcqRel);
    }
}

#[allow(unused)]
impl GaugeWrite for Write {
    fn increment(&self, value: f64) {
        self.0
            .fetch_update(Ordering::AcqRel, Ordering::Relaxed, |curr| {
                let input = f64::from_bits(curr);
                let output = input + value;
                Some(output.to_bits())
            });
    }

    fn decrement(&self, value: f64) {
        self.0
            .fetch_update(Ordering::AcqRel, Ordering::Relaxed, |curr| {
                let input = f64::from_bits(curr);
                let output = input - value;
                Some(output.to_bits())
            });
    }

    fn set(&self, value: f64) {
        self.0.swap(value.to_bits(), Ordering::AcqRel);
    }
}

impl<'a> From<(Instrument, Token<'a>)> for Metadata {
    fn from((instrument, value): (Instrument, Token<'a>)) -> Self {
        Self {
            hash: value.hash,
            instrument: instrument.into(),
            name: value.name.to_owned(),
            labels: value
                .labels
                .iter()
                .map(|(k, v)| Label {
                    key: k.to_string(),
                    value: v.to_string(),
                    ..Default::default()
                })
                .collect::<Vec<_>>(),
            ..Default::default()
        }
    }
}

#[derive(Default)]
struct MutableData {
    instruments: HashMap<u64, Arc<AtomicU64>>,
    metadata: HashMap<u64, Metadata>,
    version: u64,
}

/// A builtin in-memory [`Registry`](crate::Registry) implementation works
/// in tandem with the pull-mode data collector.
#[derive(Clone)]
pub struct ProtoBufRegistry {
    local_addr: SocketAddr,
    mutable: Arc<RwLock<MutableData>>,
}

impl Debug for ProtoBufRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProtoBufRegistry")
            .field("local_addr", &self.local_addr)
            .finish_non_exhaustive()
    }
}

impl ProtoBufRegistry {
    /// Local bound listening address.
    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }

    /// Create `MemoryRegistry` and start a `TCP` server to accept remote `status` queries.
    pub fn bind<S>(laddr: S) -> Result<Self>
    where
        S: ToSocketAddrs,
    {
        let listener = TcpListener::bind(laddr)?;

        let registry = ProtoBufRegistry {
            local_addr: listener.local_addr()?,
            mutable: Default::default(),
        };

        let server = registry.clone();

        std::thread::spawn(move || match server.run_server(listener) {
            Ok(_) => log::trace!("`MemoryRegistry` listener is closed."),
            Err(err) => {
                log::error!("`MemoryRegistry` listener is shutdown!!, {}", err);
            }
        });

        Ok(registry)
    }

    fn run_server(self, listener: TcpListener) -> Result<()> {
        loop {
            let (mut stream, from) = listener.accept()?;

            log::trace!("accept a client from {}", from);

            loop {
                match self.handle_query(&mut stream) {
                    Ok(_) => {}
                    Err(err) => {
                        log::trace!("Failed to serve query, {}", err);
                        break;
                    }
                }
            }
        }
    }

    fn handle_query(&self, stream: &mut TcpStream) -> Result<()> {
        let mut buf = [0u8; 4];

        stream.read_exact(&mut buf)?;

        let len = u32::from_be_bytes(buf);

        let mut buf = vec![0u8; len as usize];

        stream.read_exact(&mut buf)?;

        let query = Query::parse_from_bytes(&buf)?;

        let mutable = self.mutable.read();

        let mut metadatas = vec![];
        let mut values = vec![];

        let version = mutable.version;

        if version > query.version {
            for metadata in mutable.metadata.values().clone() {
                metadatas.push(metadata.clone());
            }
        }

        for (hash, value) in mutable.instruments.iter() {
            values.push(Value {
                hash: *hash,
                value: value.load(Ordering::Relaxed),
                ..Default::default()
            });
        }

        drop(mutable);

        let query_result = QueryResult {
            values,
            metadatas,
            version,
            ..Default::default()
        };

        let buf = query_result.write_to_bytes()?;

        let header = (buf.len() as u32).to_be_bytes();

        stream.write_all(&header)?;
        stream.write_all(&buf)?;

        Ok(())
    }

    fn get(&self, instrument: Instrument, token: Token<'_>) -> Arc<AtomicU64> {
        if let Some(counter) = self.mutable.read().instruments.get(&token.hash) {
            return counter.clone();
        }

        let value: Arc<AtomicU64> = Default::default();

        let mut mutable_data = self.mutable.write();

        mutable_data
            .metadata
            .insert(token.hash, Metadata::from((instrument, token)));

        mutable_data.instruments.insert(token.hash, value.clone());

        mutable_data.version += 1;

        value
    }
}

impl Registry for ProtoBufRegistry {
    fn counter(&self, token: Token<'_>) -> Counter {
        Counter::Record(Box::new(Write(self.get(Instrument::COUNTER, token))))
    }

    fn gauge(&self, token: Token<'_>) -> Gauge {
        Gauge::Record(Box::new(Write(self.get(Instrument::GAUGE, token))))
    }

    fn histogam(&self, token: Token<'_>) -> Histogram {
        Histogram::Record(Box::new(Write(self.get(Instrument::HISTOGRAM, token))))
    }
}
