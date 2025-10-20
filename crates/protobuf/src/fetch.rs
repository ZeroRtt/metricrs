//! Client to poll data from `metrics` collector.

use std::{
    io::{Read, Result, Write},
    net::{TcpStream, ToSocketAddrs},
};

use protobuf::Message;

use crate::protos::memory::{Query, QueryResult};

pub struct Fetch(TcpStream);

impl Fetch {
    /// Create a `Fetch` instance and connect to `metrics`collector.
    pub fn connect<S>(remote_addr: S) -> Result<Self>
    where
        S: ToSocketAddrs,
    {
        let stream = TcpStream::connect(remote_addr)?;

        Ok(Fetch(stream))
    }

    /// Process a metrics query.
    pub fn query(&mut self, query: Query) -> Result<QueryResult> {
        let body = query.write_to_bytes()?;

        let header = (body.len() as u32).to_be_bytes();

        self.0.write_all(&header)?;
        self.0.write_all(&body)?;

        let mut buf = [0u8; 4];

        self.0.read_exact(&mut buf)?;

        let len = u32::from_be_bytes(buf);

        let mut buf = vec![0u8; len as usize];

        self.0.read_exact(&mut buf)?;

        Ok(QueryResult::parse_from_bytes(&buf)?)
    }
}
