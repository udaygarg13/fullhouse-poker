//! Playing card types used by the poker engine.
//!
//! Defines [`Card`], [`Rank`], and [`Suit`], plus helpers for creating and
//! shuffling a standard 52-card deck.
//!
//! Other modules use these types for dealing cards and ranking hands.

use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Suit {
    Clubs,
    Diamonds,
    Hearts,
    Spades,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Card {
    pub rank: Rank,
    pub suit: Suit,
}

impl Card {
    /// Creates a new card with the specified rank and suit.
    ///
    /// # Arguments
    ///
    /// * `rank` - The rank of the card
    /// * `suit` - The suit of the card
    ///
    /// # Returns
    ///
    /// A new `Card` instance
    pub fn new(rank: Rank, suit: Suit) -> Self {
        Self { rank, suit }
    }
}

pub struct Deck {
    cards: Vec<Card>,
}

impl Deck {
    /// Creates a new standard 52-card deck.
    ///
    /// The deck is created in a deterministic order with all cards present.
    ///
    /// # Returns
    ///
    /// A new `Deck` instance containing all 52 cards
    pub fn new() -> Self {
        let mut cards = Vec::new();

        for suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
            for rank in [
                Rank::Two,
                Rank::Three,
                Rank::Four,
                Rank::Five,
                Rank::Six,
                Rank::Seven,
                Rank::Eight,
                Rank::Nine,
                Rank::Ten,
                Rank::Jack,
                Rank::Queen,
                Rank::King,
                Rank::Ace,
            ] {
                cards.push(Card::new(rank, suit));
            }
        }

        Self { cards }
    }

    /// Shuffles the deck using the provided random number generator.
    ///
    /// # Arguments
    ///
    /// * `rng` - A mutable reference to a random number generator
    pub fn shuffle(&mut self, rng: &mut impl rand::Rng) {
        self.cards.shuffle(rng);
    }

    /// Deals a card from the top of the deck.
    ///
    /// # Returns
    ///
    /// `Some(Card)` if cards remain in the deck, `None` if the deck is empty
    pub fn deal(&mut self) -> Option<Card> {
        self.cards.pop()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::thread_rng;

    #[test]
    fn test_deck() {
        let mut deck = Deck::new();
        assert_eq!(deck.cards.len(), 52);

        let mut rng = thread_rng();
        deck.shuffle(&mut rng);

        let hand: Vec<Card> = {
            let this = &mut deck;
            (0..5).filter_map(|_| this.deal()).collect()
        };
        assert_eq!(hand.len(), 5);
        assert_eq!(deck.cards.len(), 47);
    }
}
