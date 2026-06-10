use std::ffi::CStr;
use std::sync::{Mutex, Once};

use super::ffi;
use crate::core::deal::{Card, Deal, Direction, Rank, Strain, Suit, Vulnerability};
use crate::core::error::Error;
use crate::core::par::ParResult;
use crate::core::pbn;
use crate::core::position::{CurrentTrick, PlayPosition, SnapshotPosition};
use crate::core::tricks::TricksMatrix;

static DDS_INIT: Once = Once::new();
static DDS_LOCK: Mutex<()> = Mutex::new(());

pub struct CardResult {
    pub card: Card,
    pub tricks_for_side_to_act: u8,
    pub is_optimal: bool,
}

pub struct PositionMatrix {
    pub data: [[u8; 4]; 5],
}

pub struct DdsSolver;

impl DdsSolver {
    pub fn init() {
        DDS_INIT.call_once(|| unsafe {
            ffi::SetMaxThreads(0);
        });
    }

    pub fn solve_table(deal: &Deal) -> Result<TricksMatrix, Error> {
        let _guard = DDS_LOCK.lock().unwrap();
        let pbn_str = pbn::deal_to_dds_pbn(deal);

        if pbn_str.len() >= 80 {
            return Err(Error::DdsBufferTooLong {
                field: "ddTableDealPBN.cards",
                len: pbn_str.len(),
                max: 79,
            });
        }

        let mut deal_pbn: ffi::ddTableDealPBN = unsafe { std::mem::zeroed() };
        let pbn_bytes = pbn_str.as_bytes();
        for (i, &b) in pbn_bytes.iter().enumerate() {
            deal_pbn.cards[i] = b as std::os::raw::c_char;
        }

        let mut table: ffi::ddTableResults = unsafe { std::mem::zeroed() };
        let rc = unsafe { ffi::CalcDDtablePBN(deal_pbn, &mut table) };
        if rc != 1 {
            let msg = Self::error_message(rc);
            return Err(Error::Dds(msg));
        }

        Ok(TricksMatrix::from_dds(&table.resTable))
    }

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

    pub fn solve_position(
        position: &PlayPosition,
        trump: Strain,
    ) -> Result<Vec<CardResult>, Error> {
        let _guard = DDS_LOCK.lock().unwrap();
        let dds_deal = to_dds_deal(position, trump)?;
        let mut fut: ffi::futureTricks = unsafe { std::mem::zeroed() };
        let rc = unsafe { ffi::SolveBoardPBN(dds_deal, -1, 3, 1, &mut fut, 0) };
        if rc != 1 {
            let msg = Self::error_message(rc);
            return Err(Error::Dds(msg));
        }

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

            results.push(CardResult {
                card: Card::new(suit, rank),
                tricks_for_side_to_act: score,
                is_optimal: optimal,
            });

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

    pub fn solve_position_matrix(snapshot: &SnapshotPosition) -> Result<PositionMatrix, Error> {
        let mut data = [[0u8; 4]; 5];
        let hands_clone = snapshot.hands().clone();
        #[allow(clippy::needless_range_loop)]
        for next_idx in 0..4 {
            let next = Direction::from_dds_index(next_idx).unwrap();
            let ct = CurrentTrick::empty(next);
            let snap = SnapshotPosition::try_new(hands_clone.clone(), ct)?;
            let play = PlayPosition::try_from(snap)?;

            for strain in Strain::all() {
                let results = Self::solve_position(&play, strain)?;
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

    fn error_message(code: i32) -> String {
        let mut buf: [std::os::raw::c_char; 80] = [0; 80];
        unsafe {
            ffi::ErrorMessage(code, buf.as_mut_ptr());
        }
        unsafe { CStr::from_ptr(buf.as_ptr()).to_string_lossy().into_owned() }
    }
}

/// Convert a `PlayPosition` to the `DDS` `dealPBN` struct.
///
/// - `dealPBN.first = trick_leader` (the player who led this trick).
/// - `current_trick` cards are removed from `remaining_hands`.
/// - The `PBN` prefix is fixed to `N` (unrelated to the trick leader).
fn to_dds_deal(position: &PlayPosition, trump: Strain) -> Result<ffi::dealPBN, Error> {
    let leader = position.current_trick().leader();
    let remaining = position.remaining_hands();

    let remain = pbn::hands_to_dds_pbn(
        &[
            *remaining.get(Direction::North),
            *remaining.get(Direction::East),
            *remaining.get(Direction::South),
            *remaining.get(Direction::West),
        ],
        Direction::North,
    );
    if remain.len() >= 80 {
        return Err(Error::DdsBufferTooLong {
            field: "dealPBN.remainCards",
            len: remain.len(),
            max: 79,
        });
    }

    let mut dds_deal: ffi::dealPBN = unsafe { std::mem::zeroed() };
    dds_deal.trump = trump.dds_index() as i32;
    dds_deal.first = leader.dds_index() as i32;

    for (i, card) in position.current_trick().cards().iter().enumerate() {
        dds_deal.currentTrickSuit[i] = card.suit.dds_index() as i32;
        dds_deal.currentTrickRank[i] = card.rank.dds_rank();
    }

    let remain_bytes = remain.as_bytes();
    for (i, &b) in remain_bytes.iter().enumerate() {
        dds_deal.remainCards[i] = b as std::os::raw::c_char;
    }

    Ok(dds_deal)
}
