//! MPEG-2 TS demuxer / muxer.
//!
//! # TS packet level
//!
//! ```no_run
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! use mediumi_mpeg2ts::api::{ts_demuxer, ts_muxer};
//!
//! let data = std::fs::read("input.ts")?;
//! let demuxed = ts_demuxer::demux(&data)?;
//! let output = ts_muxer::mux(&demuxed)?;
//! # Ok(())
//! # }
//! ```
//!
//! # PES stream level
//!
//! ```no_run
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! use mediumi_mpeg2ts::api::{pes_demuxer, pes_muxer};
//!
//! let data = std::fs::read("input.ts")?;
//! let demuxed = pes_demuxer::demux(&data)?;
//! let output = pes_muxer::mux(&demuxed)?;
//! # Ok(())
//! # }
//! ```

pub mod api;
pub mod pes;
pub mod psi;
pub mod ts;

pub mod prelude {
    pub use crate::api::pes_demuxer;
    pub use crate::api::pes_muxer;
    pub use crate::api::ts_demuxer;
    pub use crate::api::ts_muxer;
}
