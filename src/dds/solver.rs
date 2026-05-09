use std::ffi::CStr;

use super::ffi;
use crate::core::deal::{Deal, Direction, Vulnerability};
use crate::core::error::Error;
use crate::core::par::ParResult;
use crate::core::pbn;
use crate::core::tricks::TricksMatrix;

pub struct DdsSolver;

impl DdsSolver {
    /// Initialize DDS threading. Call once at startup.
    /// `0` lets DDS auto-configure based on available cores and memory.
    pub fn init() {
        unsafe { ffi::SetMaxThreads(0); }
    }

    /// Compute the full 20-result tricks matrix for a fresh deal.
    pub fn solve_table(deal: &Deal) -> Result<TricksMatrix, Error> {
        let pbn_str = pbn::deal_to_dds_pbn(deal);

        // Reject strings that don't fit in DDS's 80-byte buffer.
        if pbn_str.len() >= 80 {
            return Err(Error::DdsBufferTooLong {
                field: "ddTableDealPBN.cards",
                len: pbn_str.len(),
                max: 79,
            });
        }

        // Populate the C struct.
        let mut deal_pbn: ffi::ddTableDealPBN = unsafe { std::mem::zeroed() };
        let pbn_bytes = pbn_str.as_bytes();
        for (i, &b) in pbn_bytes.iter().enumerate() {
            deal_pbn.cards[i] = b as std::os::raw::c_char;
        }
        // C string is null-terminated; zeroed() already fills the rest with 0.

        let mut table: ffi::ddTableResults = unsafe { std::mem::zeroed() };

        let rc = unsafe { ffi::CalcDDtablePBN(deal_pbn, &mut table) };
        if rc != 1 {
            let msg = Self::error_message(rc);
            return Err(Error::Dds(msg));
        }

        Ok(TricksMatrix::from_dds(&table.resTable))
    }

    /// Compute par contract and score from a DD results table.
    pub fn compute_par(
        table: &TricksMatrix,
        dealer: Direction,
        vul: Vulnerability,
    ) -> Result<ParResult, Error> {
        let raw_table = table.to_dds();
        let mut dd_table = ffi::ddTableResults {
            resTable: raw_table,
        };
        let mut par: ffi::parResultsDealer = unsafe { std::mem::zeroed() };

        let rc = unsafe {
            ffi::DealerPar(
                &mut dd_table,
                &mut par,
                dealer.dds_index() as i32,
                vul.dds_code(),
            )
        };
        if rc != 1 {
            let msg = Self::error_message(rc);
            return Err(Error::Dds(msg));
        }

        let contracts: Vec<String> = (0..par.number as usize)
            .map(|i| {
                unsafe {
                    CStr::from_ptr(par.contracts[i].as_ptr())
                }
                .to_string_lossy()
                .into_owned()
            })
            .collect();

        Ok(ParResult {
            score: par.score,
            contracts,
        })
    }

    /// Convert a DDS return code to a human-readable string.
    fn error_message(code: i32) -> String {
        let mut buf: [std::os::raw::c_char; 80] = [0; 80];
        unsafe { ffi::ErrorMessage(code, buf.as_mut_ptr()); }
        unsafe {
            CStr::from_ptr(buf.as_ptr())
                .to_string_lossy()
                .into_owned()
        }
    }
}
