/// Bridge suit. DDS indices: S=0, H=1, D=2, C=3.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Suit {
    Spades,
    Hearts,
    Diamonds,
    Clubs,
}

impl Suit {
    pub fn all() -> [Suit; 4] {
        [Suit::Spades, Suit::Hearts, Suit::Diamonds, Suit::Clubs]
    }

    pub fn as_char(self) -> char {
        match self {
            Suit::Spades => 'S',
            Suit::Hearts => 'H',
            Suit::Diamonds => 'D',
            Suit::Clubs => 'C',
        }
    }

    pub fn from_char(c: char) -> Option<Suit> {
        match c {
            'S' | 's' => Some(Suit::Spades),
            'H' | 'h' => Some(Suit::Hearts),
            'D' | 'd' => Some(Suit::Diamonds),
            'C' | 'c' => Some(Suit::Clubs),
            _ => None,
        }
    }

    pub fn dds_index(self) -> usize {
        match self {
            Suit::Spades => 0,
            Suit::Hearts => 1,
            Suit::Diamonds => 2,
            Suit::Clubs => 3,
        }
    }

    pub fn from_dds_index(i: usize) -> Option<Suit> {
        match i {
            0 => Some(Suit::Spades),
            1 => Some(Suit::Hearts),
            2 => Some(Suit::Diamonds),
            3 => Some(Suit::Clubs),
            _ => None,
        }
    }
}

/// Card rank. Ord: Two < Three < ... < Ace.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Rank {
    Two,
    Three,
    Four,
    Five,
    Six,
    Seven,
    Eight,
    Nine,
    Ten,
    Jack,
    Queen,
    King,
    Ace,
}

impl Rank {
    pub fn all() -> [Rank; 13] {
        [
            Rank::Ace,
            Rank::King,
            Rank::Queen,
            Rank::Jack,
            Rank::Ten,
            Rank::Nine,
            Rank::Eight,
            Rank::Seven,
            Rank::Six,
            Rank::Five,
            Rank::Four,
            Rank::Three,
            Rank::Two,
        ]
    }

    pub fn all_descending() -> [Rank; 13] {
        [
            Rank::Ace,
            Rank::King,
            Rank::Queen,
            Rank::Jack,
            Rank::Ten,
            Rank::Nine,
            Rank::Eight,
            Rank::Seven,
            Rank::Six,
            Rank::Five,
            Rank::Four,
            Rank::Three,
            Rank::Two,
        ]
    }

    pub fn as_char(self) -> char {
        match self {
            Rank::Two => '2',
            Rank::Three => '3',
            Rank::Four => '4',
            Rank::Five => '5',
            Rank::Six => '6',
            Rank::Seven => '7',
            Rank::Eight => '8',
            Rank::Nine => '9',
            Rank::Ten => 'T',
            Rank::Jack => 'J',
            Rank::Queen => 'Q',
            Rank::King => 'K',
            Rank::Ace => 'A',
        }
    }

    pub fn from_char(c: char) -> Option<Rank> {
        match c {
            '2' => Some(Rank::Two),
            '3' => Some(Rank::Three),
            '4' => Some(Rank::Four),
            '5' => Some(Rank::Five),
            '6' => Some(Rank::Six),
            '7' => Some(Rank::Seven),
            '8' => Some(Rank::Eight),
            '9' => Some(Rank::Nine),
            'T' | 't' => Some(Rank::Ten),
            'J' | 'j' => Some(Rank::Jack),
            'Q' | 'q' => Some(Rank::Queen),
            'K' | 'k' => Some(Rank::King),
            'A' | 'a' => Some(Rank::Ace),
            _ => None,
        }
    }

    /// Convert from DDS rank value (2..14) back to Rank.
    pub fn from_dds_score(v: u8) -> Option<Rank> {
        match v {
            2 => Some(Rank::Two),
            3 => Some(Rank::Three),
            4 => Some(Rank::Four),
            5 => Some(Rank::Five),
            6 => Some(Rank::Six),
            7 => Some(Rank::Seven),
            8 => Some(Rank::Eight),
            9 => Some(Rank::Nine),
            10 => Some(Rank::Ten),
            11 => Some(Rank::Jack),
            12 => Some(Rank::Queen),
            13 => Some(Rank::King),
            14 => Some(Rank::Ace),
            _ => None,
        }
    }

    /// Return the DDS rank value used in `currentTrickRank`: 2 (Deuce)..14 (Ace).
    /// This is distinct from `bit_index()`, which uses A=0 for bit storage.
    pub fn dds_rank(self) -> i32 {
        match self {
            Rank::Two => 2,
            Rank::Three => 3,
            Rank::Four => 4,
            Rank::Five => 5,
            Rank::Six => 6,
            Rank::Seven => 7,
            Rank::Eight => 8,
            Rank::Nine => 9,
            Rank::Ten => 10,
            Rank::Jack => 11,
            Rank::Queen => 12,
            Rank::King => 13,
            Rank::Ace => 14,
        }
    }

    /// Convert to a bit position within a suit (0=Ace, 1=King, ..., 12=Two).
    pub fn bit_index(self) -> usize {
        match self {
            Rank::Ace => 0,
            Rank::King => 1,
            Rank::Queen => 2,
            Rank::Jack => 3,
            Rank::Ten => 4,
            Rank::Nine => 5,
            Rank::Eight => 6,
            Rank::Seven => 7,
            Rank::Six => 8,
            Rank::Five => 9,
            Rank::Four => 10,
            Rank::Three => 11,
            Rank::Two => 12,
        }
    }

    /// Convert from bit position (0=Ace) back to Rank.
    pub fn from_bit_index(i: usize) -> Option<Rank> {
        match i {
            0 => Some(Rank::Ace),
            1 => Some(Rank::King),
            2 => Some(Rank::Queen),
            3 => Some(Rank::Jack),
            4 => Some(Rank::Ten),
            5 => Some(Rank::Nine),
            6 => Some(Rank::Eight),
            7 => Some(Rank::Seven),
            8 => Some(Rank::Six),
            9 => Some(Rank::Five),
            10 => Some(Rank::Four),
            11 => Some(Rank::Three),
            12 => Some(Rank::Two),
            _ => None,
        }
    }
}

use super::error::Error;

/// A single playing card.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Card {
    pub suit: Suit,
    pub rank: Rank,
}

impl Card {
    pub fn new(suit: Suit, rank: Rank) -> Self {
        Card { suit, rank }
    }

    /// Return the two-character PBN string for this card, e.g. `"SK"`.
    pub fn to_pbn(self) -> String {
        format!("{}{}", self.suit.as_char(), self.rank.as_char())
    }
}

/// Compass direction. DDS indices: N=0, E=1, S=2, W=3.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Direction {
    North,
    East,
    South,
    West,
}

impl Direction {
    pub fn all() -> [Direction; 4] {
        [
            Direction::North,
            Direction::East,
            Direction::South,
            Direction::West,
        ]
    }

    pub fn partner(self) -> Direction {
        match self {
            Direction::North => Direction::South,
            Direction::South => Direction::North,
            Direction::East => Direction::West,
            Direction::West => Direction::East,
        }
    }

    pub fn next(self) -> Direction {
        match self {
            Direction::North => Direction::East,
            Direction::East => Direction::South,
            Direction::South => Direction::West,
            Direction::West => Direction::North,
        }
    }

    pub fn dds_index(self) -> usize {
        match self {
            Direction::North => 0,
            Direction::East => 1,
            Direction::South => 2,
            Direction::West => 3,
        }
    }

    pub fn advance(self, seats: usize) -> Direction {
        Direction::from_dds_index((self.dds_index() + seats) % 4).unwrap()
    }

    pub fn from_dds_index(i: usize) -> Option<Direction> {
        match i {
            0 => Some(Direction::North),
            1 => Some(Direction::East),
            2 => Some(Direction::South),
            3 => Some(Direction::West),
            _ => None,
        }
    }

    pub fn from_char(c: char) -> Option<Direction> {
        match c {
            'N' | 'n' => Some(Direction::North),
            'E' | 'e' => Some(Direction::East),
            'S' | 's' => Some(Direction::South),
            'W' | 'w' => Some(Direction::West),
            _ => None,
        }
    }

    pub fn as_char(self) -> char {
        match self {
            Direction::North => 'N',
            Direction::East => 'E',
            Direction::South => 'S',
            Direction::West => 'W',
        }
    }
}

/// Denomination including No Trump. DDS indices: S=0, H=1, D=2, C=3, NT=4.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Strain {
    Spades,
    Hearts,
    Diamonds,
    Clubs,
    NoTrump,
}

impl Strain {
    pub fn all_suits() -> [Strain; 4] {
        [
            Strain::Spades,
            Strain::Hearts,
            Strain::Diamonds,
            Strain::Clubs,
        ]
    }

    pub fn all() -> [Strain; 5] {
        [
            Strain::Spades,
            Strain::Hearts,
            Strain::Diamonds,
            Strain::Clubs,
            Strain::NoTrump,
        ]
    }

    pub fn dds_index(self) -> usize {
        match self {
            Strain::Spades => 0,
            Strain::Hearts => 1,
            Strain::Diamonds => 2,
            Strain::Clubs => 3,
            Strain::NoTrump => 4,
        }
    }

    pub fn from_dds_index(i: usize) -> Option<Strain> {
        match i {
            0 => Some(Strain::Spades),
            1 => Some(Strain::Hearts),
            2 => Some(Strain::Diamonds),
            3 => Some(Strain::Clubs),
            4 => Some(Strain::NoTrump),
            _ => None,
        }
    }

    pub fn as_char(self) -> char {
        match self {
            Strain::Spades => 'S',
            Strain::Hearts => 'H',
            Strain::Diamonds => 'D',
            Strain::Clubs => 'C',
            Strain::NoTrump => 'N',
        }
    }

    pub fn from_char(c: char) -> Option<Strain> {
        match c {
            'S' | 's' => Some(Strain::Spades),
            'H' | 'h' => Some(Strain::Hearts),
            'D' | 'd' => Some(Strain::Diamonds),
            'C' | 'c' => Some(Strain::Clubs),
            'N' | 'n' => Some(Strain::NoTrump),
            _ => None,
        }
    }
}

/// One player's hand, stored as a 52-bit mask purely for internal efficiency.
/// The bit layout is our own convention and can change freely; all DDS
/// communication uses PBN strings. The public API only speaks `Card` values.
///
/// Bit layout:
///   bits  0-12  S-A, S-K, ..., S-2  (bit 0 = highest rank)
///   bits 13-25  H-A, H-K, ..., H-2
///   bits 26-38  D-A, D-K, ..., D-2
///   bits 39-51  C-A, C-K, ..., C-2
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Hand(u64);

impl Hand {
    pub fn empty() -> Self {
        Hand(0)
    }

    pub fn from_cards(cards: &[Card]) -> Result<Self, Error> {
        let mut bits: u64 = 0;
        for card in cards {
            let pos = card.suit.dds_index() * 13 + card.rank.bit_index();
            let mask = 1u64 << pos;
            if bits & mask != 0 {
                return Err(Error::InvalidDeal(format!(
                    "duplicate card: {}{}",
                    card.suit.as_char(),
                    card.rank.as_char()
                )));
            }
            bits |= mask;
        }
        Ok(Hand(bits))
    }

    pub fn cards(&self) -> impl Iterator<Item = Card> {
        let bits = self.0;
        (0..52).filter_map(move |i| {
            if (bits >> i) & 1 != 0 {
                let suit = Suit::from_dds_index(i / 13)?;
                let rank = Rank::from_bit_index(i % 13)?;
                Some(Card::new(suit, rank))
            } else {
                None
            }
        })
    }

    pub fn contains(&self, card: Card) -> bool {
        let pos = card.suit.dds_index() * 13 + card.rank.bit_index();
        (self.0 >> pos) & 1 != 0
    }

    /// Whether the hand contains any card of the given suit.
    pub fn has_suit(&self, suit: Suit) -> bool {
        let shift = suit.dds_index() * 13;
        let suit_mask: u64 = 0x1FFF << shift;
        (self.0 & suit_mask) != 0
    }

    pub fn len(&self) -> usize {
        self.0.count_ones() as usize
    }

    pub fn is_empty(&self) -> bool {
        self.0 == 0
    }

    pub fn remove(&self, card: Card) -> Self {
        let pos = card.suit.dds_index() * 13 + card.rank.bit_index();
        Hand(self.0 & !(1u64 << pos))
    }

    pub fn add(&self, card: Card) -> Result<Self, Error> {
        if self.contains(card) {
            return Err(Error::InvalidDeal(format!(
                "duplicate card: {}{}",
                card.suit.as_char(),
                card.rank.as_char()
            )));
        }
        let pos = card.suit.dds_index() * 13 + card.rank.bit_index();
        Ok(Hand(self.0 | (1u64 << pos)))
    }
}

/// Error when constructing or mutating `Hands`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HandsError {
    TooManyCards { direction: Direction, count: usize },
    DuplicateCard { card: Card },
    CardNotHeld { direction: Direction, card: Card },
}

impl std::fmt::Display for HandsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HandsError::TooManyCards { direction, count } => {
                write!(f, "{:?} has {} cards (max 13)", direction, count)
            }
            HandsError::DuplicateCard { card } => {
                write!(
                    f,
                    "duplicate card: {}{}",
                    card.suit.as_char(),
                    card.rank.as_char()
                )
            }
            HandsError::CardNotHeld { direction, card } => {
                write!(
                    f,
                    "{:?} does not hold {}{}",
                    direction,
                    card.suit.as_char(),
                    card.rank.as_char()
                )
            }
        }
    }
}

/// Four hands indexed by absolute direction (`N`, `E`, `S`, `W`).
///
/// Invariants enforced at construction and by every mutation:
/// - Each hand contains at most 13 cards.
/// - No card occurs in more than one hand.
/// - Single-hand duplicate cards are already prevented by `Hand`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hands {
    hands: [Hand; 4],
}

impl Hands {
    pub fn try_new(hands: [Hand; 4]) -> Result<Self, HandsError> {
        let mut seen: u64 = 0;
        for (i, hand) in hands.iter().enumerate() {
            if hand.len() > 13 {
                return Err(HandsError::TooManyCards {
                    direction: Direction::from_dds_index(i).unwrap(),
                    count: hand.len(),
                });
            }
            for card in hand.cards() {
                let pos = card.suit.dds_index() * 13 + card.rank.bit_index();
                let mask = 1u64 << pos;
                if seen & mask != 0 {
                    return Err(HandsError::DuplicateCard { card });
                }
                seen |= mask;
            }
        }
        Ok(Hands { hands })
    }

    pub fn get(&self, direction: Direction) -> &Hand {
        &self.hands[direction.dds_index()]
    }

    pub fn iter(&self) -> impl Iterator<Item = (Direction, &Hand)> {
        Direction::all()
            .into_iter()
            .map(|direction| (direction, self.get(direction)))
    }

    pub fn counts(&self) -> [usize; 4] {
        [
            self.hands[0].len(),
            self.hands[1].len(),
            self.hands[2].len(),
            self.hands[3].len(),
        ]
    }

    pub fn total_count(&self) -> usize {
        self.hands.iter().map(|h| h.len()).sum()
    }

    pub fn owner_of(&self, card: Card) -> Option<Direction> {
        for (i, hand) in self.hands.iter().enumerate() {
            if hand.contains(card) {
                return Direction::from_dds_index(i);
            }
        }
        None
    }

    pub fn remove(&self, direction: Direction, card: Card) -> Result<Self, HandsError> {
        let idx = direction.dds_index();
        if !self.hands[idx].contains(card) {
            return Err(HandsError::CardNotHeld { direction, card });
        }
        let mut new_hands = self.hands;
        new_hands[idx] = self.hands[idx].remove(card);
        Ok(Hands { hands: new_hands })
    }

    pub fn add(&self, direction: Direction, card: Card) -> Result<Self, HandsError> {
        let idx = direction.dds_index();
        if self.hands[idx].len() >= 13 {
            return Err(HandsError::TooManyCards {
                direction,
                count: self.hands[idx].len() + 1,
            });
        }
        if self.owner_of(card).is_some() {
            return Err(HandsError::DuplicateCard { card });
        }
        let mut new_hands = self.hands;
        new_hands[idx] = self.hands[idx].add(card).map_err(|_| {
            // add() only fails on duplicates at Hand level, but we already checked
            HandsError::DuplicateCard { card }
        })?;
        Ok(Hands { hands: new_hands })
    }
}

/// Four hands plus the `<first>` direction from the Deal tag.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Deal {
    /// The `<first>` direction: whose hand is listed first in the PBN deal.
    first: Direction,
    /// Hands indexed by absolute direction.
    hands: Hands,
}

impl Deal {
    pub fn try_new(first: Direction, hands: Hands) -> Result<Self, Error> {
        let counts = hands.counts();
        for (direction, count) in Direction::all().into_iter().zip(counts) {
            if count != 13 {
                return Err(Error::InvalidDeal(format!(
                    "{} has {} cards, expected 13",
                    direction.as_char(),
                    count
                )));
            }
        }
        if hands.total_count() != 52 {
            return Err(Error::InvalidDeal(format!(
                "board has {} cards, expected 52",
                hands.total_count()
            )));
        }
        Ok(Deal { first, hands })
    }

    pub fn first(&self) -> Direction {
        self.first
    }

    pub fn hands(&self) -> &Hands {
        &self.hands
    }
}

/// A complete board parsed from a PBN record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Board {
    /// Dealer (from the `Dealer` tag). Distinct from `deal.first`.
    pub dealer: Direction,
    /// Dealt hands.
    pub deal: Deal,
    /// Vulnerability (from the `Vulnerable` tag).
    pub vulnerable: Vulnerability,
}

/// Which side: NS or EW.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    NS,
    EW,
}

/// Vulnerability encoding matches DDS: 0=None, 1=Both, 2=NS, 3=EW.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Vulnerability {
    None = 0,
    Both = 1,
    NS = 2,
    EW = 3,
}

impl Vulnerability {
    pub fn dds_code(self) -> i32 {
        self as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hand_roundtrip() {
        let cards = vec![
            Card::new(Suit::Spades, Rank::Ace),
            Card::new(Suit::Hearts, Rank::King),
            Card::new(Suit::Diamonds, Rank::Queen),
            Card::new(Suit::Clubs, Rank::Two),
        ];
        let hand = Hand::from_cards(&cards).unwrap();
        assert_eq!(hand.len(), 4);
        let recovered: Vec<Card> = hand.cards().collect();
        assert_eq!(recovered.len(), 4);
        for c in &cards {
            assert!(hand.contains(*c));
            let removed = hand.remove(*c);
            assert_eq!(removed.len(), 3);
            assert!(!removed.contains(*c));
            let added_back = removed.add(*c).unwrap();
            assert_eq!(added_back.len(), 4);
            assert!(added_back.contains(*c));
        }
    }

    #[test]
    fn test_hand_empty() {
        let hand = Hand::empty();
        assert_eq!(hand.len(), 0);
        assert!(hand.cards().next().is_none());
    }

    #[test]
    fn test_rank_ordering() {
        assert!(Rank::Ace > Rank::King);
        assert!(Rank::Ace > Rank::Two);
        assert!(Rank::King > Rank::Queen);
        assert!(Rank::Two < Rank::Three);
    }

    #[test]
    fn test_suit_all_count() {
        assert_eq!(Suit::all().len(), 4);
    }

    #[test]
    fn test_rank_all_count() {
        assert_eq!(Rank::all().len(), 13);
        assert_eq!(Rank::all_descending().len(), 13);
    }

    #[test]
    fn test_direction_partner() {
        assert_eq!(Direction::North.partner(), Direction::South);
        assert_eq!(Direction::East.partner(), Direction::West);
        assert_eq!(Direction::South.partner(), Direction::North);
        assert_eq!(Direction::West.partner(), Direction::East);
    }

    #[test]
    fn test_direction_next() {
        assert_eq!(Direction::North.next(), Direction::East);
        assert_eq!(Direction::East.next(), Direction::South);
        assert_eq!(Direction::South.next(), Direction::West);
        assert_eq!(Direction::West.next(), Direction::North);
    }

    #[test]
    fn test_strain_dds_indices() {
        assert_eq!(Strain::Spades.dds_index(), 0);
        assert_eq!(Strain::Hearts.dds_index(), 1);
        assert_eq!(Strain::Diamonds.dds_index(), 2);
        assert_eq!(Strain::Clubs.dds_index(), 3);
        assert_eq!(Strain::NoTrump.dds_index(), 4);
    }

    #[test]
    fn test_direction_advance() {
        assert_eq!(Direction::North.advance(0), Direction::North);
        assert_eq!(Direction::North.advance(1), Direction::East);
        assert_eq!(Direction::North.advance(4), Direction::North);
        assert_eq!(Direction::East.advance(3), Direction::North);
    }

    #[test]
    fn test_hands_try_new_valid() {
        let hands = [
            Hand::from_cards(&[
                Card::new(Suit::Spades, Rank::Ace),
                Card::new(Suit::Spades, Rank::King),
            ])
            .unwrap(),
            Hand::from_cards(&[Card::new(Suit::Hearts, Rank::Ace)]).unwrap(),
            Hand::from_cards(&[Card::new(Suit::Diamonds, Rank::Queen)]).unwrap(),
            Hand::from_cards(&[Card::new(Suit::Clubs, Rank::Two)]).unwrap(),
        ];
        let h = Hands::try_new(hands).unwrap();
        assert_eq!(h.total_count(), 5);
        assert_eq!(h.counts(), [2, 1, 1, 1]);
    }

    #[test]
    fn test_hands_try_new_rejects_too_many_cards() {
        let mut cards: Vec<Card> = Rank::all()
            .into_iter()
            .map(|rank| Card::new(Suit::Spades, rank))
            .collect();
        cards.push(Card::new(Suit::Hearts, Rank::Ace));
        let result = Hands::try_new([
            Hand::from_cards(&cards).unwrap(),
            Hand::empty(),
            Hand::empty(),
            Hand::empty(),
        ]);
        assert_eq!(
            result.unwrap_err(),
            HandsError::TooManyCards {
                direction: Direction::North,
                count: 14,
            }
        );
    }

    #[test]
    fn test_hands_iter_uses_direction_order() {
        let cards = [
            Card::new(Suit::Spades, Rank::Ace),
            Card::new(Suit::Hearts, Rank::King),
            Card::new(Suit::Diamonds, Rank::Queen),
            Card::new(Suit::Clubs, Rank::Jack),
        ];
        let hands = Hands::try_new([
            Hand::from_cards(&cards[0..1]).unwrap(),
            Hand::from_cards(&cards[1..2]).unwrap(),
            Hand::from_cards(&cards[2..3]).unwrap(),
            Hand::from_cards(&cards[3..4]).unwrap(),
        ])
        .unwrap();

        let entries: Vec<_> = hands.iter().collect();
        assert_eq!(
            entries
                .iter()
                .map(|(direction, _)| *direction)
                .collect::<Vec<_>>(),
            Direction::all()
        );
        for ((direction, hand), card) in entries.into_iter().zip(cards) {
            assert!(hand.contains(card));
            assert_eq!(hands.owner_of(card), Some(direction));
        }
    }

    #[test]
    fn test_deal_requires_complete_hands() {
        let hands = Hands::try_new([
            Hand::from_cards(&[Card::new(Suit::Spades, Rank::Ace)]).unwrap(),
            Hand::from_cards(&[Card::new(Suit::Hearts, Rank::Ace)]).unwrap(),
            Hand::from_cards(&[Card::new(Suit::Diamonds, Rank::Ace)]).unwrap(),
            Hand::from_cards(&[Card::new(Suit::Clubs, Rank::Ace)]).unwrap(),
        ])
        .unwrap();

        assert!(Deal::try_new(Direction::North, hands).is_err());
    }

    #[test]
    fn test_hands_add_rejects_when_full() {
        let all_spades: Vec<Card> = Rank::all()
            .iter()
            .map(|&r| Card::new(Suit::Spades, r))
            .collect();
        let hands = [
            Hand::from_cards(&all_spades).unwrap(),
            Hand::empty(),
            Hand::empty(),
            Hand::empty(),
        ];
        let h = Hands::try_new(hands).unwrap();
        assert_eq!(h.get(Direction::North).len(), 13);
        let result = h.add(Direction::North, Card::new(Suit::Hearts, Rank::Ace));
        assert!(matches!(result, Err(HandsError::TooManyCards { .. })));
    }

    #[test]
    fn test_hands_rejects_duplicate_across_hands() {
        let sa = Card::new(Suit::Spades, Rank::Ace);
        let hands = [
            Hand::from_cards(&[sa]).unwrap(),
            Hand::from_cards(&[sa]).unwrap(),
            Hand::empty(),
            Hand::empty(),
        ];
        assert!(Hands::try_new(hands).is_err());
    }

    #[test]
    fn test_hands_remove_and_add() {
        let sa = Card::new(Suit::Spades, Rank::Ace);
        let hands = [
            Hand::from_cards(&[sa]).unwrap(),
            Hand::from_cards(&[Card::new(Suit::Hearts, Rank::Ace)]).unwrap(),
            Hand::empty(),
            Hand::empty(),
        ];
        let h = Hands::try_new(hands).unwrap();
        assert_eq!(h.owner_of(sa), Some(Direction::North));

        let h2 = h.remove(Direction::North, sa).unwrap();
        assert_eq!(h2.total_count(), 1);
        assert!(h2.owner_of(sa).is_none());

        let h3 = h2.add(Direction::North, sa).unwrap();
        assert_eq!(h3.total_count(), 2);
        assert_eq!(h3.owner_of(sa), Some(Direction::North));
    }

    #[test]
    fn test_suit_from_dds_index() {
        assert_eq!(Suit::from_dds_index(0), Some(Suit::Spades));
        assert_eq!(Suit::from_dds_index(3), Some(Suit::Clubs));
        assert_eq!(Suit::from_dds_index(4), None);
    }
}
