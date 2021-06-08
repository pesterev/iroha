//! `Maintenance` module provides structures and implementation blocks related to `Iroha`
//! maintenance functions like Healthcheck, Monitoring, etc.

use iroha_derive::Io;
use iroha_error::Result;
use parity_scale_codec::{Decode, Encode};
use serde::{Deserialize, Serialize};

use crate::config::Configuration;

/// Entry point and main entity in `maintenance` API.
/// Provides all information about the system needed for administrators and users.
#[derive(Debug)]
pub struct System {
    configuration: Configuration,
}

impl System {
    /// Default `System` constructor.
    pub fn new(configuration: &Configuration) -> Self {
        System {
            configuration: configuration.clone(),
        }
    }

    /// Scrape current system metrics.
    ///
    /// # Errors
    ///
    pub async fn scrape_metrics(&self) -> Result<Metrics> {
        let mut metrics = Metrics::new(&self.configuration);
        metrics.calculate().await?;
        Ok(metrics)
    }
}

/// `Health` enumerates different variants of Iroha `Peer` states.
/// Each variant can provide additional information if needed.
#[derive(Copy, Clone, Debug, Io, Encode, Decode, Deserialize, Serialize)]
pub enum Health {
    /// `Healthy` variant means that `Peer` has finished initial setup.
    Healthy,
    /// `Ready` variant means that `Peer` bootstrapping completed.
    Ready,
}

/// Metrics struct compose all Iroha metrics and provides an ability to export them in monitoring
/// systems.
#[derive(Clone, Debug, Default, Io, Encode, Decode)]
pub struct Metrics {
    cpu: cpu::Cpu,
    disk: disk::Disk,
    memory: memory::Memory,
}

impl Metrics {
    /// Default `Metrics` constructor.
    pub fn new(configuration: &Configuration) -> Self {
        Metrics {
            disk: disk::Disk::new(&configuration.kura_configuration),
            ..Metrics::default()
        }
    }

    /// Update current `Metrics` state with new data.
    ///
    /// # Errors
    /// Can fail during cpu and memory usage calculations
    pub async fn calculate(&mut self) -> Result<()> {
        self.disk.calculate().await?;
        self.cpu.calculate().await?;
        self.memory.calculate().await?;
        Ok(())
    }
}

mod disk {
    use iroha_derive::Io;
    use iroha_error::{Result, WrapErr};
    use parity_scale_codec::{Decode, Encode};
    use tokio::fs::read_dir;
    use tokio_stream::{wrappers::ReadDirStream, StreamExt};

    use crate::kura::config::KuraConfiguration;

    #[derive(Clone, Debug, Default, Io, Encode, Decode)]
    pub struct Disk {
        block_storage_size: u64,
        block_storage_path: String,
    }

    impl Disk {
        pub fn new(configuration: &KuraConfiguration) -> Self {
            Disk {
                block_storage_path: configuration.kura_block_store_path.clone(),
                ..Disk::default()
            }
        }

        pub async fn calculate(&mut self) -> Result<()> {
            let mut total_size: u64 = 0;
            let mut stream = ReadDirStream::new(
                read_dir(&self.block_storage_path)
                    .await
                    .wrap_err("Failed to read block storage directoru")?,
            );
            while let Some(entry) = stream.next().await {
                let path = entry.wrap_err("Failed to retrieve entry path")?.path();
                if path.is_file() {
                    total_size += path
                        .metadata()
                        .wrap_err("Failed to get file metadata")?
                        .len();
                }
            }
            self.block_storage_size = total_size;
            Ok(())
        }
    }
}

mod cpu {
    use heim::cpu;
    use iroha_derive::Io;
    use iroha_error::Result;
    use parity_scale_codec::{Decode, Encode};

    #[derive(Clone, Debug, Default, Io, Encode, Decode)]
    pub struct Cpu {
        load: Load,
    }

    impl Cpu {
        pub fn new() -> Self {
            Cpu::default()
        }

        #[iroha_futures::telemetry_future]
        pub async fn calculate(&mut self) -> Result<()> {
            self.load.calculate().await
        }
    }

    #[derive(Clone, Debug, Default, Io, Encode, Decode)]
    pub struct Load {
        frequency: String,
        stats: String,
        time: String,
    }

    impl Load {
        pub fn new() -> Self {
            Load::default()
        }

        /// Calculates cpu usage
        ///
        /// # Errors
        /// Can fail during computing metrics
        #[iroha_futures::telemetry_future]
        pub async fn calculate(&mut self) -> Result<()> {
            self.frequency = format!("{:?}", cpu::frequency().await);
            self.stats = format!("{:?}", cpu::stats().await);
            self.time = format!("{:?}", cpu::time().await);
            Ok(())
        }
    }
}

mod memory {
    use heim::memory;
    use iroha_derive::Io;
    use iroha_error::Result;
    use parity_scale_codec::{Decode, Encode};

    #[derive(Clone, Debug, Default, Io, Encode, Decode)]
    pub struct Memory {
        memory: String,
        swap: String,
    }

    impl Memory {
        pub fn new() -> Self {
            Memory::default()
        }

        /// Calculates memory usage
        ///
        /// # Errors
        /// Can fail during computing memory metrics
        #[iroha_futures::telemetry_future]
        pub async fn calculate(&mut self) -> Result<()> {
            self.memory = format!("{:?}", memory::memory().await);
            self.swap = format!("{:?}", memory::swap().await);
            Ok(())
        }
    }

    #[cfg(test)]
    mod tests {
        #![allow(clippy::restriction)]

        use super::*;

        #[tokio::test]
        async fn test_calculate_memory() {
            let mut memory = Memory::default();
            memory
                .calculate()
                .await
                .expect("Failed to calculate memory.");
            assert!(!memory.memory.is_empty());
            assert!(!memory.swap.is_empty());
        }
    }
}