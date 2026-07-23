use super::deal::{Board, Card, Deal, Direction, Hand, Hands, Rank, Strain, Suit, Vulnerability};
use super::error::Error;

/// Parse a single PBN record into a `Board`.
pub fn parse_record(input: &str) -> Result<Board, Error> {
    let mut deal: Option<Deal> = None;
    let mut dealer: Option<Direction> = None;
    let mut vulnerable: Option<Vulnerability> = None;

    for line in input.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let (tag, value) = parse_tag_line(line)?;
        match tag {
            "Deal" => {
                if deal.is_some() {
                    return Err(Error::DuplicatePbnTag("Deal"));
                }
                deal = Some(parse_deal_tag(value)?);
            }
            "Dealer" => {
                if dealer.is_some() {
                    return Err(Error::DuplicatePbnTag("Dealer"));
                }
                dealer = Some(parse_dealer_tag(value)?);
            }
            "Vulnerable" => {
                if vulnerable.is_some() {
                    return Err(Error::DuplicatePbnTag("Vulnerable"));
                }
                vulnerable = Some(parse_vulnerable_tag(value)?);
            }
            // Unknown tags are ignored.
            _ => {}
        }
    }

    let deal = deal.ok_or(Error::MissingPbnTag("Deal"))?;
    let dealer = dealer.ok_or(Error::MissingPbnTag("Dealer"))?;
    let vulnerable = vulnerable.ok_or(Error::MissingPbnTag("Vulnerable"))?;

    Ok(Board {
        dealer,
        deal,
        vulnerable,
    })
}

fn parse_tag_line(line: &str) -> Result<(&str, &str), Error> {
    let line = line.trim();
    if !line.starts_with('[') || !line.ends_with(']') {
        return Err(Error::PbnParse(format!("invalid tag line: {}", line)));
    }
    let inner = &line[1..line.len() - 1]; // strip [ and ]
    let space_pos = inner
        .find(' ')
        .ok_or_else(|| Error::PbnParse(format!("missing space in tag line: {}", line)))?;
    let tag = &inner[..space_pos];
    let raw_value = inner[space_pos + 1..].trim();
    // Strip surrounding double quotes.
    let value = if raw_value.starts_with('"') && raw_value.ends_with('"') {
        &raw_value[1..raw_value.len() - 1]
    } else {
        return Err(Error::PbnParse(format!(
            "tag value must be quoted: {}",
            line
        )));
    };
    Ok((tag, value))
}

/// Parse a `Deal` tag value.
///
/// Format: `<first>:<hand1> <hand2> <hand3> <hand4>`
/// Hands are clockwise from `<first>`, each hand is four suit fields in `S.H.D.C` order.
pub fn parse_deal_tag(value: &str) -> Result<Deal, Error> {
    // Find the <first> prefix.
    let colon_pos = value
        .find(':')
        .ok_or_else(|| Error::PbnParse(format!("missing ':' in deal tag: {}", value)))?;
    if colon_pos != 1 {
        return Err(Error::InvalidDeal(format!(
            "<first> must be a single direction letter before ':', got '{}'",
            &value[..colon_pos]
        )));
    }
    let first = Direction::from_char(value.chars().next().unwrap())
        .ok_or_else(|| Error::InvalidDeal(format!("invalid <first> direction: {}", &value[..1])))?;

    let hands_str = &value[colon_pos + 1..];
    let hand_strs: Vec<&str> = hands_str.split_whitespace().collect();
    if hand_strs.len() != 4 {
        return Err(Error::InvalidDeal(format!(
            "expected 4 hands, got {}",
            hand_strs.len()
        )));
    }

    // Reject partial deals: no hand may be "-".
    for (i, hs) in hand_strs.iter().enumerate() {
        if *hs == "-" {
            return Err(Error::InvalidDeal(format!(
                "hand {} is incomplete ('-'); partial deals not supported",
                i + 1
            )));
        }
    }

    // Parse each hand in clockwise order, then map to N/E/S/W indices.
    let mut hands = [Hand::empty(); 4];
    for (i, hand_str) in hand_strs.iter().enumerate() {
        let dest_idx = (first.dds_index() + i) % 4;
        let cards = parse_hand_pbn(hand_str)?;
        hands[dest_idx] = Hand::from_cards(&cards)?;
    }

    let hands = Hands::try_new(hands).map_err(|error| Error::InvalidDeal(error.to_string()))?;
    Deal::try_new(first, hands)
}

/// Parse a single hand in PBN format: `S.H.D.C`, e.g. `AKQJT98..8642`.
fn parse_hand_pbn(hand_str: &str) -> Result<Vec<Card>, Error> {
    let suit_strs: Vec<&str> = hand_str.split('.').collect();
    if suit_strs.len() != 4 {
        return Err(Error::InvalidDeal(format!(
            "hand '{}' does not have 4 suit fields separated by '.'",
            hand_str
        )));
    }
    let suits = Suit::all(); // S, H, D, C
    let mut cards = Vec::with_capacity(13);

    for (suit_idx, suit_str) in suit_strs.iter().enumerate() {
        for ch in suit_str.chars() {
            let rank = Rank::from_char(ch).ok_or_else(|| {
                Error::InvalidDeal(format!("invalid rank char '{}' in hand '{}'", ch, hand_str))
            })?;
            cards.push(Card::new(suits[suit_idx], rank));
        }
    }
    Ok(cards)
}

fn parse_dealer_tag(value: &str) -> Result<Direction, Error> {
    let trimmed = value.trim();
    if trimmed.len() != 1 {
        return Err(Error::InvalidPbnTag {
            tag: "Dealer",
            value: value.to_string(),
        });
    }
    Direction::from_char(trimmed.chars().next().unwrap()).ok_or_else(|| Error::InvalidPbnTag {
        tag: "Dealer",
        value: value.to_string(),
    })
}

fn parse_vulnerable_tag(value: &str) -> Result<Vulnerability, Error> {
    match value {
        "None" | "Love" | "-" => Ok(Vulnerability::None),
        "NS" => Ok(Vulnerability::NS),
        "EW" => Ok(Vulnerability::EW),
        "All" | "Both" => Ok(Vulnerability::Both),
        _ => Err(Error::InvalidPbnTag {
            tag: "Vulnerable",
            value: value.to_string(),
        }),
    }
}

/// Serialize a `Deal` into the format expected by DDS `ddTableDealPBN.cards`.
///
/// Output format: `<first>:<hand1> <hand2> <hand3> <hand4>` where hands are
/// emitted clockwise from `Deal::first()`, suit order `S.H.D.C`, descending ranks.
pub fn deal_to_dds_pbn(deal: &Deal) -> String {
    let first = deal.first();
    let mut parts = Vec::with_capacity(4);

    // Emit hands clockwise from first.
    for i in 0..4 {
        let direction = first.advance(i);
        parts.push(hand_to_pbn(deal.hands().get(direction)));
    }

    format!(
        "{}:{} {} {} {}",
        first.as_char(),
        parts[0],
        parts[1],
        parts[2],
        parts[3]
    )
}

/// Serialize a single hand to a PBN fragment: `S-cards.H-cards.D-cards.C-cards`.
fn hand_to_pbn(hand: &Hand) -> String {
    let suits = Suit::all();
    let suit_strs: Vec<String> = suits
        .iter()
        .map(|suit| {
            let suit_cards: String = Rank::all_descending()
                .iter()
                .filter(|rank| hand.contains(Card::new(*suit, **rank)))
                .map(|rank| rank.as_char())
                .collect();
            suit_cards
        })
        .collect();
    suit_strs.join(".")
}

/// Serialize four hands into the PBN string format expected by
/// `dealPBN.remainCards`. Emits hands clockwise from `first_hand`,
/// suit order `S.H.D.C`, descending ranks within each suit.
pub fn hands_to_dds_pbn(hands: &Hands, first_hand: Direction) -> String {
    let mut parts = Vec::with_capacity(4);
    for i in 0..4 {
        let direction = first_hand.advance(i);
        parts.push(hand_to_pbn(hands.get(direction)));
    }
    format!(
        "{}:{} {} {} {}",
        first_hand.as_char(),
        parts[0],
        parts[1],
        parts[2],
        parts[3]
    )
}

/// Parsed residual position from a PBN record.
#[derive(Debug)]
pub struct ResidualInput {
    pub hands: Hands,
    pub first: Option<Direction>,
    pub trump: Option<Strain>,
    pub current_trick: Vec<(Direction, Card)>,
}

/// Parse a residual PBN record containing `Position`, `First`, and optional `Trump`/
/// Parse a residual PBN record containing `Position`, `First`, optional `Trump`
/// and `CurrentTrick` tags.
pub fn parse_residual_record(input: &str) -> Result<ResidualInput, Error> {
    let mut hands: Option<Hands> = None;
    let mut first: Option<Direction> = None;
    let mut trump: Option<Strain> = None;
    let mut current_trick: Vec<(Direction, Card)> = vec![];

    for line in input.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let (tag, value) = parse_tag_line(line)?;
        match tag {
            "Position" => {
                if hands.is_some() {
                    return Err(Error::DuplicatePbnTag("Position"));
                }
                hands = Some(parse_residual_hands(value)?);
            }
            "First" => {
                if first.is_some() {
                    return Err(Error::DuplicatePbnTag("First"));
                }
                first = Some(parse_first_tag(value)?);
            }
            "Trump" => {
                if trump.is_some() {
                    return Err(Error::DuplicatePbnTag("Trump"));
                }
                trump = Some(parse_trump_tag(value)?);
            }
            "CurrentTrick" => {
                for entry in value.split_whitespace() {
                    if entry.len() < 3 || !entry.contains(':') {
                        return Err(Error::PbnParse(format!(
                            "invalid CurrentTrick entry '{}'; expected format N:SA",
                            entry
                        )));
                    }
                    let colon = entry.find(':').unwrap();
                    let dir_str = &entry[..colon];
                    let card_str = &entry[colon + 1..];
                    let dir = Direction::from_char(dir_str.chars().next().unwrap_or(' '))
                        .ok_or_else(|| {
                            Error::PbnParse(format!(
                                "invalid direction in CurrentTrick: {}",
                                dir_str
                            ))
                        })?;
                    if card_str.len() != 2 {
                        return Err(Error::PbnParse(format!(
                            "invalid card in CurrentTrick: {}",
                            card_str
                        )));
                    }
                    let mut chars = card_str.chars();
                    let suit = Suit::from_char(chars.next().unwrap()).ok_or_else(|| {
                        Error::PbnParse(format!("invalid suit in CurrentTrick: {}", card_str))
                    })?;
                    let rank = Rank::from_char(chars.next().unwrap()).ok_or_else(|| {
                        Error::PbnParse(format!("invalid rank in CurrentTrick: {}", card_str))
                    })?;
                    current_trick.push((dir, Card::new(suit, rank)));
                }
            }
            // Unknown tags are ignored.
            _ => {}
        }
    }

    let hands = hands.ok_or(Error::MissingPbnTag("Position"))?;

    // Validate CurrentTrick: each card must be held by the claimed player.
    for (player, card) in &current_trick {
        if !hands.get(*player).contains(*card) {
            return Err(Error::InvalidPosition(format!(
                "CurrentTrick: {:?} does not hold {}{}",
                player,
                card.suit.as_char(),
                card.rank.as_char()
            )));
        }
    }

    Ok(ResidualInput {
        hands,
        first,
        trump,
        current_trick,
    })
}

/// Parse residual hands: same four-hand clockwise format as Deal, but each
/// hand may contain fewer than 13 cards. All hands must have equal count
/// (only clean trick boundaries are accepted via PBN input).
fn parse_residual_hands(value: &str) -> Result<Hands, Error> {
    let colon_pos = value
        .find(':')
        .ok_or_else(|| Error::PbnParse(format!("missing ':' in Position tag: {}", value)))?;
    if colon_pos != 1 {
        return Err(Error::InvalidDeal(
            "Position <first> must be a single direction letter before ':'".into(),
        ));
    }
    let first = Direction::from_char(value.chars().next().unwrap())
        .ok_or_else(|| Error::InvalidDeal(format!("invalid <first> direction: {}", &value[..1])))?;

    let hands_str = &value[colon_pos + 1..];
    let hand_strs: Vec<&str> = hands_str.split_whitespace().collect();
    if hand_strs.len() != 4 {
        return Err(Error::InvalidDeal(format!(
            "expected 4 hands in Position, got {}",
            hand_strs.len()
        )));
    }

    let mut hands = [Hand::empty(); 4];

    for (i, hand_str) in hand_strs.iter().enumerate() {
        let dest_idx = (first.dds_index() + i) % 4;
        let cards = parse_hand_pbn(hand_str)?;
        hands[dest_idx] = Hand::from_cards(&cards)?;
    }

    Hands::try_new(hands).map_err(|error| Error::InvalidPosition(error.to_string()))
}

fn parse_first_tag(value: &str) -> Result<Direction, Error> {
    let trimmed = value.trim();
    if trimmed.len() != 1 {
        return Err(Error::InvalidFirst(format!(
            "'{}' is not a valid direction",
            value
        )));
    }
    Direction::from_char(trimmed.chars().next().unwrap())
        .ok_or_else(|| Error::InvalidFirst(format!("invalid direction: '{}'", value)))
}

fn parse_trump_tag(value: &str) -> Result<Strain, Error> {
    Strain::from_char(value.trim().chars().next().unwrap_or(' '))
        .ok_or_else(|| Error::InvalidTrump(format!("'{}' is not a valid trump", value)))
}

#[cfg(test)]
mod tests {
    use super::*;

    static VALID_RECORD: &str = "\
[Deal \"N:QJ6.K652.J85.T98 873.J97.AT764.Q4 K5.T83.KQ9.A7652 AT942.AQ4.32.KJ3\"]
[Dealer \"N\"]
[Vulnerable \"None\"]
";

    #[test]
    fn test_parse_valid_record() {
        let board = parse_record(VALID_RECORD).unwrap();
        assert_eq!(board.dealer, Direction::North);
        assert_eq!(board.deal.first(), Direction::North);
        assert_eq!(board.vulnerable, Vulnerability::None);
        // Verify 52 cards exactly once.
        let mut count = 0;
        for (_, hand) in board.deal.hands().iter() {
            count += hand.len();
        }
        assert_eq!(count, 52);
    }

    #[test]
    fn test_deal_to_dds_pbn_roundtrip() {
        let board = parse_record(VALID_RECORD).unwrap();
        let pbn = deal_to_dds_pbn(&board.deal);
        // Re-parse and compare.
        let deal2 = parse_deal_tag(&pbn).unwrap();
        assert_eq!(board.deal, deal2);
    }

    #[test]
    fn test_missing_dealer() {
        let result = parse_record("[Deal \"N:QJ6.K652.J85.T98 873.J97.AT764.Q4 K5.T83.KQ9.A7652 AT942.AQ4.32.KJ3\"]\n[Vulnerable \"None\"]\n");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Dealer"));
    }

    #[test]
    fn test_duplicate_dealer() {
        let result = parse_record(
            "[Deal \"N:QJ6.K652.J85.T98 873.J97.AT764.Q4 K5.T83.KQ9.A7652 AT942.AQ4.32.KJ3\"]\n[Dealer \"N\"]\n[Dealer \"S\"]\n[Vulnerable \"None\"]\n",
        );
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("duplicate"));
    }

    #[test]
    fn test_vulnerable_aliases() {
        let base = "[Deal \"N:QJ6.K652.J85.T98 873.J97.AT764.Q4 K5.T83.KQ9.A7652 AT942.AQ4.32.KJ3\"]\n[Dealer \"N\"]\n";
        for (val, expected) in &[
            ("None", Vulnerability::None),
            ("Love", Vulnerability::None),
            ("-", Vulnerability::None),
            ("NS", Vulnerability::NS),
            ("EW", Vulnerability::EW),
            ("All", Vulnerability::Both),
            ("Both", Vulnerability::Both),
        ] {
            let rec = format!("{}[Vulnerable \"{}\"]\n", base, val);
            let board = parse_record(&rec).unwrap();
            assert_eq!(
                board.vulnerable, *expected,
                "Vulnerable '{}' did not map correctly",
                val
            );
        }
    }

    #[test]
    fn test_partial_deal_rejected() {
        let result = parse_record(
            "[Deal \"N:QJ6.K652.J85.T98 - K5.T83.KQ9.A7652 AT942.AQ4.32.KJ3\"]\n[Dealer \"N\"]\n[Vulnerable \"None\"]\n",
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_deal_first_differs_from_dealer() {
        let rec = "\
[Deal \"E:QJ6.K652.J85.T98 873.J97.AT764.Q4 K5.T83.KQ9.A7652 AT942.AQ4.32.KJ3\"]
[Dealer \"N\"]
[Vulnerable \"None\"]
";
        let board = parse_record(rec).unwrap();
        // Deal.first is E because that's what the deal string says.
        assert_eq!(board.deal.first(), Direction::East);
        // Dealer is N because the Dealer tag says so.
        assert_eq!(board.dealer, Direction::North);
    }

    #[test]
    fn test_reject_multi_char_dealer() {
        let rec = "\
[Deal \"N:QJ6.K652.J85.T98 873.J97.AT764.Q4 K5.T83.KQ9.A7652 AT942.AQ4.32.KJ3\"]
[Dealer \"North\"]
[Vulnerable \"None\"]
";
        assert!(parse_record(rec).is_err());
    }

    #[test]
    fn test_reject_unquoted_tag_value() {
        let rec = "\
[Deal \"N:QJ6.K652.J85.T98 873.J97.AT764.Q4 K5.T83.KQ9.A7652 AT942.AQ4.32.KJ3\"]
[Dealer N]
[Vulnerable \"None\"]
";
        assert!(parse_record(rec).is_err());
    }

    #[test]
    fn test_reject_multi_char_deal_first() {
        let rec = "\
[Deal \"North:QJ6.K652.J85.T98 873.J97.AT764.Q4 K5.T83.KQ9.A7652 AT942.AQ4.32.KJ3\"]
[Dealer \"N\"]
[Vulnerable \"None\"]
";
        assert!(parse_record(rec).is_err());
    }
}
