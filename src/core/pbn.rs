use std::collections::HashSet;

use super::deal::{Card, Deal, Direction, Hand, Hands, Rank, Strain, Suit, Vulnerability};
use super::error::Error;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ParsedRecord {
    pub deal: Option<Deal>,
    pub dealer: Option<Direction>,
    pub vulnerable: Option<Vulnerability>,
    pub position: Option<Hands>,
    pub first: Option<Direction>,
    pub trump: Option<Strain>,
    pub current_trick: Option<ParsedCurrentTrick>,
    pub contract: Option<ParsedContract>,
    pub declarer: Option<Direction>,
    pub auction: Option<ParsedAuction>,
    pub play: Option<ParsedPlay>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DirectedCard {
    pub player: Direction,
    pub card: Card,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedCurrentTrick {
    pub cards: Vec<DirectedCard>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParsedContract {
    pub level: u8,
    pub strain: Strain,
    pub doubling: Doubling,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Doubling {
    Undoubled,
    Doubled,
    Redoubled,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedAuction {
    pub first: Direction,
    pub calls: Vec<AuctionCall>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuctionCall {
    Bid { level: u8, strain: Strain },
    Pass,
    Double,
    Redouble,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParsedPlay {
    Standard {
        first_column: Direction,
        rows: Vec<PlayRow>,
    },
    Legacy {
        opening_leader: Option<Direction>,
        cards: Vec<Card>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayRow {
    pub cards: [Option<Card>; 4],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ActiveSection {
    None,
    Auction,
    StandardPlay,
}

/// Parse one supported `PBN` record without selecting an operation.
pub fn parse_record(input: &str) -> Result<ParsedRecord, Error> {
    let mut record = ParsedRecord::default();
    let mut section = ActiveSection::None;

    for raw_line in input.lines() {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        reject_unsupported_text(line)?;

        if line.starts_with('[') {
            let (tag, value) = parse_tag_line(line)?;
            section = ActiveSection::None;
            match tag {
                "Deal" => set_once(&mut record.deal, "Deal", parse_deal_tag(value)?)?,
                "Dealer" => set_once(
                    &mut record.dealer,
                    "Dealer",
                    parse_direction_tag("Dealer", value)?,
                )?,
                "Vulnerable" => set_once(
                    &mut record.vulnerable,
                    "Vulnerable",
                    parse_vulnerable_tag(value)?,
                )?,
                "Position" => {
                    set_once(&mut record.position, "Position", parse_position_tag(value)?)?
                }
                "First" => set_once(
                    &mut record.first,
                    "First",
                    parse_direction_tag("First", value)?,
                )?,
                "Trump" => set_once(&mut record.trump, "Trump", parse_trump_tag(value)?)?,
                "CurrentTrick" => set_once(
                    &mut record.current_trick,
                    "CurrentTrick",
                    parse_current_trick_tag(value)?,
                )?,
                "Contract" => {
                    set_once(&mut record.contract, "Contract", parse_contract_tag(value)?)?
                }
                "Declarer" => set_once(
                    &mut record.declarer,
                    "Declarer",
                    parse_direction_tag("Declarer", value)?,
                )?,
                "Auction" => {
                    set_once(
                        &mut record.auction,
                        "Auction",
                        ParsedAuction {
                            first: parse_direction_tag("Auction", value)?,
                            calls: vec![],
                        },
                    )?;
                    section = ActiveSection::Auction;
                }
                "Play" => {
                    if record.play.is_some() {
                        return Err(Error::DuplicatePbnTag("Play"));
                    }
                    if let Some(first_column) = parse_exact_direction(value) {
                        record.play = Some(ParsedPlay::Standard {
                            first_column,
                            rows: vec![],
                        });
                        section = ActiveSection::StandardPlay;
                    } else {
                        record.play = Some(parse_legacy_play(value)?);
                    }
                }
                "Claim" => {
                    return Err(Error::UnsupportedPbnFeature(
                        "Claim records are not supported".into(),
                    ));
                }
                _ => {}
            }
            continue;
        }

        match section {
            ActiveSection::Auction => {
                let auction = record.auction.as_mut().expect("active Auction section");
                for token in line.split_whitespace() {
                    auction.calls.push(parse_auction_call(token)?);
                }
            }
            ActiveSection::StandardPlay => {
                let ParsedPlay::Standard { rows, .. } =
                    record.play.as_mut().expect("active Play section")
                else {
                    unreachable!("active standard Play section has legacy value")
                };
                rows.push(parse_play_row(line)?);
            }
            ActiveSection::None => {
                return Err(Error::PbnParse(format!(
                    "section data without a supported section header: {}",
                    line
                )));
            }
        }
    }

    if let Some(ParsedPlay::Standard { rows, .. }) = &record.play {
        validate_standard_rows(rows)?;
    }

    Ok(record)
}

fn set_once<T>(slot: &mut Option<T>, tag: &'static str, value: T) -> Result<(), Error> {
    if slot.is_some() {
        return Err(Error::DuplicatePbnTag(tag));
    }
    *slot = Some(value);
    Ok(())
}

fn reject_unsupported_text(line: &str) -> Result<(), Error> {
    if line.starts_with(';') || line.starts_with('%') || line.contains('{') || line.contains('}') {
        return Err(Error::UnsupportedPbnFeature(format!(
            "comments are not supported: {}",
            line
        )));
    }
    Ok(())
}

fn parse_tag_line(line: &str) -> Result<(&str, &str), Error> {
    if !line.starts_with('[') || !line.ends_with(']') {
        return Err(Error::PbnParse(format!("invalid tag line: {}", line)));
    }
    let inner = &line[1..line.len() - 1];
    let separator = inner
        .find(char::is_whitespace)
        .ok_or_else(|| Error::PbnParse(format!("missing tag value: {}", line)))?;
    let tag = &inner[..separator];
    if tag.is_empty() || !tag.chars().all(|ch| ch.is_ascii_alphanumeric()) {
        return Err(Error::PbnParse(format!("invalid tag name: {}", tag)));
    }

    let raw_value = inner[separator..].trim();
    if raw_value.len() < 2 || !raw_value.starts_with('"') || !raw_value.ends_with('"') {
        return Err(Error::PbnParse(format!(
            "tag value must be quoted: {}",
            line
        )));
    }
    let value = &raw_value[1..raw_value.len() - 1];
    if value.contains('"') || value.contains('\\') {
        return Err(Error::UnsupportedPbnFeature(format!(
            "escaped or embedded tag values are not supported: {}",
            line
        )));
    }
    if value == "#" {
        return Err(Error::UnsupportedPbnFeature(
            "tag-value inheritance is not supported".into(),
        ));
    }
    Ok((tag, value))
}

fn parse_exact_direction(value: &str) -> Option<Direction> {
    match value {
        "N" => Some(Direction::North),
        "E" => Some(Direction::East),
        "S" => Some(Direction::South),
        "W" => Some(Direction::West),
        _ => None,
    }
}

fn parse_direction_tag(tag: &'static str, value: &str) -> Result<Direction, Error> {
    parse_exact_direction(value).ok_or_else(|| Error::InvalidPbnTag {
        tag,
        value: value.to_string(),
    })
}

fn parse_exact_strain(value: &str) -> Option<Strain> {
    match value {
        "S" => Some(Strain::Spades),
        "H" => Some(Strain::Hearts),
        "D" => Some(Strain::Diamonds),
        "C" => Some(Strain::Clubs),
        "NT" => Some(Strain::NoTrump),
        _ => None,
    }
}

fn parse_card(value: &str) -> Result<Card, Error> {
    let bytes = value.as_bytes();
    if bytes.len() != 2 {
        return Err(Error::PbnParse(format!(
            "invalid card '{}'; expected format SA",
            value
        )));
    }
    let suit = match bytes[0] {
        b'S' => Suit::Spades,
        b'H' => Suit::Hearts,
        b'D' => Suit::Diamonds,
        b'C' => Suit::Clubs,
        _ => {
            return Err(Error::PbnParse(format!(
                "invalid card '{}'; invalid suit",
                value
            )))
        }
    };
    let rank = Rank::from_char(bytes[1] as char)
        .filter(|_| (bytes[1] as char).is_ascii_uppercase() || bytes[1].is_ascii_digit())
        .ok_or_else(|| Error::PbnParse(format!("invalid card '{}'; invalid rank", value)))?;
    Ok(Card::new(suit, rank))
}

fn parse_deal_tag(value: &str) -> Result<Deal, Error> {
    let (first, hand_strs) = parse_hands_prefix(value, "Deal")?;
    for (index, hand) in hand_strs.iter().enumerate() {
        if *hand == "-" {
            return Err(Error::InvalidDeal(format!(
                "hand {} is incomplete ('-'); partial deals not supported",
                index + 1
            )));
        }
    }

    let hands = parse_hands(first, &hand_strs, Error::InvalidDeal)?;
    Deal::try_new(first, hands)
}

fn parse_position_tag(value: &str) -> Result<Hands, Error> {
    let (first, hand_strs) = parse_hands_prefix(value, "Position")?;
    let hands = parse_hands(first, &hand_strs, Error::InvalidPosition)?;
    let counts = hands.counts();
    if counts.iter().any(|count| *count != counts[0]) {
        return Err(Error::InvalidPosition(format!(
            "Position hands must have equal card counts, got {:?}",
            counts
        )));
    }
    Ok(hands)
}

fn parse_hands_prefix<'a>(value: &'a str, tag: &str) -> Result<(Direction, Vec<&'a str>), Error> {
    let Some((prefix, hands)) = value.split_once(':') else {
        return Err(Error::PbnParse(format!(
            "missing ':' in {} tag: {}",
            tag, value
        )));
    };
    let first = parse_exact_direction(prefix)
        .ok_or_else(|| Error::PbnParse(format!("{} first hand must be one of N, E, S, W", tag)))?;
    let hand_strs: Vec<_> = hands.split_whitespace().collect();
    if hand_strs.len() != 4 {
        return Err(Error::PbnParse(format!(
            "expected 4 hands in {}, got {}",
            tag,
            hand_strs.len()
        )));
    }
    Ok((first, hand_strs))
}

fn parse_hands(
    first: Direction,
    hand_strs: &[&str],
    map_error: fn(String) -> Error,
) -> Result<Hands, Error> {
    let mut hands = [Hand::empty(); 4];
    for (index, hand_str) in hand_strs.iter().enumerate() {
        let cards = parse_hand_cards(hand_str).map_err(map_error)?;
        hands[(first.dds_index() + index) % 4] =
            Hand::from_cards(&cards).map_err(|error| map_error(error.to_string()))?;
    }
    Hands::try_new(hands).map_err(|error| map_error(error.to_string()))
}

fn parse_hand_cards(hand: &str) -> Result<Vec<Card>, String> {
    let suits = hand.split('.').collect::<Vec<_>>();
    if suits.len() != 4 {
        return Err(format!(
            "hand '{}' does not have 4 suit fields separated by '.'",
            hand
        ));
    }
    let mut cards = Vec::with_capacity(13);
    for (suit_index, ranks) in suits.iter().enumerate() {
        for rank_char in ranks.chars() {
            if !rank_char.is_ascii_uppercase() && !rank_char.is_ascii_digit() {
                return Err(format!("invalid rank '{}' in hand '{}'", rank_char, hand));
            }
            let rank = Rank::from_char(rank_char)
                .ok_or_else(|| format!("invalid rank '{}' in hand '{}'", rank_char, hand))?;
            cards.push(Card::new(Suit::all()[suit_index], rank));
        }
    }
    Ok(cards)
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

fn parse_trump_tag(value: &str) -> Result<Strain, Error> {
    parse_exact_strain(value).ok_or_else(|| Error::InvalidPbnTag {
        tag: "Trump",
        value: value.to_string(),
    })
}

fn parse_current_trick_tag(value: &str) -> Result<ParsedCurrentTrick, Error> {
    if value.is_empty() {
        return Ok(ParsedCurrentTrick { cards: vec![] });
    }
    let entries: Vec<_> = value.split_whitespace().collect();
    if entries.len() > 3 {
        return Err(Error::InvalidPosition(format!(
            "CurrentTrick has {} cards, max 3",
            entries.len()
        )));
    }

    let mut cards = Vec::with_capacity(entries.len());
    let mut seen = HashSet::new();
    let mut leader = None;
    for (index, entry) in entries.iter().enumerate() {
        let Some((player_value, card_value)) = entry.split_once(':') else {
            return Err(Error::PbnParse(format!(
                "invalid CurrentTrick entry '{}'; expected format N:SA",
                entry
            )));
        };
        if card_value.contains(':') {
            return Err(Error::PbnParse(format!(
                "invalid CurrentTrick entry '{}'; expected format N:SA",
                entry
            )));
        }
        let player = parse_exact_direction(player_value).ok_or_else(|| {
            Error::PbnParse(format!(
                "invalid direction in CurrentTrick: {}",
                player_value
            ))
        })?;
        let card = parse_card(card_value)?;
        let first = *leader.get_or_insert(player);
        let expected = first.advance(index);
        if player != expected {
            return Err(Error::InvalidPosition(format!(
                "CurrentTrick: expected {} as player {}, got {}",
                expected.as_char(),
                index + 1,
                player.as_char()
            )));
        }
        if !seen.insert(card) {
            return Err(Error::InvalidPosition(format!(
                "CurrentTrick contains duplicate card {}",
                card.to_pbn()
            )));
        }
        cards.push(DirectedCard { player, card });
    }
    Ok(ParsedCurrentTrick { cards })
}

fn parse_contract_tag(value: &str) -> Result<ParsedContract, Error> {
    let bytes = value.as_bytes();
    if bytes.is_empty() || !(b'1'..=b'7').contains(&bytes[0]) {
        return Err(invalid_tag("Contract", value));
    }
    let level = bytes[0] - b'0';
    let mut denomination = &value[1..];
    let doubling = if let Some(prefix) = denomination.strip_suffix("XX") {
        denomination = prefix;
        Doubling::Redoubled
    } else if let Some(prefix) = denomination.strip_suffix('X') {
        denomination = prefix;
        Doubling::Doubled
    } else {
        Doubling::Undoubled
    };
    let strain = parse_exact_strain(denomination).ok_or_else(|| invalid_tag("Contract", value))?;
    Ok(ParsedContract {
        level,
        strain,
        doubling,
    })
}

fn parse_auction_call(token: &str) -> Result<AuctionCall, Error> {
    match token {
        "Pass" => return Ok(AuctionCall::Pass),
        "X" => return Ok(AuctionCall::Double),
        "XX" => return Ok(AuctionCall::Redouble),
        _ => {}
    }
    let bytes = token.as_bytes();
    if bytes.is_empty() || !(b'1'..=b'7').contains(&bytes[0]) {
        return Err(Error::PbnParse(format!("invalid Auction call: {}", token)));
    }
    let strain = parse_exact_strain(&token[1..])
        .ok_or_else(|| Error::PbnParse(format!("invalid Auction call: {}", token)))?;
    Ok(AuctionCall::Bid {
        level: bytes[0] - b'0',
        strain,
    })
}

fn invalid_tag(tag: &'static str, value: &str) -> Error {
    Error::InvalidPbnTag {
        tag,
        value: value.to_string(),
    }
}

fn parse_legacy_play(value: &str) -> Result<ParsedPlay, Error> {
    let (opening_leader, cards_value) = if value.len() >= 2 && value.as_bytes()[1] == b':' {
        let leader = parse_exact_direction(&value[..1])
            .ok_or_else(|| Error::PbnParse(format!("invalid legacy Play prefix: {}", value)))?;
        (Some(leader), &value[2..])
    } else {
        (None, value)
    };

    let mut cards = Vec::new();
    for group in cards_value.split_whitespace() {
        for token in group.split('=') {
            if token.is_empty() {
                return Err(Error::PbnParse(format!(
                    "empty card token in legacy Play: {}",
                    value
                )));
            }
            cards.push(parse_card(token)?);
        }
    }
    if !cards_value.is_empty() && cards_value.split_whitespace().next().is_none() {
        return Err(Error::PbnParse(
            "legacy Play contains only whitespace".into(),
        ));
    }
    Ok(ParsedPlay::Legacy {
        opening_leader,
        cards,
    })
}

fn parse_play_row(line: &str) -> Result<PlayRow, Error> {
    let tokens: Vec<_> = line.split_whitespace().collect();
    if tokens.len() != 4 {
        return Err(Error::PbnParse(format!(
            "standard Play row must contain 4 tokens, got {}: {}",
            tokens.len(),
            line
        )));
    }
    let mut cards = [None; 4];
    for (index, token) in tokens.iter().enumerate() {
        if *token != "-" {
            cards[index] = Some(parse_card(token)?);
        }
    }
    Ok(PlayRow { cards })
}

fn validate_standard_rows(rows: &[PlayRow]) -> Result<(), Error> {
    for (index, row) in rows.iter().enumerate() {
        let card_count = row.cards.iter().flatten().count();
        if card_count == 0 {
            return Err(Error::PbnParse(format!(
                "standard Play row {} cannot contain four placeholders",
                index + 1
            )));
        }
        if card_count < 4 && index + 1 != rows.len() {
            return Err(Error::PbnParse(format!(
                "incomplete standard Play row {} must be final",
                index + 1
            )));
        }
    }
    Ok(())
}

/// Serialize a `Deal` into the format expected by `DDS`.
pub fn deal_to_dds_pbn(deal: &Deal) -> String {
    let first = deal.first();
    let parts: Vec<_> = (0..4)
        .map(|offset| hand_to_pbn(deal.hands().get(first.advance(offset))))
        .collect();
    format!(
        "{}:{} {} {} {}",
        first.as_char(),
        parts[0],
        parts[1],
        parts[2],
        parts[3]
    )
}

fn hand_to_pbn(hand: &Hand) -> String {
    Suit::all()
        .iter()
        .map(|suit| {
            Rank::all_descending()
                .iter()
                .filter(|rank| hand.contains(Card::new(*suit, **rank)))
                .map(|rank| rank.as_char())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join(".")
}

/// Serialize four hands into the `PBN` string expected by `DDS`.
pub fn hands_to_dds_pbn(hands: &Hands, first_hand: Direction) -> String {
    let parts: Vec<_> = (0..4)
        .map(|offset| hand_to_pbn(hands.get(first_hand.advance(offset))))
        .collect();
    format!(
        "{}:{} {} {} {}",
        first_hand.as_char(),
        parts[0],
        parts[1],
        parts[2],
        parts[3]
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const DEAL: &str = "N:QJ6.K652.J85.T98 873.J97.AT764.Q4 K5.T83.KQ9.A7652 AT942.AQ4.32.KJ3";

    #[test]
    fn parses_partial_and_complete_records() {
        let input = format!(
            "[Deal \"{}\"]\n[Dealer \"N\"]\n[Vulnerable \"None\"]\n",
            DEAL
        );
        let record = parse_record(&input).unwrap();
        assert_eq!(record.dealer, Some(Direction::North));
        assert_eq!(record.vulnerable, Some(Vulnerability::None));
        assert_eq!(record.deal.as_ref().unwrap().first(), Direction::North);

        let partial = parse_record("[Trump \"NT\"]\n").unwrap();
        assert_eq!(partial.trump, Some(Strain::NoTrump));
        assert!(partial.deal.is_none());
    }

    #[test]
    fn deal_roundtrips_through_dds_pbn() {
        let deal = parse_deal_tag(DEAL).unwrap();
        assert_eq!(parse_deal_tag(&deal_to_dds_pbn(&deal)).unwrap(), deal);
    }

    #[test]
    fn parses_all_vulnerability_aliases() {
        for (value, expected) in [
            ("None", Vulnerability::None),
            ("Love", Vulnerability::None),
            ("-", Vulnerability::None),
            ("NS", Vulnerability::NS),
            ("EW", Vulnerability::EW),
            ("All", Vulnerability::Both),
            ("Both", Vulnerability::Both),
        ] {
            let record = parse_record(&format!("[Vulnerable \"{}\"]", value)).unwrap();
            assert_eq!(record.vulnerable, Some(expected));
        }
    }

    #[test]
    fn parses_every_supported_field() {
        let input = format!(
            "[Deal \"{}\"]\n\
             [Dealer \"N\"]\n\
             [Vulnerable \"Both\"]\n\
             [Position \"N:A... .A.. ..A. ...A\"]\n\
             [First \"E\"]\n\
             [Trump \"NT\"]\n\
             [CurrentTrick \"N:SA E:HA\"]\n\
             [Contract \"4SXX\"]\n\
             [Declarer \"S\"]\n\
             [Auction \"N\"]\n1C Pass 1H X\n\
             [Play \"W\"]\nS2 S3 S4 S5\n",
            DEAL
        );
        let record = parse_record(&input).unwrap();
        assert!(record.deal.is_some());
        assert!(record.position.is_some());
        assert_eq!(record.current_trick.unwrap().cards.len(), 2);
        assert_eq!(record.contract.unwrap().doubling, Doubling::Redoubled);
        assert_eq!(record.auction.unwrap().calls.len(), 4);
        assert!(matches!(record.play, Some(ParsedPlay::Standard { .. })));
    }

    #[test]
    fn rejects_duplicate_supported_tags_and_malformed_lines() {
        assert!(matches!(
            parse_record("[Dealer \"N\"]\n[Dealer \"E\"]"),
            Err(Error::DuplicatePbnTag("Dealer"))
        ));
        assert!(parse_record("[Dealer N]").is_err());
        assert!(parse_record("not a tag").is_err());
        assert!(parse_record("[Unknown \"x\"]\nsection data").is_err());
        assert!(matches!(
            parse_record("[Auction \"N\"]\nPass\n[Auction \"E\"]"),
            Err(Error::DuplicatePbnTag("Auction"))
        ));
        assert!(matches!(
            parse_record("[Play \"N\"]\n[Play \"E:S3\"]"),
            Err(Error::DuplicatePbnTag("Play"))
        ));
    }

    #[test]
    fn ignores_unknown_tags_but_rejects_unsupported_features() {
        let record = parse_record("[Event \"Club Game\"]\n[Dealer \"N\"]").unwrap();
        assert_eq!(record.dealer, Some(Direction::North));
        assert!(parse_record("; comment").is_err());
        assert!(parse_record("[Event \"a\\\"b\"]").is_err());
        assert!(parse_record("[Event \"#\"]").is_err());
        assert!(parse_record("[Claim \"7\"]").is_err());
    }

    #[test]
    fn validates_position_and_current_trick_intrinsically() {
        assert!(parse_record("[Position \"N:A... .A.. ..A. ...\"]").is_err());
        let current = parse_record("[CurrentTrick \"E:HA S:D2 W:C3\"]")
            .unwrap()
            .current_trick
            .unwrap();
        assert_eq!(current.cards[1].player, Direction::South);

        let error = parse_record("[CurrentTrick \"E:HA N:SA\"]")
            .unwrap_err()
            .to_string();
        assert_eq!(
            error,
            "invalid position: CurrentTrick: expected S as player 2, got N"
        );
        assert!(parse_record("[CurrentTrick \"N:SA E:SA\"]").is_err());
        assert!(parse_record("[CurrentTrick \"N:SA E:HA S:DA W:CA\"]").is_err());
    }

    #[test]
    fn parses_contract_and_auction_subsets() {
        let record = parse_record("[Contract \"3NTX\"]\n[Auction \"E\"]\n1C Pass 1NT XX").unwrap();
        let contract = record.contract.unwrap();
        assert_eq!(contract.level, 3);
        assert_eq!(contract.strain, Strain::NoTrump);
        assert_eq!(contract.doubling, Doubling::Doubled);
        assert_eq!(record.auction.unwrap().calls.len(), 4);

        for invalid in ["Pass", "0S", "8S", "4N", "4Sx", "4SXXX"] {
            assert!(parse_record(&format!("[Contract \"{}\"]", invalid)).is_err());
        }
        assert!(parse_record("[Auction \"N\"]\n1C P").is_err());
    }

    #[test]
    fn discriminates_and_parses_standard_play() {
        let record = parse_record("[Play \"W\"]\nS6 S4 SJ SQ\n- H2 H3 -").unwrap();
        let Some(ParsedPlay::Standard { first_column, rows }) = record.play else {
            panic!("expected standard Play")
        };
        assert_eq!(first_column, Direction::West);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[1].cards[0], None);

        let empty = parse_record("[Play \"N\"]").unwrap();
        assert!(matches!(
            empty.play,
            Some(ParsedPlay::Standard { rows, .. }) if rows.is_empty()
        ));
    }

    #[test]
    fn rejects_invalid_standard_play_rows() {
        for input in [
            "[Play \"N\"]\nSA SK SQ",
            "[Play \"N\"]\n- - - -",
            "[Play \"N\"]\nSA - - -\nS2 S3 S4 S5",
            "[Play \"N\"]\nSA sk SQ SJ",
        ] {
            assert!(parse_record(input).is_err(), "accepted {input}");
        }
    }

    #[test]
    fn parses_legacy_play_compatibility_forms() {
        for (value, leader, count) in [
            ("", None, 0),
            ("E:", Some(Direction::East), 0),
            ("S3=S5", None, 2),
            ("E:S3=S5=S2=SQ=H3", Some(Direction::East), 5),
            ("E:S3 S5=S2 SQ", Some(Direction::East), 4),
        ] {
            let record = parse_record(&format!("[Play \"{}\"]", value)).unwrap();
            assert!(matches!(
                record.play,
                Some(ParsedPlay::Legacy { opening_leader, cards })
                    if opening_leader == leader && cards.len() == count
            ));
        }
    }

    #[test]
    fn rejects_malformed_legacy_play_and_section_data() {
        for input in [
            "[Play \"E:S3==S5\"]",
            "[Play \"E:=S3\"]",
            "[Play \"E:S3=\"]",
            "[Play \"E:S3=-\"]",
            "[Play \"s3\"]",
            "[Play \"N:S3\"]\nS5 S2 SQ H3",
            "[Play \"E:S3\"]\nS5 S2 SQ H3",
        ] {
            assert!(parse_record(input).is_err(), "accepted {input}");
        }
    }
}
