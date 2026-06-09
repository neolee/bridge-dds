use bridge_dds::core;
use bridge_dds::dds::DdsSolver;

/// Expected values from engine/dds/examples/hands.cpp.
const PBN: [&str; 3] = [
    "N:QJ6.K652.J85.T98 873.J97.AT764.Q4 K5.T83.KQ9.A7652 AT942.AQ4.32.KJ3",
    "E:QJT5432.T.6.QJ82 .J97543.K7532.94 87.A62.QJT4.AT75 AK96.KQ8.A98.K63",
    "N:73.QJT.AQ54.T752 QT6.876.KJ9.AQ84 5.A95432.7632.K6 AKJ9842.K.T8.J93",
];

/// DDtable in [strain][declarer] order: S(N,E,S,W), H(N,E,S,W), ..., NT(N,E,S,W).
const DD_TABLE: [[[u8; 4]; 5]; 3] = [
    // Deal 0
    [
        [5, 8, 5, 8], // S: N=5, E=8, S=5, W=8
        [6, 6, 6, 6], // H
        [5, 7, 5, 7], // D
        [7, 5, 7, 5], // C
        [6, 6, 6, 6], // NT
    ],
    // Deal 1
    [
        [4, 9, 4, 9],
        [10, 2, 10, 2],
        [8, 3, 8, 3],
        [6, 7, 6, 7],
        [9, 3, 9, 3],
    ],
    // Deal 2
    [
        [3, 10, 3, 10],
        [9, 4, 9, 4],
        [8, 4, 8, 4],
        [3, 9, 3, 9],
        [4, 8, 4, 8],
    ],
];

const DEALER: [u8; 3] = [0, 1, 0]; // N=0, E=1, N=0
const VUL: [u8; 3] = [0, 2, 0]; // None=0, NS=2, None=0
const DEALER_SCORE: [i32; 3] = [-110, 100, -300];
const DEALER_CONTRACT: [[&str; 1]; 3] = [["2S-EW"], ["4S*-EW-1"], ["5H*-NS-2"]];

#[test]
fn test_dds_table_deal_0() {
    test_deal(0);
}

#[test]
fn test_dds_table_deal_1() {
    test_deal(1);
}

#[test]
fn test_dds_table_deal_2() {
    test_deal(2);
}

fn test_deal(idx: usize) {
    DdsSolver::init();

    let deal = core::pbn::parse_deal_tag(PBN[idx]).unwrap();
    let table = DdsSolver::solve_table(&deal).unwrap();

    // Verify all 20 trick values.
    for (strain, dd_row) in DD_TABLE[idx].iter().enumerate() {
        for (decl, &expected) in dd_row.iter().enumerate() {
            let got = table.data()[strain][decl];
            assert_eq!(
                got, expected,
                "deal {} strain {} decl {}: expected {}, got {}",
                idx, strain, decl, expected, got
            );
        }
    }

    // Verify DealerPar results.
    let dealer = core::deal::Direction::from_dds_index(DEALER[idx] as usize).unwrap();
    let vul = match VUL[idx] {
        0 => core::deal::Vulnerability::None,
        1 => core::deal::Vulnerability::Both,
        2 => core::deal::Vulnerability::NS,
        3 => core::deal::Vulnerability::EW,
        _ => unreachable!(),
    };
    let par = DdsSolver::compute_par(&table, dealer, vul).unwrap();

    assert_eq!(par.score, DEALER_SCORE[idx], "deal {} par score", idx);
    assert_eq!(
        par.contracts.len(),
        DEALER_CONTRACT[idx].len(),
        "deal {} par contract count",
        idx
    );
    for (i, expected) in DEALER_CONTRACT[idx].iter().enumerate() {
        assert_eq!(
            par.contracts[i], *expected,
            "deal {} par contract {}",
            idx, i
        );
    }
}

// --- Phase 1b: position analysis ---

use bridge_dds::core::deal::{Card, Direction, Hand, Rank, Strain, Suit};
use bridge_dds::core::pbn;
use bridge_dds::core::position::{PlayedCard, Position};

fn one_suit_hand(suit: Suit) -> Hand {
    Hand::from_cards(&[
        Card::new(suit, Rank::Ace),
        Card::new(suit, Rank::King),
        Card::new(suit, Rank::Queen),
        Card::new(suit, Rank::Jack),
    ])
    .unwrap()
}

fn four_suit_position() -> Position {
    Position {
        hands: [
            one_suit_hand(Suit::Spades),
            one_suit_hand(Suit::Hearts),
            one_suit_hand(Suit::Diamonds),
            one_suit_hand(Suit::Clubs),
        ],
        next_to_act: Direction::North,
        current_trick: vec![],
    }
}

#[test]
fn test_position_clean_trick() {
    DdsSolver::init();
    let pos = four_suit_position();
    let results = DdsSolver::solve_position(&pos, Strain::NoTrump).unwrap();
    // N has only spades, leads with NT. All 4 spade cards returned.
    assert_eq!(results.len(), 4);
    for r in &results {
        assert_eq!(r.card.suit, Suit::Spades);
        assert_eq!(r.tricks_for_side_to_act, 4);
    }
}

#[test]
fn test_position_matrix() {
    DdsSolver::init();
    let pos = four_suit_position();
    let pm = DdsSolver::solve_position_matrix(&pos).unwrap();
    // Each cell should be a valid trick count (0..=4 for 4 cards each).
    for strain in Strain::all() {
        for next_idx in 0..4 {
            let v = pm.data[strain.dds_index()][next_idx];
            assert!(v <= 4, "strain {:?} next {}: got {}", strain, next_idx, v);
        }
    }
    // When a player leads their own suit, they should get at least some tricks.
    for i in 0..4 {
        let strain = Strain::from_dds_index(i).unwrap();
        assert!(pm.data[i][i] > 0, "own suit {:?}: got 0", strain);
    }
}

#[test]
fn test_position_mid_trick_1_card() {
    DdsSolver::init();
    let pos = Position {
        hands: [
            one_suit_hand(Suit::Spades),
            one_suit_hand(Suit::Hearts),
            one_suit_hand(Suit::Diamonds),
            one_suit_hand(Suit::Clubs),
        ],
        next_to_act: Direction::East,
        current_trick: vec![PlayedCard {
            player: Direction::North,
            card: Card::new(Suit::Spades, Rank::Ace),
        }],
    };
    let results = DdsSolver::solve_position(&pos, Strain::NoTrump).unwrap();
    // E is next, holds hearts only. N's SA wins this trick.
    for r in &results {
        assert_eq!(r.card.suit, Suit::Hearts);
    }
}

#[test]
fn test_position_mid_trick_3_cards() {
    DdsSolver::init();
    let pos = Position {
        hands: [
            one_suit_hand(Suit::Spades),
            one_suit_hand(Suit::Hearts),
            one_suit_hand(Suit::Diamonds),
            one_suit_hand(Suit::Clubs),
        ],
        next_to_act: Direction::West,
        current_trick: vec![
            PlayedCard {
                player: Direction::North,
                card: Card::new(Suit::Spades, Rank::Ace),
            },
            PlayedCard {
                player: Direction::East,
                card: Card::new(Suit::Hearts, Rank::Ace),
            },
            PlayedCard {
                player: Direction::South,
                card: Card::new(Suit::Diamonds, Rank::Ace),
            },
        ],
    };
    let results = DdsSolver::solve_position(&pos, Strain::NoTrump).unwrap();
    // W is last, holds clubs only. N's SA wins.
    for r in &results {
        assert_eq!(r.card.suit, Suit::Clubs);
    }
}

#[test]
fn test_residual_pbn_parsing() {
    let input = "\
[Position \"N:AKQJ... .AKQJ.. ..AKQJ. ...AKQJ\"]\n[First \"N\"]\n[Trump \"NT\"]\n";
    let res = pbn::parse_residual_record(input).unwrap();
    assert_eq!(res.first.unwrap(), Direction::North);
    assert_eq!(res.trump.unwrap(), Strain::NoTrump);
    assert_eq!(res.hands[0].len(), 4); // N has 4 spades
    assert_eq!(res.hands[1].len(), 4); // E has 4 hearts
}

#[test]
fn test_residual_pbn_with_current_trick() {
    let input = "\
[Position \"N:AKQJ... .AKQJ.. ..AKQJ. ...AKQJ\"]\n[First \"E\"]\n[Trump \"NT\"]\n[CurrentTrick \"N:SA\"]\n";
    let res = pbn::parse_residual_record(input).unwrap();
    assert_eq!(res.first.unwrap(), Direction::East);
    assert_eq!(res.current_trick.len(), 1);
    assert_eq!(res.current_trick[0].0, Direction::North);
    assert_eq!(res.current_trick[0].1, Card::new(Suit::Spades, Rank::Ace));
}

#[test]
fn test_residual_pbn_current_trick_invalid_card() {
    let input = "\
[Position \"N:AKQJ... .AKQJ.. ..AKQJ. ...AKQJ\"]\n[First \"W\"]\n[Trump \"NT\"]\n[CurrentTrick \"N:SA E:S2\"]\n";
    let err = pbn::parse_residual_record(input).unwrap_err();
    assert!(err.to_string().contains("East does not hold S2"));
}
