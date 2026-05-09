#![allow(non_snake_case)]

use std::ffi::c_char;
use std::os::raw::c_int;

/// PBN deal input for `CalcDDtablePBN`. Single 80-byte buffer containing
/// the full deal string: four hands separated by spaces, each hand is
/// four suit strings separated by dots, e.g. `"W:T5.K4... K6.QJT9..."`
#[repr(C)]
pub struct ddTableDealPBN {
    pub cards: [c_char; 80],
}

/// 20-result tricks table. `resTable[strain 0=S..4=NT][declarer 0=N..3=W]`.
#[repr(C)]
pub struct ddTableResults {
    pub resTable: [[c_int; 4]; 5],
}

/// Structured par result from `DealerPar()`.
#[repr(C)]
pub struct parResultsDealer {
    /// Count of par contracts (1-10).
    pub number: c_int,
    /// Score from NS perspective.
    pub score: c_int,
    /// Par contract strings, e.g. `"4S-NS"`, `"5C*-NS-2"`.
    pub contracts: [[c_char; 10]; 10],
}

extern "C" {
    /// Auto-configure threads. Call once at startup. 0 = let DDS decide.
    pub fn SetMaxThreads(userThreads: c_int);

    /// Compute the 20-result double-dummy table for a fresh deal.
    /// `tableDealPBN` is passed by value per the DDS API.
    pub fn CalcDDtablePBN(tableDealPBN: ddTableDealPBN, tablep: *mut ddTableResults) -> c_int;

    /// Compute par score and contracts from a DD table, dealer-aware.
    /// `dealer`: 0=N, 1=E, 2=S, 3=W. `vulnerable`: 0=None, 1=Both, 2=NS, 3=EW.
    pub fn DealerPar(
        tablep: *mut ddTableResults,
        presp: *mut parResultsDealer,
        dealer: c_int,
        vulnerable: c_int,
    ) -> c_int;

    /// Convert a DDS return code to a human-readable string.
    pub fn ErrorMessage(code: c_int, line: *mut c_char);
}
