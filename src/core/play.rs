use super::deal::{Deal, Direction, Strain};
use super::error::Error;
use super::pbn::{ParsedPlay, PlayRow};
use super::position::{CurrentTrick, PlayPosition, PlayedCard, SnapshotPosition};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormalizedPlay {
    opening_leader: Direction,
    played_cards: Vec<PlayedCard>,
    final_position: PlayPosition,
}

impl NormalizedPlay {
    pub fn opening_leader(&self) -> Direction {
        self.opening_leader
    }

    pub fn played_cards(&self) -> &[PlayedCard] {
        &self.played_cards
    }

    pub fn final_position(&self) -> &PlayPosition {
        &self.final_position
    }
}

/// Normalize either supported `Play` representation and advance its deal once.
pub fn normalize_play(
    play: &ParsedPlay,
    deal: &Deal,
    trump: Strain,
    opening_leader: Direction,
) -> Result<NormalizedPlay, Error> {
    let snapshot =
        SnapshotPosition::try_new(deal.hands().clone(), CurrentTrick::empty(opening_leader))?;
    let mut position = PlayPosition::try_from(snapshot)?;
    let mut played_cards = Vec::new();

    match play {
        ParsedPlay::Legacy { cards, .. } => {
            for card in cards {
                advance(&mut position, &mut played_cards, *card, trump)?;
            }
        }
        ParsedPlay::Standard { first_column, rows } => {
            for (row_index, row) in rows.iter().enumerate() {
                normalize_standard_row(
                    row,
                    row_index,
                    row_index + 1 == rows.len(),
                    *first_column,
                    &mut position,
                    &mut played_cards,
                    trump,
                )?;
            }
        }
    }

    Ok(NormalizedPlay {
        opening_leader,
        played_cards,
        final_position: position,
    })
}

fn normalize_standard_row(
    row: &PlayRow,
    row_index: usize,
    is_final_row: bool,
    first_column: Direction,
    position: &mut PlayPosition,
    played_cards: &mut Vec<PlayedCard>,
    trump: Strain,
) -> Result<(), Error> {
    let card_count = row.cards.iter().flatten().count();
    if card_count == 0 {
        return Err(Error::InvalidPlayTrace(format!(
            "standard Play row {} contains no cards",
            row_index + 1
        )));
    }
    if card_count < 4 && !is_final_row {
        return Err(Error::InvalidPlayTrace(format!(
            "incomplete standard Play row {} is not final",
            row_index + 1
        )));
    }
    if !position.current_trick().is_empty() {
        return Err(Error::InvalidPlayTrace(format!(
            "standard Play row {} begins before the previous trick is complete",
            row_index + 1
        )));
    }

    let leader = position.current_trick().leader();
    let mut missing_turn = false;
    for offset in 0..4 {
        let player = leader.advance(offset);
        let column = (player.dds_index() + 4 - first_column.dds_index()) % 4;
        match row.cards[column] {
            Some(card) if missing_turn => {
                return Err(Error::InvalidPlayTrace(format!(
                    "standard Play row {} has {} after a missing turn for {}",
                    row_index + 1,
                    card.to_pbn(),
                    player.as_char()
                )));
            }
            Some(card) => advance(position, played_cards, card, trump)?,
            None => missing_turn = true,
        }
    }
    Ok(())
}

fn advance(
    position: &mut PlayPosition,
    played_cards: &mut Vec<PlayedCard>,
    card: super::deal::Card,
    trump: Strain,
) -> Result<(), Error> {
    let player = position.current_trick().next_to_act();
    position.play_card(card, trump).map_err(|error| {
        Error::InvalidPlayTrace(format!(
            "card {} ({} by {}): {}",
            played_cards.len() + 1,
            card.to_pbn(),
            player.as_char(),
            error
        ))
    })?;
    played_cards.push(PlayedCard::new(player, card));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::super::deal::{Card, Rank, Suit};
    use super::super::pbn::parse_record;
    use super::*;

    const DEAL: &str = "N:QJ6.K652.J85.T98 873.J97.AT764.Q4 K5.T83.KQ9.A7652 AT942.AQ4.32.KJ3";

    fn deal() -> Deal {
        parse_record(&format!("[Deal \"{}\"]", DEAL))
            .unwrap()
            .deal
            .unwrap()
    }

    fn parse_play(input: &str) -> ParsedPlay {
        parse_record(input).unwrap().play.unwrap()
    }

    #[test]
    fn normalizes_legacy_cards_chronologically_once() {
        let play = parse_play("[Play \"E:S3=S5=S2=SQ\"]");
        let normalized = normalize_play(&play, &deal(), Strain::Spades, Direction::East).unwrap();
        let players: Vec<_> = normalized
            .played_cards()
            .iter()
            .map(PlayedCard::player)
            .collect();
        assert_eq!(
            players,
            [
                Direction::East,
                Direction::South,
                Direction::West,
                Direction::North
            ]
        );
        assert_eq!(normalized.opening_leader(), Direction::East);
        assert_eq!(normalized.played_cards()[0].card().to_pbn(), "S3");
        assert_eq!(
            normalized.final_position().current_trick().leader(),
            Direction::North
        );
        assert!(normalized.final_position().current_trick().is_empty());
    }

    #[test]
    fn normalizes_empty_and_long_equals_legacy_sequences() {
        let empty = parse_play("[Play \"\"]");
        let normalized = normalize_play(&empty, &deal(), Strain::Spades, Direction::East).unwrap();
        assert!(normalized.played_cards().is_empty());
        assert_eq!(
            normalized.final_position().current_trick().leader(),
            Direction::East
        );

        let long = parse_play("[Play \"E:S3=S5=S2=SQ=H2\"]");
        let normalized = normalize_play(&long, &deal(), Strain::Spades, Direction::East).unwrap();
        assert_eq!(normalized.played_cards().len(), 5);
        assert_eq!(
            normalized.final_position().current_trick().leader(),
            Direction::North
        );
        assert_eq!(normalized.final_position().current_trick().len(), 1);
    }

    #[test]
    fn standard_rows_follow_changing_leaders_and_fixed_columns() {
        let play = parse_play("[Play \"E\"]\nS3 S5 S2 SQ\nH7 H3 HA H2\n- - C3 C8");
        let normalized = normalize_play(&play, &deal(), Strain::Spades, Direction::East).unwrap();
        assert_eq!(normalized.played_cards().len(), 10);
        assert_eq!(normalized.played_cards()[4].player(), Direction::North);
        assert_eq!(normalized.played_cards()[8].player(), Direction::West);
        assert_eq!(
            normalized.final_position().current_trick().leader(),
            Direction::West
        );
        assert_eq!(normalized.final_position().current_trick().len(), 2);
        assert_eq!(
            normalized.final_position().current_trick().next_to_act(),
            Direction::East
        );
    }

    #[test]
    fn accepts_incomplete_prefix_for_every_leader_and_length() {
        let deal = deal();
        let first_column = Direction::West;
        for leader in Direction::all() {
            for length in 1..=3 {
                let mut row = PlayRow { cards: [None; 4] };
                for offset in 0..length {
                    let player = leader.advance(offset);
                    let card = deal
                        .hands()
                        .get(player)
                        .cards()
                        .find(|card| card.suit == Suit::Spades)
                        .unwrap();
                    let column = (player.dds_index() + 4 - first_column.dds_index()) % 4;
                    row.cards[column] = Some(card);
                }
                let play = ParsedPlay::Standard {
                    first_column,
                    rows: vec![row],
                };
                let normalized = normalize_play(&play, &deal, Strain::NoTrump, leader).unwrap();
                assert_eq!(normalized.played_cards().len(), length);
                assert_eq!(normalized.final_position().current_trick().leader(), leader);
            }
        }
    }

    #[test]
    fn rejects_chronological_gap_in_standard_final_row() {
        let play = ParsedPlay::Standard {
            first_column: Direction::North,
            rows: vec![PlayRow {
                cards: [
                    Some(Card::new(Suit::Spades, Rank::Queen)),
                    None,
                    Some(Card::new(Suit::Spades, Rank::King)),
                    None,
                ],
            }],
        };
        assert!(matches!(
            normalize_play(&play, &deal(), Strain::NoTrump, Direction::North),
            Err(Error::InvalidPlayTrace(_))
        ));
    }

    #[test]
    fn rejects_ownership_and_follow_suit_violations() {
        let ownership = parse_play("[Play \"E:S5\"]");
        assert!(matches!(
            normalize_play(&ownership, &deal(), Strain::Spades, Direction::East),
            Err(Error::InvalidPlayTrace(_))
        ));

        let revoke = parse_play("[Play \"E:S3=H3\"]");
        assert!(matches!(
            normalize_play(&revoke, &deal(), Strain::Spades, Direction::East),
            Err(Error::InvalidPlayTrace(_))
        ));

        let wrong_column = parse_play("[Play \"E\"]\nS5 S3 S2 SQ");
        assert!(matches!(
            normalize_play(&wrong_column, &deal(), Strain::Spades, Direction::East),
            Err(Error::InvalidPlayTrace(_))
        ));
    }
}
