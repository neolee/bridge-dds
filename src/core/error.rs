use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("PBN parse error: {0}")]
    PbnParse(String),

    #[error("missing required PBN tag: {0}")]
    MissingPbnTag(&'static str),

    #[error("duplicate PBN tag: {0}")]
    DuplicatePbnTag(&'static str),

    #[error("invalid PBN tag {tag}: {value}")]
    InvalidPbnTag { tag: &'static str, value: String },

    #[error("unsupported PBN feature: {0}")]
    UnsupportedPbnFeature(String),

    #[error("invalid deal: {0}")]
    InvalidDeal(String),

    #[error("DDS buffer too long for {field}: {len} bytes, max {max}")]
    DdsBufferTooLong {
        field: &'static str,
        len: usize,
        max: usize,
    },

    #[error("DDS error: {0}")]
    Dds(String),

    #[error("invalid vulnerability '{0}'; expected one of: none, ns, ew, both")]
    InvalidVulnerability(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}
