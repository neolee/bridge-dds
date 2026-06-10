use super::deal::{Card, Direction, Hands, Strain, Suit};
use super::error::Error;

/// The incomplete current trick: `0..=3` cards in play order alongside
/// the player who led this trick. Player identity for each card is derived
/// clockwise from `leader` rather than stored per card.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CurrentTrick {
    leader: Direction,
    cards: Vec<Card>,
}

impl CurrentTrick {
    pub fn try_new(leader: Direction, cards: Vec<Card>) -> Result<Self, Error> {
        if cards.len() > 3 {
            return Err(Error::InvalidPosition(format!(
                "current_trick has {} cards, max 3",
                cards.len()
            )));
        }
        Ok(CurrentTrick { leader, cards })
    }

    pub fn empty(leader: Direction) -> Self {
        CurrentTrick {
            leader,
            cards: vec![],
        }
    }

    pub fn leader(&self) -> Direction {
        self.leader
    }

    pub fn cards(&self) -> &[Card] {
        &self.cards
    }

    pub fn len(&self) -> usize {
        self.cards.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cards.is_empty()
    }

    pub fn player_at(&self, index: usize) -> Option<Direction> {
        if index < self.cards.len() {
            Some(self.leader.advance(index))
        } else {
            None
        }
    }

    pub fn next_to_act(&self) -> Direction {
        self.leader.advance(self.cards.len())
    }

    pub fn led_suit(&self) -> Option<Suit> {
        self.cards.first().map(|c| c.suit)
    }
}

/// Public input/output model. `hands` include the incomplete current trick's
/// cards and have equal counts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnapshotPosition {
    hands: Hands,
    current_trick: CurrentTrick,
}

impl SnapshotPosition {
    pub fn try_new(hands: Hands, current_trick: CurrentTrick) -> Result<Self, Error> {
        let counts = hands.counts();
        let expected = counts[0];
        for (i, &c) in counts.iter().enumerate() {
            if c != expected {
                return Err(Error::InvalidPosition(format!(
                    "SnapshotPosition: hand {} has {} cards, expected {}",
                    i, c, expected
                )));
            }
        }
        for i in 0..current_trick.len() {
            let card = current_trick.cards()[i];
            let player = current_trick.player_at(i).unwrap();
            if !hands.get(player).contains(card) {
                return Err(Error::InvalidPosition(format!(
                    "SnapshotPosition: {:?} does not hold {}{} (current trick card {})",
                    player,
                    card.suit.as_char(),
                    card.rank.as_char(),
                    i + 1
                )));
            }
        }
        if let Some(led) = current_trick.led_suit() {
            for i in 1..current_trick.len() {
                let card = current_trick.cards()[i];
                let player = current_trick.player_at(i).unwrap();
                if hands.get(player).has_suit(led) && card.suit != led {
                    return Err(Error::InvalidPosition(format!(
                        "SnapshotPosition: {:?} must follow suit {} but played {}{}",
                        player,
                        led.as_char(),
                        card.suit.as_char(),
                        card.rank.as_char()
                    )));
                }
            }
        }
        Ok(SnapshotPosition {
            hands,
            current_trick,
        })
    }

    pub fn hands(&self) -> &Hands {
        &self.hands
    }

    pub fn current_trick(&self) -> &CurrentTrick {
        &self.current_trick
    }
}

/// Internal advancement model. Current-trick cards have been removed from
/// `remaining_hands`. Accepted by the `DDS` wrapper.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayPosition {
    remaining_hands: Hands,
    current_trick: CurrentTrick,
}

impl PlayPosition {
    pub fn try_new(remaining_hands: Hands, current_trick: CurrentTrick) -> Result<Self, Error> {
        for i in 0..current_trick.len() {
            let card = current_trick.cards()[i];
            let player = current_trick.player_at(i).unwrap();
            if remaining_hands.get(player).contains(card) {
                return Err(Error::InvalidPosition(format!(
                    "PlayPosition: {:?} still holds {}{} (should be removed)",
                    player,
                    card.suit.as_char(),
                    card.rank.as_char()
                )));
            }
        }
        let mut reconstructed = remaining_hands.clone();
        for i in 0..current_trick.len() {
            let card = current_trick.cards()[i];
            let player = current_trick.player_at(i).unwrap();
            reconstructed = reconstructed
                .add(player, card)
                .map_err(|e| Error::InvalidPosition(format!("PlayPosition: {}", e)))?;
        }
        let counts = reconstructed.counts();
        let expected = counts[0];
        for (i, &c) in counts.iter().enumerate() {
            if c != expected {
                return Err(Error::InvalidPosition(format!(
                    "PlayPosition: after adding back current trick, hand {} has {} cards, expected {}",
                    i, c, expected
                )));
            }
        }
        Ok(PlayPosition {
            remaining_hands,
            current_trick,
        })
    }

    pub fn remaining_hands(&self) -> &Hands {
        &self.remaining_hands
    }

    pub fn current_trick(&self) -> &CurrentTrick {
        &self.current_trick
    }

    pub fn legal_cards(&self) -> Vec<Card> {
        let hand = self.remaining_hands.get(self.current_trick.next_to_act());
        let all_cards: Vec<Card> = hand.cards().collect();
        if self.current_trick.is_empty() {
            return all_cards;
        }
        let led_suit = self.current_trick.led_suit().unwrap();
        if hand.has_suit(led_suit) {
            all_cards
                .into_iter()
                .filter(|c| c.suit == led_suit)
                .collect()
        } else {
            all_cards
        }
    }

    pub fn play_card(&mut self, card: Card, trump: Strain) -> Result<(), Error> {
        let player = self.current_trick.next_to_act();
        if !self.remaining_hands.get(player).contains(card) {
            return Err(Error::InvalidPosition(format!(
                "{:?} does not hold {}{}",
                player,
                card.suit.as_char(),
                card.rank.as_char()
            )));
        }
        if !self.current_trick.is_empty() {
            let led = self.current_trick.led_suit().unwrap();
            if self.remaining_hands.get(player).has_suit(led) && card.suit != led {
                return Err(Error::InvalidPosition(format!(
                    "{:?} must follow suit {}",
                    player,
                    led.as_char()
                )));
            }
        }

        self.remaining_hands = self
            .remaining_hands
            .remove(player, card)
            .map_err(|e| Error::InvalidPosition(format!("PlayPosition::play_card: {}", e)))?;

        let mut new_cards = self.current_trick.cards().to_vec();
        new_cards.push(card);

        if new_cards.len() == 4 {
            let trick: Vec<PlayedCard> = (0..4)
                .map(|i| PlayedCard {
                    player: self.current_trick.leader().advance(i),
                    card: new_cards[i],
                })
                .collect();
            let winner = trick_winner(&trick, trump);
            self.current_trick = CurrentTrick::empty(winner);
        } else {
            self.current_trick = CurrentTrick::try_new(self.current_trick.leader(), new_cards)?;
        }
        Ok(())
    }
}

// --- Conversions ---

impl TryFrom<SnapshotPosition> for PlayPosition {
    type Error = Error;

    fn try_from(snap: SnapshotPosition) -> Result<Self, Self::Error> {
        let mut remaining = snap.hands.clone();
        for i in 0..snap.current_trick.len() {
            let card = snap.current_trick.cards()[i];
            let player = snap.current_trick.player_at(i).unwrap();
            remaining = remaining.remove(player, card).map_err(|e| {
                Error::InvalidPosition(format!(
                    "SnapshotPosition -> PlayPosition: cannot remove {}{}: {}",
                    card.suit.as_char(),
                    card.rank.as_char(),
                    e
                ))
            })?;
        }
        PlayPosition::try_new(remaining, snap.current_trick)
    }
}

impl TryFrom<&PlayPosition> for SnapshotPosition {
    type Error = Error;

    fn try_from(play: &PlayPosition) -> Result<Self, Self::Error> {
        let mut hands = play.remaining_hands.clone();
        for i in 0..play.current_trick.len() {
            let card = play.current_trick.cards()[i];
            let player = play.current_trick.player_at(i).unwrap();
            hands = hands.add(player, card).map_err(|e| {
                Error::InvalidPosition(format!(
                    "PlayPosition -> SnapshotPosition: cannot add back {}{}: {}",
                    card.suit.as_char(),
                    card.rank.as_char(),
                    e
                ))
            })?;
        }
        SnapshotPosition::try_new(hands, play.current_trick.clone())
    }
}

// --- Internal helper types ---

/// A card played by a specific player. Used only by `trick_winner`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayedCard {
    player: Direction,
    card: Card,
}

/// Determine the winner of a complete 4-card trick.
pub fn trick_winner(trick: &[PlayedCard], trump: Strain) -> Direction {
    assert_eq!(trick.len(), 4, "trick must have exactly 4 cards");
    let led_suit = trick[0].card.suit;
    let t_suit = trump_suit(trump);
    let mut winner = trick[0].player;
    let mut best_rank = trick[0].card.rank;
    let mut best_is_trump = Some(trick[0].card.suit) == t_suit;

    for played in &trick[1..] {
        let is_trump = Some(played.card.suit) == t_suit;
        let beats = if is_trump && !best_is_trump {
            true
        } else if is_trump && best_is_trump {
            played.card.rank > best_rank
        } else if !is_trump && best_is_trump {
            false
        } else {
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
mod new_tests {
    use super::super::deal::{Hand, Hands, Rank};
    use super::*;

    fn one_card(suit: Suit, rank: Rank) -> Hand {
        Hand::from_cards(&[Card::new(suit, rank)]).unwrap()
    }

    fn two_cards(suit: Suit, r1: Rank, r2: Rank) -> Hand {
        Hand::from_cards(&[Card::new(suit, r1), Card::new(suit, r2)]).unwrap()
    }

    #[test]
    fn test_current_trick_basics() {
        let ct = CurrentTrick::try_new(Direction::North, vec![]).unwrap();
        assert!(ct.is_empty());
        assert_eq!(ct.next_to_act(), Direction::North);

        let ct = CurrentTrick::try_new(Direction::East, vec![Card::new(Suit::Spades, Rank::Ace)])
            .unwrap();
        assert_eq!(ct.len(), 1);
        assert_eq!(ct.player_at(0), Some(Direction::East));
        assert_eq!(ct.next_to_act(), Direction::South);
        assert_eq!(ct.led_suit(), Some(Suit::Spades));
    }

    #[test]
    fn test_current_trick_rejects_4_cards() {
        let cards = vec![
            Card::new(Suit::Spades, Rank::Ace),
            Card::new(Suit::Hearts, Rank::Ace),
            Card::new(Suit::Diamonds, Rank::Ace),
            Card::new(Suit::Clubs, Rank::Ace),
        ];
        assert!(CurrentTrick::try_new(Direction::North, cards).is_err());
    }

    #[test]
    fn test_snapshot_position_valid() {
        let hands = Hands::try_new([
            one_card(Suit::Spades, Rank::Ace),
            one_card(Suit::Hearts, Rank::Ace),
            one_card(Suit::Diamonds, Rank::Ace),
            one_card(Suit::Clubs, Rank::Ace),
        ])
        .unwrap();
        let ct = CurrentTrick::try_new(Direction::North, vec![]).unwrap();
        assert!(SnapshotPosition::try_new(hands, ct).is_ok());
    }

    #[test]
    fn test_snapshot_rejects_unequal_hands() {
        let hands = Hands::try_new([
            two_cards(Suit::Spades, Rank::Ace, Rank::King),
            one_card(Suit::Hearts, Rank::Ace),
            one_card(Suit::Diamonds, Rank::Ace),
            one_card(Suit::Clubs, Rank::Ace),
        ])
        .unwrap();
        let ct = CurrentTrick::try_new(Direction::North, vec![]).unwrap();
        assert!(SnapshotPosition::try_new(hands, ct).is_err());
    }

    #[test]
    fn test_snapshot_to_play_roundtrip() {
        let hands = Hands::try_new([
            two_cards(Suit::Spades, Rank::Ace, Rank::King),
            two_cards(Suit::Hearts, Rank::Ace, Rank::King),
            two_cards(Suit::Diamonds, Rank::Ace, Rank::King),
            two_cards(Suit::Clubs, Rank::Ace, Rank::King),
        ])
        .unwrap();
        let ct = CurrentTrick::try_new(Direction::North, vec![Card::new(Suit::Spades, Rank::Ace)])
            .unwrap();
        let snap = SnapshotPosition::try_new(hands, ct).unwrap();
        let play = PlayPosition::try_from(snap.clone()).unwrap();
        assert_eq!(play.remaining_hands().get(Direction::North).len(), 1);
        assert_eq!(play.remaining_hands().get(Direction::East).len(), 2);
        let snap2 = SnapshotPosition::try_from(&play).unwrap();
        assert_eq!(snap, snap2);
    }

    #[test]
    fn test_play_position_play_card() {
        let hands = Hands::try_new([
            two_cards(Suit::Spades, Rank::Ace, Rank::King),
            two_cards(Suit::Hearts, Rank::Ace, Rank::King),
            two_cards(Suit::Diamonds, Rank::Ace, Rank::King),
            two_cards(Suit::Clubs, Rank::Ace, Rank::King),
        ])
        .unwrap();
        let ct = CurrentTrick::try_new(Direction::North, vec![]).unwrap();
        let snap = SnapshotPosition::try_new(hands, ct).unwrap();
        let mut play = PlayPosition::try_from(snap).unwrap();
        play.play_card(Card::new(Suit::Spades, Rank::Ace), Strain::NoTrump)
            .unwrap();
        assert_eq!(play.current_trick().len(), 1);
        assert_eq!(play.current_trick().next_to_act(), Direction::East);
        play.play_card(Card::new(Suit::Hearts, Rank::Ace), Strain::NoTrump)
            .unwrap();
        assert_eq!(play.current_trick().len(), 2);
    }

    #[test]
    fn test_play_position_follow_suit_enforced() {
        let hands = Hands::try_new([
            two_cards(Suit::Spades, Rank::Ace, Rank::King),
            two_cards(Suit::Spades, Rank::Queen, Rank::Jack),
            two_cards(Suit::Diamonds, Rank::Ace, Rank::King),
            two_cards(Suit::Clubs, Rank::Ace, Rank::King),
        ])
        .unwrap();
        let ct = CurrentTrick::try_new(Direction::North, vec![Card::new(Suit::Spades, Rank::Ace)])
            .unwrap();
        let snap = SnapshotPosition::try_new(hands, ct).unwrap();
        let mut play = PlayPosition::try_from(snap).unwrap();
        let result = play.play_card(Card::new(Suit::Diamonds, Rank::Ace), Strain::NoTrump);
        assert!(result.is_err());
    }
}
