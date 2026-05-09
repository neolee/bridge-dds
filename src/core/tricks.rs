use serde::Serialize;

use super::deal::{Direction, Side, Strain};

/// Double-dummy tricks matrix. Row = strain (S,H,D,C,NT), col = declarer (N,E,S,W).
#[derive(Debug, Clone)]
pub struct TricksMatrix {
    pub(crate) data: [[u8; 4]; 5],
}

impl TricksMatrix {
    /// Create from the DDS `ddTableResults` raw array.
    /// DDS layout: resTable[strain 0=S..4=NT][declarer 0=N..3=W].
    pub fn from_dds(raw: &[[i32; 4]; 5]) -> Self {
        let mut data = [[0u8; 4]; 5];
        for strain in 0..5 {
            for decl in 0..4 {
                data[strain][decl] = raw[strain][decl] as u8;
            }
        }
        TricksMatrix { data }
    }

    pub fn get(&self, strain: Strain, declarer: Direction) -> u8 {
        self.data[strain.dds_index()][declarer.dds_index()]
    }

    pub fn best_for_side(&self, side: Side, strain: Strain) -> u8 {
        let si = strain.dds_index();
        match side {
            Side::NS => self.data[si][0].max(self.data[si][2]), // N(0) or S(2)
            Side::EW => self.data[si][1].max(self.data[si][3]), // E(1) or W(3)
        }
    }

    /// Convert back to DDS `ddTableResults` layout for `DealerPar()`.
    pub(crate) fn to_dds(&self) -> [[i32; 4]; 5] {
        let mut raw = [[0i32; 4]; 5];
        for strain in 0..5 {
            for decl in 0..4 {
                raw[strain][decl] = self.data[strain][decl] as i32;
            }
        }
        raw
    }
}

/// Serializable shape for the JSON output.
#[derive(Debug, Clone, Serialize)]
pub struct TricksJson {
    #[serde(rename = "N")]
    pub north: StrainTricks,
    #[serde(rename = "E")]
    pub east: StrainTricks,
    #[serde(rename = "S")]
    pub south: StrainTricks,
    #[serde(rename = "W")]
    pub west: StrainTricks,
}

#[derive(Debug, Clone, Serialize)]
pub struct StrainTricks {
    #[serde(rename = "S")]
    pub spades: u8,
    #[serde(rename = "H")]
    pub hearts: u8,
    #[serde(rename = "D")]
    pub diamonds: u8,
    #[serde(rename = "C")]
    pub clubs: u8,
    #[serde(rename = "NT")]
    pub notrump: u8,
}

impl TricksMatrix {
    pub fn to_json(&self) -> TricksJson {
        TricksJson {
            north: StrainTricks {
                spades: self.data[0][0],
                hearts: self.data[1][0],
                diamonds: self.data[2][0],
                clubs: self.data[3][0],
                notrump: self.data[4][0],
            },
            east: StrainTricks {
                spades: self.data[0][1],
                hearts: self.data[1][1],
                diamonds: self.data[2][1],
                clubs: self.data[3][1],
                notrump: self.data[4][1],
            },
            south: StrainTricks {
                spades: self.data[0][2],
                hearts: self.data[1][2],
                diamonds: self.data[2][2],
                clubs: self.data[3][2],
                notrump: self.data[4][2],
            },
            west: StrainTricks {
                spades: self.data[0][3],
                hearts: self.data[1][3],
                diamonds: self.data[2][3],
                clubs: self.data[3][3],
                notrump: self.data[4][3],
            },
        }
    }
}
