pub mod router;
pub mod smart_crusher;
pub mod diff;
pub mod log;
pub mod search;
pub mod code;
pub mod rag;
pub mod ccr;
pub mod pipeline;

pub use router::{ContentKind, ContentRouter};
pub use smart_crusher::SmartCrusher;
pub use diff::DiffCompressor;
pub use log::LogCompressor;
pub use search::SearchCompressor;
pub use code::CodeCompressor;
pub use rag::{RagCompressor, ConfigCompressor};
pub use ccr::CcrStore;
pub use pipeline::{HeadroomPipeline, CompressionStats};
