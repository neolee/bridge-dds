use super::deal::{Card, Direction};
use super::error::Error;

/// Parse a PBN `Play` tag value into a flat sequence of cards in play order.
///
/// Format: optional leading direction (`W:`), then cards. Tricks are separated
/// by whitespace; cards within a trick are separated by `=`. Example:
/// `"W:S6=S4=SJ=SQ=S3=S7=S9=SK"`.
pub fn parse_play_tag(value: &str) -> Result<(Option<Direction>, Vec<Card>), Error> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok((None, vec![]));
    }

    // Check for optional leading direction prefix like "W:".
    let (tag_leader, card_data) = if trimmed.len() >= 2
        && &trimmed[1..2] == ":"
        && Direction::from_char(trimmed.chars().next().unwrap()).is_some()
    {
        let dir = Direction::from_char(trimmed.chars().next().unwrap());
        (dir, &trimmed[2..])
    } else {
        (None, trimmed)
    };

    let cards = parse_card_sequence(card_data)?;
    Ok((tag_leader, cards))
}

/// Parse a sequence of cards where tricks are space-separated and cards
/// within a trick are `=`-separated. Returns cards in play order.
fn parse_card_sequence(data: &str) -> Result<Vec<Card>, Error> {
    let mut cards = Vec::new();

    for trick_str in data.split_whitespace() {
        for card_str in trick_str.split('=') {
            let card_str = card_str.trim();
            if card_str.is_empty() {
                continue;
            }
            if card_str.len() != 2 {
                return Err(Error::InvalidPlayTrace(format!(
                    "invalid card '{}'; expected format SA",
                    card_str
                )));
            }
            let mut chars = card_str.chars();
            let suit = super::deal::Suit::from_char(chars.next().unwrap()).ok_or_else(|| {
                Error::InvalidPlayTrace(format!("invalid suit in '{}'", card_str))
            })?;
            let rank = super::deal::Rank::from_char(chars.next().unwrap()).ok_or_else(|| {
                Error::InvalidPlayTrace(format!("invalid rank in '{}'", card_str))
            })?;
            cards.push(Card::new(suit, rank));
        }
    }

    Ok(cards)
}

#[cfg(test)]
mod tests {
    use super::super::deal::{Rank, Suit};
    use super::*;

    #[test]
    fn test_parse_play_tag_basic() {
        let (leader, cards) = parse_play_tag("W:S6=S4=SJ=SQ=S3=S7=S9=SK").unwrap();
        assert_eq!(leader, Some(Direction::West));
        assert_eq!(cards.len(), 8);
        assert_eq!(cards[0], Card::new(Suit::Spades, Rank::Six));
        assert_eq!(cards[7], Card::new(Suit::Spades, Rank::King));
    }

    #[test]
    fn test_parse_play_tag_no_prefix() {
        let (leader, cards) = parse_play_tag("S6=S4=SJ=SQ").unwrap();
        assert_eq!(leader, None);
        assert_eq!(cards.len(), 4);
    }

    #[test]
    fn test_parse_play_tag_multi_trick() {
        let (leader, cards) = parse_play_tag("N:SA=HK=DQ=CJ S2=H3=D4=C5").unwrap();
        assert_eq!(leader, Some(Direction::North));
        assert_eq!(cards.len(), 8);
    }

    #[test]
    fn test_parse_play_tag_empty() {
        let (leader, cards) = parse_play_tag("").unwrap();
        assert_eq!(leader, None);
        assert!(cards.is_empty());
    }

    #[test]
    fn test_parse_play_tag_invalid_card() {
        assert!(parse_play_tag("N:SA=H").is_err());
    }
}
