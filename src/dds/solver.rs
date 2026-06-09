use std::ffi::CStr;
use std::sync::{Mutex, Once};

use super::ffi;
use crate::core::deal::{Card, Deal, Direction, Rank, Strain, Suit, Vulnerability};
use crate::core::error::Error;
use crate::core::par::ParResult;
use crate::core::pbn;
use crate::core::position::{Position, PositionKind};
use crate::core::tricks::TricksMatrix;

static DDS_INIT: Once = Once::new();

/// Serialize all DDS calls. The DDS library uses shared internal state
/// (transposition tables, thread pools) that is not safe for concurrent
/// access from multiple Rust threads.
static DDS_LOCK: Mutex<()> = Mutex::new(());

/// Result for one legal card from `solve_position`.
pub struct CardResult {
    pub card: Card,
    pub tricks_for_side_to_act: u8,
    pub is_optimal: bool,
}

/// Position matrix: `data[strain][next_to_act]` = tricks for the side to act.
pub struct PositionMatrix {
    pub data: [[u8; 4]; 5],
}

pub struct DdsSolver;

impl DdsSolver {
    /// Initialize DDS threading. Safe to call multiple times; only runs once.
    /// `0` lets DDS auto-configure based on available cores and memory.
    pub fn init() {
        DDS_INIT.call_once(|| unsafe {
            ffi::SetMaxThreads(0);
        });
    }

    /// Compute the full 20-result tricks matrix for a fresh deal.
    pub fn solve_table(deal: &Deal) -> Result<TricksMatrix, Error> {
        let _guard = DDS_LOCK.lock().unwrap();
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
        let _guard = DDS_LOCK.lock().unwrap();
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
                unsafe { CStr::from_ptr(par.contracts[i].as_ptr()) }
                    .to_string_lossy()
                    .into_owned()
            })
            .collect();

        Ok(ParResult {
            score: par.score,
            contracts,
        })
    }

    /// Evaluate all legal continuations from a position.
    pub fn solve_position(position: &Position, trump: Strain) -> Result<Vec<CardResult>, Error> {
        let _guard = DDS_LOCK.lock().unwrap();

        let remain = pbn::hands_to_dds_pbn(&position.hands, position.next_to_act);
        if remain.len() >= 80 {
            return Err(Error::DdsBufferTooLong {
                field: "dealPBN.remainCards",
                len: remain.len(),
                max: 79,
            });
        }

        let mut dds_deal: ffi::dealPBN = unsafe { std::mem::zeroed() };
        dds_deal.trump = trump.dds_index() as i32;
        dds_deal.first = position.next_to_act.dds_index() as i32;

        for (i, played) in position.current_trick.iter().enumerate() {
            dds_deal.currentTrickSuit[i] = played.card.suit.dds_index() as i32;
            dds_deal.currentTrickRank[i] = played.card.rank.dds_rank();
        }

        let remain_bytes = remain.as_bytes();
        for (i, &b) in remain_bytes.iter().enumerate() {
            dds_deal.remainCards[i] = b as std::os::raw::c_char;
        }

        let mut fut: ffi::futureTricks = unsafe { std::mem::zeroed() };
        let rc = unsafe { ffi::SolveBoardPBN(dds_deal, -1, 3, 1, &mut fut, 0) };
        if rc != 1 {
            let msg = Self::error_message(rc);
            return Err(Error::Dds(msg));
        }

        // Map futureTricks to CardResult, expanding equivalent cards.
        let mut results = Vec::new();
        let best_score = (0..fut.cards as usize)
            .map(|i| fut.score[i])
            .max()
            .unwrap_or(-1);

        for i in 0..fut.cards as usize {
            let suit = Suit::from_dds_index(fut.suit[i] as usize)
                .ok_or_else(|| Error::Dds(format!("invalid suit index: {}", fut.suit[i])))?;
            let rank = Rank::from_dds_score(fut.rank[i] as u8)
                .ok_or_else(|| Error::Dds(format!("invalid rank value: {}", fut.rank[i])))?;
            let score = fut.score[i] as u8;
            let optimal = fut.score[i] == best_score;

            // Primary card.
            results.push(CardResult {
                card: Card::new(suit, rank),
                tricks_for_side_to_act: score,
                is_optimal: optimal,
            });

            // Expand equivalent lower-ranked cards from the equals bitmask.
            let equals = fut.equals[i];
            if equals != 0 {
                for bit in 2..=14 {
                    if (equals >> bit) & 1 != 0 {
                        if let Some(eq_rank) = Rank::from_dds_score(bit as u8) {
                            if eq_rank < rank {
                                results.push(CardResult {
                                    card: Card::new(suit, eq_rank),
                                    tricks_for_side_to_act: score,
                                    is_optimal: optimal,
                                });
                            }
                        }
                    }
                }
            }
        }

        Ok(results)
    }

    /// Evaluate a clean residual snapshot across all `next_to_act` and strain values.
    pub fn solve_position_matrix(snapshot: &Position) -> Result<PositionMatrix, Error> {
        snapshot.validate(PositionKind::EntrySnapshot)?;

        let mut data = [[0u8; 4]; 5];
        #[allow(clippy::needless_range_loop)]
        for next_idx in 0..4 {
            let next = Direction::from_dds_index(next_idx).unwrap();
            let mut pos = snapshot.clone();
            pos.next_to_act = next;

            for strain in Strain::all() {
                let results = Self::solve_position(&pos, strain)?;
                let best = results
                    .iter()
                    .map(|r| r.tricks_for_side_to_act)
                    .max()
                    .unwrap_or(0);
                data[strain.dds_index()][next_idx] = best;
            }
        }

        Ok(PositionMatrix { data })
    }

    /// Convert a DDS return code to a human-readable string.
    fn error_message(code: i32) -> String {
        let mut buf: [std::os::raw::c_char; 80] = [0; 80];
        unsafe {
            ffi::ErrorMessage(code, buf.as_mut_ptr());
        }
        unsafe { CStr::from_ptr(buf.as_ptr()).to_string_lossy().into_owned() }
    }
}
