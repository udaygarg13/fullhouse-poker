//! Poker hand ranking and evaluation.
//!
//! This module checks a 5-card hand and returns a [`HandValue`] that can be
//! compared with other hands to decide the winner (or a tie).

use crate::cards::{Card, Rank};
use std::collections::HashMap;
use std::fmt;

/// The different poker hand categories, ordered from lowest to highest.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub enum HandKind {
    HighCard = 0,
    OnePair = 1,
    TwoPair = 2,
    ThreeKind = 3,
    Straight = 4,
    Flush = 5,
    FullHouse = 6,
    FourKind = 7,
    StraightFlush = 8,
    RoyalFlush = 9,
}
/// The evaluated value of a 5-card hand.
///
/// This is what we compare to determine which hand wins.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct HandValue {
    pub kind: HandKind,
    /// Primary ranking value for tie-breaking (first level only)
    /// For pairs/trips/quads: the rank of the matched cards
    /// For straights: high card of the straight
    /// For high card/flush: highest card
    pub primary: Rank,
    /// Secondary ranking value (for two pair's second pair)
    pub secondary: Option<Rank>,
}

impl Ord for HandValue {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Compare hand kind first, then primary tie breaker,
        // If all are equal, it's a split pot
        self.kind
            .cmp(&other.kind)
            .then_with(|| self.primary.cmp(&other.primary))
            .then_with(|| self.secondary.cmp(&other.secondary))
    }
}

impl PartialOrd for HandValue {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl fmt::Display for HandValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.kind {
            HandKind::RoyalFlush => write!(f, "Royal Flush"),
            HandKind::StraightFlush => write!(f, "Straight Flush ({:?}-high)", self.primary),
            HandKind::FourKind => write!(f, "Four of a Kind ({:?}s)", self.primary),
            HandKind::FullHouse => write!(f, "Full House ({:?}s full)", self.primary),
            HandKind::Flush => write!(f, "Flush ({:?}-high)", self.primary),
            HandKind::Straight => write!(f, "Straight ({:?}-high)", self.primary),
            HandKind::ThreeKind => write!(f, "Three of a Kind ({:?}s)", self.primary),
            HandKind::TwoPair => {
                if let Some(second) = self.secondary {
                    write!(f, "Two Pair ({:?}s and {:?}s)", self.primary, second)
                } else {
                    write!(f, "Two Pair ({:?}s)", self.primary)
                }
            }
            HandKind::OnePair => write!(f, "Pair of {:?}s", self.primary),
            HandKind::HighCard => write!(f, "{:?}-high", self.primary),
        }
    }
}

/// Struct that counts repeated ranks in a hand, such as pairs, trips, quads.
struct RankMultiples {
    quads: Vec<Rank>,
    trips: Vec<Rank>,
    pairs: Vec<Rank>,
}

impl RankMultiples {
    fn from_hand(hand: &[Card]) -> Self {
        // Builds multiple lists from a 5 card hand, such as pairs trips quads.
        let mut multiples: HashMap<Rank, usize> = HashMap::new();
        for card in hand {
            *multiples.entry(card.rank).or_insert(0) += 1;
        }

        let mut quads = Vec::new();
        let mut trips = Vec::new();
        let mut pairs = Vec::new();

        for (&rank, &count) in multiples.iter() {
            match count {
                4 => quads.push(rank),
                3 => trips.push(rank),
                2 => pairs.push(rank),
                _ => {}
            }
        }

        quads.sort_by(|a, b| b.cmp(a));
        trips.sort_by(|a, b| b.cmp(a));
        pairs.sort_by(|a, b| b.cmp(a));

        Self {
            quads,
            trips,
            pairs,
        }
    }
}

/// Checks if the hand is a straight.
///
/// # Returns
/// `Some(high_card)` if the hand is a straight, otherwise `None`.
fn check_straight(hand: &[Card]) -> Option<Rank> {
    let mut ranks: Vec<Rank> = hand.iter().map(|c| c.rank).collect();
    ranks.sort();
    ranks.dedup();

    if ranks.len() != 5 {
        return None;
    }

    // Check A-2-3-4-5 wheel straight
    if ranks == vec![Rank::Two, Rank::Three, Rank::Four, Rank::Five, Rank::Ace] {
        return Some(Rank::Five);
    }

    // Check if the ranks are consecutive in a simple for loop
    for i in 0..ranks.len() - 1 {
        let current = ranks[i] as u8;
        let next = ranks[i + 1] as u8;
        if next != current + 1 {
            return None;
        }
    }

    // If we get here, it's a straight
    Some(ranks[ranks.len() - 1])
}

/// Checks if all cards have the same suit (Flush)
fn check_flush(hand: &[Card]) -> bool {
    if hand.is_empty() {
        return false;
    }
    let first_suit = hand[0].suit;
    hand.iter().all(|c| c.suit == first_suit)
}

/// Returns the highest rank in the hand.
fn get_highest_rank(hand: &[Card]) -> Rank {
    hand.iter()
        .map(|c| c.rank)
        .max()
        .expect("Hand should not be empty")
}

pub fn evaluate(hand: &[Card]) -> HandValue {
    // Evaluates a 5-card poker hand and returns its ranking.
    //
    // # Arguments
    //
    // * `hand` - A slice of exactly 5 cards.
    //
    // # Returns
    //
    // A [`HandValue`] that can be compared with other evaluated hands.

    assert_eq!(hand.len(), 5, "Hand must contain exactly 5 cards");

    let multiples = RankMultiples::from_hand(hand);
    let is_flush = check_flush(hand);
    let straight_high = check_straight(hand);

    // Royal Flush (A-high straight flush)
    if is_flush && straight_high == Some(Rank::Ace) {
        return HandValue {
            kind: HandKind::RoyalFlush,
            primary: Rank::Ace,
            secondary: None,
        };
    }

    // Straight Flush (tie-breaker: high card of straight)
    if is_flush && straight_high.is_some() {
        return HandValue {
            kind: HandKind::StraightFlush,
            primary: straight_high.unwrap(),
            secondary: None,
        };
    }

    // Four of a Kind (tie-breaker: rank of the quad)
    if let Some(&quad_rank) = multiples.quads.first() {
        return HandValue {
            kind: HandKind::FourKind,
            primary: quad_rank,
            secondary: None,
        };
    }

    // Full House (tie-breaker: rank of the trips, then rank of the pair)
    if let (Some(&trip_rank), Some(&pair_rank)) = (multiples.trips.first(), multiples.pairs.first())
    {
        return HandValue {
            kind: HandKind::FullHouse,
            primary: trip_rank,
            secondary: Some(pair_rank),
        };
    }

    // Flush (tie-breaker: highest card)
    if is_flush {
        return HandValue {
            kind: HandKind::Flush,
            primary: get_highest_rank(hand),
            secondary: None,
        };
    }

    // Straight (tie-breaker: high card of straight)
    if let Some(high) = straight_high {
        return HandValue {
            kind: HandKind::Straight,
            primary: high,
            secondary: None,
        };
    }

    // Three of a Kind (tie-breaker: rank of the trips)
    if let Some(&trip_rank) = multiples.trips.first() {
        return HandValue {
            kind: HandKind::ThreeKind,
            primary: trip_rank,
            secondary: None,
        };
    }

    // Two Pair (tie-breaker: higher pair, then lower pair)
    if multiples.pairs.len() == 2 {
        return HandValue {
            kind: HandKind::TwoPair,
            primary: multiples.pairs[0], // High pair (already sorted high to low)
            secondary: Some(multiples.pairs[1]), // Low pair
        };
    }

    // One Pair (tie-breaker: rank of the pair)
    if let Some(&pair_rank) = multiples.pairs.first() {
        return HandValue {
            kind: HandKind::OnePair,
            primary: pair_rank,
            secondary: None,
        };
    }

    // High Card (tie-breaker: highest card)
    HandValue {
        kind: HandKind::HighCard,
        primary: get_highest_rank(hand),
        secondary: None,
    }
}

/// Unit tests for Ranking
#[cfg(test)]
mod tests {
    use super::*;
    use crate::cards::{Card, Rank, Suit};

    fn c(rank: Rank, suit: Suit) -> Card {
        Card { rank, suit }
    }

    #[test]
    // Normal straight
    fn check_straight_reg() {
        let hand = vec![
            c(Rank::Nine, Suit::Spades),
            c(Rank::Ten, Suit::Diamonds),
            c(Rank::Jack, Suit::Clubs),
            c(Rank::Queen, Suit::Hearts),
            c(Rank::King, Suit::Spades),
        ];
        assert_eq!(check_straight(&hand), Some(Rank::King));
        let x1 = evaluate(&hand);
        assert_eq!(x1.kind, HandKind::Straight);
        assert_eq!(x1.primary, Rank::King);
    }

    #[test]

    // Ace low straight
    fn check_straight_ace() {
        let hand = vec![
            c(Rank::Ace, Suit::Spades),
            c(Rank::Two, Suit::Diamonds),
            c(Rank::Three, Suit::Clubs),
            c(Rank::Four, Suit::Hearts),
            c(Rank::Five, Suit::Spades),
        ];
        assert_eq!(check_straight(&hand), Some(Rank::Five));
    }

    #[test]
    // Normal flush
    fn check_flush_yes() {
        let hand = vec![
            c(Rank::Eight, Suit::Spades),
            c(Rank::Ten, Suit::Spades),
            c(Rank::Jack, Suit::Spades),
            c(Rank::Queen, Suit::Spades),
            c(Rank::King, Suit::Spades),
        ];
        assert!(check_flush(&hand));
        let x1 = evaluate(&hand);
        assert_eq!(x1.kind, HandKind::Flush);
        assert_eq!(x1.primary, Rank::King);
    }

    #[test]
    // Error checking flush
    fn check_flush_no() {
        let hand = vec![
            c(Rank::Nine, Suit::Spades),
            c(Rank::Ten, Suit::Spades),
            c(Rank::Jack, Suit::Spades),
            c(Rank::Queen, Suit::Spades),
            c(Rank::King, Suit::Hearts),
        ];
        assert!(!check_flush(&hand));
    }

    #[test]
    // Straight Flush check
    fn evaluate_sflush() {
        let sflush = vec![
            c(Rank::Nine, Suit::Spades),
            c(Rank::Ten, Suit::Spades),
            c(Rank::Jack, Suit::Spades),
            c(Rank::Queen, Suit::Spades),
            c(Rank::King, Suit::Spades),
        ];
        let x1 = evaluate(&sflush);
        assert_eq!(x1.kind, HandKind::StraightFlush);
        assert_eq!(x1.primary, Rank::King);
    }

    #[test]
    // Royal Flush check
    fn evaluate_rflush() {
        let sflush = vec![
            c(Rank::Ace, Suit::Spades),
            c(Rank::Ten, Suit::Spades),
            c(Rank::Jack, Suit::Spades),
            c(Rank::Queen, Suit::Spades),
            c(Rank::King, Suit::Spades),
        ];
        let x1 = evaluate(&sflush);
        assert_eq!(x1.kind, HandKind::RoyalFlush);
        assert_eq!(x1.primary, Rank::Ace);
    }

    #[test]
    // Four of a kind
    fn evaluate_fourkind() {
        let hand = vec![
            c(Rank::Ten, Suit::Hearts),
            c(Rank::Ten, Suit::Clubs),
            c(Rank::Ten, Suit::Spades),
            c(Rank::Ten, Suit::Diamonds),
            c(Rank::King, Suit::Hearts),
        ];
        let x1 = evaluate(&hand);
        assert_eq!(x1.kind, HandKind::FourKind);
        assert_eq!(x1.primary, Rank::Ten);
    }

    #[test]
    // Fullhouse
    fn evaluate_fullhouse() {
        let hand = vec![
            c(Rank::Ten, Suit::Hearts),
            c(Rank::Ten, Suit::Clubs),
            c(Rank::Ten, Suit::Spades),
            c(Rank::King, Suit::Spades),
            c(Rank::King, Suit::Hearts),
        ];
        let x1 = evaluate(&hand);
        assert_eq!(x1.kind, HandKind::FullHouse);
        assert_eq!(x1.primary, Rank::Ten);
    }

    #[test]
    // Three of a kind
    fn evaluate_threekind() {
        let hand = vec![
            c(Rank::Ten, Suit::Hearts),
            c(Rank::Ten, Suit::Clubs),
            c(Rank::Ten, Suit::Spades),
            c(Rank::Queen, Suit::Spades),
            c(Rank::King, Suit::Hearts),
        ];
        let x1 = evaluate(&hand);
        assert_eq!(x1.kind, HandKind::ThreeKind);
        assert_eq!(x1.primary, Rank::Ten);
    }

    #[test]
    // Two pair
    fn evaluate_twopair() {
        let hand = vec![
            c(Rank::Ten, Suit::Hearts),
            c(Rank::Ten, Suit::Spades),
            c(Rank::Nine, Suit::Hearts),
            c(Rank::Nine, Suit::Spades),
            c(Rank::King, Suit::Hearts),
        ];
        let x1 = evaluate(&hand);
        assert_eq!(x1.kind, HandKind::TwoPair);
        assert_eq!(x1.primary, Rank::Ten);
    }

    #[test]
    // OnePair
    fn evaluate_pair() {
        let hand = vec![
            c(Rank::Ten, Suit::Hearts),
            c(Rank::Ten, Suit::Spades),
            c(Rank::Nine, Suit::Spades),
            c(Rank::Queen, Suit::Spades),
            c(Rank::King, Suit::Hearts),
        ];
        let x1 = evaluate(&hand);
        assert_eq!(x1.kind, HandKind::OnePair);
        assert_eq!(x1.primary, Rank::Ten);
    }

    #[test]
    // Highcard hand
    fn evaluate_highcard() {
        let hand = vec![
            c(Rank::Three, Suit::Spades),
            c(Rank::Two, Suit::Spades),
            c(Rank::Ten, Suit::Spades),
            c(Rank::Queen, Suit::Spades),
            c(Rank::King, Suit::Hearts),
        ];
        let x1 = evaluate(&hand);
        assert_eq!(x1.kind, HandKind::HighCard);
        assert_eq!(x1.primary, Rank::King);
    }

    // General comparison tests
    #[test]
    fn flush_ace_vs_king() {
        let hand1 = vec![
            c(Rank::Ace, Suit::Spades),
            c(Rank::Ten, Suit::Spades),
            c(Rank::Eight, Suit::Spades),
            c(Rank::Six, Suit::Spades),
            c(Rank::Four, Suit::Spades),
        ];

        let hand2 = vec![
            c(Rank::King, Suit::Hearts),
            c(Rank::Queen, Suit::Hearts),
            c(Rank::Jack, Suit::Hearts),
            c(Rank::Nine, Suit::Hearts),
            c(Rank::Seven, Suit::Hearts),
        ];

        let x1 = evaluate(&hand1);
        let x2 = evaluate(&hand2);
        assert_eq!(x1.kind, HandKind::Flush);
        assert_eq!(x2.kind, HandKind::Flush);
        assert_eq!(x1.primary, Rank::Ace);
        assert_eq!(x2.primary, Rank::King);
        assert!(x1 > x2);
    }

    #[test]
    fn royal_vs_king_straight_flush() {
        let hand1 = vec![
            c(Rank::Ace, Suit::Spades),
            c(Rank::King, Suit::Spades),
            c(Rank::Queen, Suit::Spades),
            c(Rank::Jack, Suit::Spades),
            c(Rank::Ten, Suit::Spades),
        ];

        let hand2 = vec![
            c(Rank::King, Suit::Hearts),
            c(Rank::Queen, Suit::Hearts),
            c(Rank::Jack, Suit::Hearts),
            c(Rank::Ten, Suit::Hearts),
            c(Rank::Nine, Suit::Hearts),
        ];

        let x1 = evaluate(&hand1);
        let x2 = evaluate(&hand2);
        assert_eq!(x1.kind, HandKind::RoyalFlush);
        assert_eq!(x2.kind, HandKind::StraightFlush);
        assert_eq!(x1.primary, Rank::Ace);
        assert_eq!(x2.primary, Rank::King);
        assert!(x1 > x2);
    }

    #[test]
    fn flush_vs_straight() {
        let hand1 = vec![
            c(Rank::King, Suit::Spades),
            c(Rank::Jack, Suit::Spades),
            c(Rank::Nine, Suit::Spades),
            c(Rank::Seven, Suit::Spades),
            c(Rank::Five, Suit::Spades),
        ];

        let hand2 = vec![
            c(Rank::Ace, Suit::Hearts),
            c(Rank::King, Suit::Diamonds),
            c(Rank::Queen, Suit::Clubs),
            c(Rank::Jack, Suit::Spades),
            c(Rank::Ten, Suit::Hearts),
        ];

        let x1 = evaluate(&hand1);
        let x2 = evaluate(&hand2);
        assert_eq!(x1.kind, HandKind::Flush);
        assert_eq!(x2.kind, HandKind::Straight);
        assert_eq!(x1.primary, Rank::King);
        assert_eq!(x2.primary, Rank::Ace);
        assert!(x1 > x2);
    }

    #[test]
    fn fourkind_vs_fourkind() {
        let hand1 = vec![
            c(Rank::King, Suit::Hearts),
            c(Rank::King, Suit::Clubs),
            c(Rank::King, Suit::Spades),
            c(Rank::King, Suit::Diamonds),
            c(Rank::Two, Suit::Hearts),
        ];

        let hand2 = vec![
            c(Rank::Seven, Suit::Hearts),
            c(Rank::Seven, Suit::Clubs),
            c(Rank::Seven, Suit::Spades),
            c(Rank::Seven, Suit::Diamonds),
            c(Rank::Ace, Suit::Hearts),
        ];

        let x1 = evaluate(&hand1);
        let x2 = evaluate(&hand2);
        assert_eq!(x1.kind, HandKind::FourKind);
        assert_eq!(x2.kind, HandKind::FourKind);
        assert_eq!(x1.primary, Rank::King);
        assert_eq!(x2.primary, Rank::Seven);
        assert!(x1 > x2);
    }

    #[test]
    fn fullhouse_same_trips_diff_pairs() {
        let hand1 = vec![
            c(Rank::Ten, Suit::Hearts),
            c(Rank::Ten, Suit::Clubs),
            c(Rank::Ten, Suit::Spades),
            c(Rank::King, Suit::Spades),
            c(Rank::King, Suit::Hearts),
        ];

        let hand2 = vec![
            c(Rank::Ten, Suit::Diamonds),
            c(Rank::Ten, Suit::Spades),
            c(Rank::Ten, Suit::Hearts),
            c(Rank::Seven, Suit::Spades),
            c(Rank::Seven, Suit::Hearts),
        ];

        let x1 = evaluate(&hand1);
        let x2 = evaluate(&hand2);
        assert_eq!(x1.kind, HandKind::FullHouse);
        assert_eq!(x2.kind, HandKind::FullHouse);
        assert_eq!(x1.primary, Rank::Ten);
        assert_eq!(x2.primary, Rank::Ten);
        assert!(x1 > x2);
    }

    #[test]
    fn straight_flush_vs_fourkind() {
        let hand1 = vec![
            c(Rank::Six, Suit::Spades),
            c(Rank::Seven, Suit::Spades),
            c(Rank::Eight, Suit::Spades),
            c(Rank::Nine, Suit::Spades),
            c(Rank::Ten, Suit::Spades),
        ];

        let hand2 = vec![
            c(Rank::Ace, Suit::Hearts),
            c(Rank::Ace, Suit::Clubs),
            c(Rank::Ace, Suit::Spades),
            c(Rank::Ace, Suit::Diamonds),
            c(Rank::King, Suit::Hearts),
        ];

        let x1 = evaluate(&hand1);
        let x2 = evaluate(&hand2);
        assert_eq!(x1.kind, HandKind::StraightFlush);
        assert_eq!(x2.kind, HandKind::FourKind);
        assert_eq!(x1.primary, Rank::Ten);
        assert_eq!(x2.primary, Rank::Ace);
        assert!(x1 > x2);
    }

    // Tie breaker tests
    #[test]
    fn straight_threekind() {
        // straight
        let hand1 = vec![
            c(Rank::Three, Suit::Spades),
            c(Rank::Four, Suit::Spades),
            c(Rank::Five, Suit::Spades),
            c(Rank::Six, Suit::Spades),
            c(Rank::Seven, Suit::Hearts),
        ];
        // threekind
        let hand2 = vec![
            c(Rank::Three, Suit::Hearts),
            c(Rank::Three, Suit::Spades),
            c(Rank::Three, Suit::Clubs),
            c(Rank::Queen, Suit::Spades),
            c(Rank::King, Suit::Hearts),
        ];
        let x1 = evaluate(&hand1);
        let x2 = evaluate(&hand2);
        assert!(x1 > x2);
    }

    #[test]
    fn ordering_higherstraight() {
        let hand1 = vec![
            c(Rank::Three, Suit::Spades),
            c(Rank::Four, Suit::Spades),
            c(Rank::Five, Suit::Spades),
            c(Rank::Six, Suit::Spades),
            c(Rank::Seven, Suit::Hearts),
        ];

        let hand2 = vec![
            c(Rank::Eight, Suit::Spades),
            c(Rank::Four, Suit::Spades),
            c(Rank::Five, Suit::Spades),
            c(Rank::Six, Suit::Spades),
            c(Rank::Seven, Suit::Hearts),
        ];
        let x1 = evaluate(&hand1);
        let x2 = evaluate(&hand2);
        assert!(x1 < x2);
    }

    #[test]
    fn tiebreaker() {
        //should split pot
        let hand1 = vec![
            c(Rank::Three, Suit::Spades),
            c(Rank::Four, Suit::Spades),
            c(Rank::Five, Suit::Spades),
            c(Rank::Six, Suit::Spades),
            c(Rank::Seven, Suit::Hearts),
        ];

        let hand2 = vec![
            c(Rank::Three, Suit::Diamonds),
            c(Rank::Four, Suit::Diamonds),
            c(Rank::Five, Suit::Diamonds),
            c(Rank::Six, Suit::Diamonds),
            c(Rank::Seven, Suit::Clubs),
        ];
        let x1 = evaluate(&hand1);
        let x2 = evaluate(&hand2);
        assert_eq!(x1.kind, HandKind::Straight);
        assert_eq!(x2.kind, HandKind::Straight);
        assert_eq!(x1.primary, Rank::Seven);
        assert_eq!(x2.primary, Rank::Seven);

        assert_eq!(x1, x2);
    }

    #[test]
    fn tiebreakerv2() {
        //should split pot, twopair test
        let hand1 = vec![
            c(Rank::Three, Suit::Spades),
            c(Rank::Three, Suit::Hearts),
            c(Rank::Five, Suit::Spades),
            c(Rank::Five, Suit::Hearts),
            c(Rank::Seven, Suit::Hearts),
        ];

        let hand2 = vec![
            c(Rank::Three, Suit::Diamonds),
            c(Rank::Three, Suit::Clubs),
            c(Rank::Five, Suit::Diamonds),
            c(Rank::Five, Suit::Clubs),
            c(Rank::Eight, Suit::Diamonds),
        ];
        let x1 = evaluate(&hand1);
        let x2 = evaluate(&hand2);
        assert_eq!(x1.kind, HandKind::TwoPair);
        assert_eq!(x2.kind, HandKind::TwoPair);
        assert_eq!(x1.primary, Rank::Five);
        assert_eq!(x2.primary, Rank::Five);

        assert_eq!(x1, x2);
    }

    #[test]
    fn tiebreaker_twopair() {
        let hand1 = vec![
            c(Rank::Three, Suit::Spades),
            c(Rank::Three, Suit::Hearts),
            c(Rank::Five, Suit::Spades),
            c(Rank::Five, Suit::Hearts),
            c(Rank::Seven, Suit::Hearts),
        ];

        let hand2 = vec![
            c(Rank::Two, Suit::Diamonds),
            c(Rank::Two, Suit::Clubs),
            c(Rank::Five, Suit::Diamonds),
            c(Rank::Five, Suit::Clubs),
            c(Rank::Seven, Suit::Diamonds),
        ];
        let x1 = evaluate(&hand1);
        let x2 = evaluate(&hand2);
        assert_eq!(x1.kind, HandKind::TwoPair);
        assert_eq!(x2.kind, HandKind::TwoPair);
        assert_eq!(x1.primary, Rank::Five);
        assert_eq!(x2.primary, Rank::Five);

        assert!(x1 > x2);
    }
}
