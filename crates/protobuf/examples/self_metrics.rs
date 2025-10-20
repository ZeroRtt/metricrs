use std::{io::Result, thread::sleep, time::Duration};

use metricrs::{global::set_global_registry, instrument};
use metricrs_protobuf::{fetch::Fetch, protos::memory::Query, registry::ProtoBufRegistry};

struct LoopFetch {
    fetch: Fetch,
    verson: u64,
}

impl LoopFetch {
    pub fn run(&mut self) -> Result<()> {
        loop {
            self.fetch_once()?;
            sleep(Duration::from_secs(1));
        }
    }

    #[instrument(kind = Counter)]
    fn fetch_once(&mut self) -> Result<()> {
        let query_result = self.fetch.query(Query {
            version: self.verson,
            ..Default::default()
        })?;

        self.verson = query_result.version;

        log::trace!("{}", query_result);

        Ok(())
    }
}

fn main() -> Result<()> {
    pretty_env_logger::init_timed();

    let registry = ProtoBufRegistry::bind("127.0.0.1:0")?;

    let local_addr = registry.local_addr();

    set_global_registry(registry).unwrap();

    let mut fetch = LoopFetch {
        fetch: Fetch::connect(local_addr)?,
        verson: 0,
    };

    fetch.run()
}
