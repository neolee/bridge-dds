use serde::Serialize;

/// Par result returned by DDS `DealerPar()`.
///
/// Contracts are text strings like `"4S-NS"`, `"3N-EW"`, `"5C*-NS-2"`.
#[derive(Debug, Clone, Serialize)]
pub struct ParResult {
    /// Numeric score from NS perspective: positive = NS gain, negative = EW gain.
    pub score: i32,
    /// List of par contract strings, one per alternative (e.g. `["4S-NS", "4H-NS"]`).
    pub contracts: Vec<String>,
}
