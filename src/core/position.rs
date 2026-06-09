use super::deal::{Card, Direction, Hand, Strain, Suit};
use super::error::Error;

/// The game state at a given moment in the play.
///
/// `current_trick` cards are already removed from `hands`. When `current_trick`
/// is empty, `next_to_act` is also the trick leader.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Position {
    pub hands: [Hand; 4],
    pub next_to_act: Direction,
    pub current_trick: Vec<PlayedCard>,
}

/// A card played by a specific player.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayedCard {
    pub player: Direction,
    pub card: Card,
}

/// Whether a position is a manually entered snapshot or a runtime state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PositionKind {
    /// Clean trick boundary, all hands equal size.
    EntrySnapshot,
    /// May have uneven hands and a partial current trick.
    Runtime,
}

impl Position {
    /// Validate the position invariants.
    pub fn validate(&self, kind: PositionKind) -> Result<(), Error> {
        // current_trick <= 3
        if self.current_trick.len() > 3 {
            return Err(Error::InvalidPosition(format!(
                "current_trick has {} cards, max 3",
                self.current_trick.len()
            )));
        }

        // current_trick players in clockwise order
        let mut expected_player = if self.current_trick.is_empty() {
            self.next_to_act
        } else {
            self.current_trick[0].player
        };
        for played in &self.current_trick {
            if played.player != expected_player {
                return Err(Error::InvalidPosition(format!(
                    "current_trick players out of order: expected {:?}, got {:?}",
                    expected_player, played.player
                )));
            }
            expected_player = expected_player.next();
        }

        // next_to_act must be the player after the last current_trick card
        if !self.current_trick.is_empty() {
            let last_player = self.current_trick.last().unwrap().player;
            if self.next_to_act != last_player.next() {
                return Err(Error::InvalidPosition(format!(
                    "next_to_act {:?} does not follow last current_trick player {:?}",
                    self.next_to_act, last_player
                )));
            }
        }

        // EntrySnapshot: equal hand counts, empty current_trick
        if kind == PositionKind::EntrySnapshot {
            if !self.current_trick.is_empty() {
                return Err(Error::InvalidPosition(
                    "EntrySnapshot must have empty current_trick".into(),
                ));
            }
            let expected_count = self.hands[0].len();
            for (i, hand) in self.hands.iter().enumerate() {
                if hand.len() != expected_count {
                    return Err(Error::InvalidPosition(format!(
                        "EntrySnapshot: hand {} has {} cards, expected {}",
                        i,
                        hand.len(),
                        expected_count
                    )));
                }
            }
        }

        // Verify played cards are not still in hands (consistency check)
        for played in &self.current_trick {
            if self.hands[played.player.dds_index()].contains(played.card) {
                return Err(Error::InvalidPosition(format!(
                    "card {}{} is in current_trick but still in {:?}'s hand",
                    played.card.suit.as_char(),
                    played.card.rank.as_char(),
                    played.player,
                )));
            }
        }

        Ok(())
    }

    /// Verify no card appears in more than one hand.
    pub fn validate_no_duplicates(&self) -> Result<(), Error> {
        let mut seen: u64 = 0;
        for hand in &self.hands {
            for card in hand.cards() {
                let pos = card.suit.dds_index() * 13 + card.rank.bit_index();
                let mask = 1u64 << pos;
                if seen & mask != 0 {
                    return Err(Error::InvalidPosition(format!(
                        "duplicate card: {}{}",
                        card.suit.as_char(),
                        card.rank.as_char()
                    )));
                }
                seen |= mask;
            }
        }
        for played in &self.current_trick {
            let pos = played.card.suit.dds_index() * 13 + played.card.rank.bit_index();
            let mask = 1u64 << pos;
            if seen & mask != 0 {
                return Err(Error::InvalidPosition(format!(
                    "duplicate card in current_trick: {}{}",
                    played.card.suit.as_char(),
                    played.card.rank.as_char()
                )));
            }
            seen |= mask;
        }
        Ok(())
    }

    /// List legal cards for `next_to_act`. If `current_trick` is non-empty
    /// and the player holds the led suit, only cards of that suit are legal.
    pub fn legal_cards(&self) -> Vec<Card> {
        let hand = &self.hands[self.next_to_act.dds_index()];
        let all_cards: Vec<Card> = hand.cards().collect();

        if self.current_trick.is_empty() {
            return all_cards;
        }

        let led_suit = self.current_trick[0].card.suit;
        if hand.has_suit(led_suit) {
            all_cards
                .into_iter()
                .filter(|c| c.suit == led_suit)
                .collect()
        } else {
            all_cards
        }
    }

    /// Play a card from `next_to_act`'s hand. Returns the new position.
    ///
    /// The caller must ensure the card is legal (passes `legal_cards`).
    pub fn play_card(&self, card: Card, trump: Strain) -> Result<Position, Error> {
        let hand = &self.hands[self.next_to_act.dds_index()];
        if !hand.contains(card) {
            return Err(Error::InvalidPosition(format!(
                "{:?} does not hold {}{}",
                self.next_to_act,
                card.suit.as_char(),
                card.rank.as_char()
            )));
        }

        let mut new_hands = self.hands;
        new_hands[self.next_to_act.dds_index()] = hand.remove(card);

        let played = PlayedCard {
            player: self.next_to_act,
            card,
        };

        let mut new_trick = self.current_trick.clone();
        new_trick.push(played);

        if new_trick.len() == 4 {
            // Trick complete: determine winner, clear trick, winner leads next.
            let winner = trick_winner(&new_trick, trump);
            Ok(Position {
                hands: new_hands,
                next_to_act: winner,
                current_trick: vec![],
            })
        } else {
            // Trick in progress: next player clockwise.
            Ok(Position {
                hands: new_hands,
                next_to_act: self.next_to_act.next(),
                current_trick: new_trick,
            })
        }
    }
}

/// Determine the winner of a complete 4-card trick.
pub fn trick_winner(trick: &[PlayedCard], trump: Strain) -> Direction {
    assert_eq!(trick.len(), 4, "trick must have exactly 4 cards");

    let led_suit = trick[0].card.suit;

    // Find the highest trump played, and the highest card in the led suit.
    let mut winner = trick[0].player;
    let mut best_rank = trick[0].card.rank;
    let t_suit = trump_suit(trump);
    let mut best_is_trump = Some(trick[0].card.suit) == t_suit;

    for played in &trick[1..] {
        let is_trump = Some(played.card.suit) == t_suit;
        let beats = if is_trump && !best_is_trump {
            // Trump beats non-trump.
            true
        } else if is_trump && best_is_trump {
            // Both trump: higher rank wins.
            played.card.rank > best_rank
        } else if !is_trump && best_is_trump {
            // Our card is not trump but current best is: cannot beat.
            false
        } else {
            // Neither is trump: must follow suit to win, higher rank wins.
            played.card.suit == led_suit && played.card.rank > best_rank
        };

        if beats {
            winner = played.player;
            best_rank = played.card.rank;
            best_is_trump = is_trump;
        }
    }

    winner
}

/// The suit that is trumps, or None for NoTrump.
fn trump_suit(trump: Strain) -> Option<Suit> {
    match trump {
        Strain::NoTrump => None,
        Strain::Spades => Some(Suit::Spades),
        Strain::Hearts => Some(Suit::Hearts),
        Strain::Diamonds => Some(Suit::Diamonds),
        Strain::Clubs => Some(Suit::Clubs),
    }
}

#[cfg(test)]
mod tests {
    use super::super::deal::{Card, Direction, Hand, Rank, Strain, Suit};
    use super::*;

    fn make_hand(cards: &[(Suit, Rank)]) -> Hand {
        let cards: Vec<Card> = cards.iter().map(|(s, r)| Card::new(*s, *r)).collect();
        Hand::from_cards(&cards).unwrap()
    }

    #[test]
    fn test_position_validate_entry_snapshot() {
        let hands = [
            make_hand(&[(Suit::Spades, Rank::Ace)]),
            make_hand(&[(Suit::Hearts, Rank::King)]),
            make_hand(&[(Suit::Diamonds, Rank::Queen)]),
            make_hand(&[(Suit::Clubs, Rank::Two)]),
        ];
        let pos = Position {
            hands,
            next_to_act: Direction::North,
            current_trick: vec![],
        };
        assert!(pos.validate(PositionKind::EntrySnapshot).is_ok());
    }

    #[test]
    fn test_position_validate_unequal_hands_fails() {
        let hands = [
            make_hand(&[(Suit::Spades, Rank::Ace), (Suit::Spades, Rank::King)]),
            make_hand(&[(Suit::Hearts, Rank::King)]),
            make_hand(&[(Suit::Diamonds, Rank::Queen)]),
            make_hand(&[(Suit::Clubs, Rank::Two)]),
        ];
        let pos = Position {
            hands,
            next_to_act: Direction::North,
            current_trick: vec![],
        };
        assert!(pos.validate(PositionKind::EntrySnapshot).is_err());
    }

    #[test]
    fn test_position_validate_runtime_allows_uneven() {
        let hands = [
            make_hand(&[(Suit::Spades, Rank::Ace)]),
            make_hand(&[(Suit::Hearts, Rank::King)]),
            make_hand(&[(Suit::Diamonds, Rank::Queen)]),
            make_hand(&[(Suit::Clubs, Rank::Two)]),
        ];
        let pos = Position {
            hands,
            next_to_act: Direction::East,
            current_trick: vec![PlayedCard {
                player: Direction::North,
                card: Card::new(Suit::Spades, Rank::Three),
            }],
        };
        assert!(pos.validate(PositionKind::Runtime).is_ok());
    }

    #[test]
    fn test_legal_cards_enforces_follow_suit() {
        let hands = [
            make_hand(&[(Suit::Spades, Rank::King), (Suit::Hearts, Rank::Two)]),
            make_hand(&[(Suit::Hearts, Rank::Ace)]),
            make_hand(&[(Suit::Diamonds, Rank::Queen)]),
            make_hand(&[(Suit::Clubs, Rank::Two)]),
        ];
        let pos = Position {
            hands,
            next_to_act: Direction::North,
            current_trick: vec![PlayedCard {
                player: Direction::West,
                card: Card::new(Suit::Spades, Rank::Ace),
            }],
        };
        // N has a spade, must play spade.
        let cards = pos.legal_cards();
        assert_eq!(cards.len(), 1);
        assert_eq!(cards[0], Card::new(Suit::Spades, Rank::King));
    }

    #[test]
    fn test_legal_cards_no_follow_suit_when_void() {
        let hands = [
            make_hand(&[(Suit::Hearts, Rank::Two)]),
            make_hand(&[(Suit::Hearts, Rank::Ace)]),
            make_hand(&[(Suit::Diamonds, Rank::Queen)]),
            make_hand(&[(Suit::Clubs, Rank::Two)]),
        ];
        let pos = Position {
            hands,
            next_to_act: Direction::North,
            current_trick: vec![PlayedCard {
                player: Direction::West,
                card: Card::new(Suit::Spades, Rank::Ace),
            }],
        };
        // N has no spade, can play anything.
        let cards = pos.legal_cards();
        assert_eq!(cards.len(), 1);
        assert_eq!(cards[0].suit, Suit::Hearts);
    }

    #[test]
    fn test_play_card_advances_within_trick() {
        let hands = [
            make_hand(&[(Suit::Spades, Rank::Ace), (Suit::Spades, Rank::King)]),
            make_hand(&[(Suit::Hearts, Rank::Two)]),
            make_hand(&[(Suit::Diamonds, Rank::Three)]),
            make_hand(&[(Suit::Clubs, Rank::Four)]),
        ];
        let pos = Position {
            hands,
            next_to_act: Direction::North,
            current_trick: vec![],
        };
        let new_pos = pos
            .play_card(Card::new(Suit::Spades, Rank::Ace), Strain::NoTrump)
            .unwrap();
        assert_eq!(new_pos.current_trick.len(), 1);
        assert_eq!(new_pos.next_to_act, Direction::East);
        assert!(!new_pos.hands[0].contains(Card::new(Suit::Spades, Rank::Ace)));
    }

    #[test]
    fn test_play_card_completes_trick() {
        let hands = [
            make_hand(&[(Suit::Spades, Rank::Ace)]),
            make_hand(&[(Suit::Spades, Rank::King)]),
            make_hand(&[(Suit::Spades, Rank::Queen)]),
            make_hand(&[(Suit::Spades, Rank::Two)]),
        ];
        let pos = Position {
            hands,
            next_to_act: Direction::West,
            current_trick: vec![
                PlayedCard {
                    player: Direction::North,
                    card: Card::new(Suit::Spades, Rank::Ten),
                },
                PlayedCard {
                    player: Direction::East,
                    card: Card::new(Suit::Spades, Rank::Nine),
                },
                PlayedCard {
                    player: Direction::South,
                    card: Card::new(Suit::Spades, Rank::Eight),
                },
            ],
        };
        let new_pos = pos
            .play_card(Card::new(Suit::Spades, Rank::Two), Strain::NoTrump)
            .unwrap();
        // Trick complete: winner should be North (lead the Ten, which beats Two).
        assert!(new_pos.current_trick.is_empty());
        assert_eq!(new_pos.next_to_act, Direction::North);
    }

    #[test]
    fn test_trick_winner_trump_beats_non_trump() {
        let trick = vec![
            PlayedCard {
                player: Direction::North,
                card: Card::new(Suit::Spades, Rank::Two),
            },
            PlayedCard {
                player: Direction::East,
                card: Card::new(Suit::Spades, Rank::Ace),
            },
            PlayedCard {
                player: Direction::South,
                card: Card::new(Suit::Hearts, Rank::Two),
            },
            PlayedCard {
                player: Direction::West,
                card: Card::new(Suit::Spades, Rank::King),
            },
        ];
        let winner = trick_winner(&trick, Strain::Hearts);
        // S's H2 is trump, beats everyone.
        assert_eq!(winner, Direction::South);
    }

    #[test]
    fn test_trick_winner_no_trump_highest_led_suit() {
        let trick = vec![
            PlayedCard {
                player: Direction::North,
                card: Card::new(Suit::Diamonds, Rank::Three),
            },
            PlayedCard {
                player: Direction::East,
                card: Card::new(Suit::Diamonds, Rank::Ace),
            },
            PlayedCard {
                player: Direction::South,
                card: Card::new(Suit::Diamonds, Rank::King),
            },
            PlayedCard {
                player: Direction::West,
                card: Card::new(Suit::Spades, Rank::Ace),
            },
        ];
        // NoTrump: E's DA wins (highest diamond).
        let winner = trick_winner(&trick, Strain::NoTrump);
        assert_eq!(winner, Direction::East);
    }
}
