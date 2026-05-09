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
