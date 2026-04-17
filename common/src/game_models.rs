use crate::cards::Card;
use serde::{Deserialize, Serialize};

/// The three supported poker variants.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum GameMode {
    FiveCardDraw,
    TexasHoldem,
    SevenCardStud,
}

impl GameMode {
    pub fn label(&self) -> &str {
        match self {
            GameMode::FiveCardDraw => "Five Card Draw",
            GameMode::TexasHoldem => "Texas Hold'em",
            GameMode::SevenCardStud => "Seven Card Stud",
        }
    }
}

/// All phases across all three game variants.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Phase {
    // Shared
    Ante,
    Showdown,
    RoundOver,
    GameOver,
    DealerChoice,

    // Five-Card Draw
    FirstBetting,
    Draw,
    SecondBetting,

    // Texas Hold'em
    SmallBlind,
    BigBlind,
    PreFlop,
    Flop,
    Turn,
    River,

    // Seven-Card Stud
    ThirdStreet,
    FourthStreet,
    FifthStreet,
    SixthStreet,
    SeventhStreet,
}

impl Phase {
    pub fn label(&self) -> &str {
        match self {
            Phase::Ante          => "Ante",
            Phase::Showdown      => "Showdown",
            Phase::RoundOver     => "Round Over",
            Phase::GameOver      => "Game Over",
            Phase::DealerChoice  => "Dealer's Choice",
            // Five-Card Draw
            Phase::FirstBetting  => "First Betting Round",
            Phase::Draw          => "Draw Phase",
            Phase::SecondBetting => "Second Betting Round",
            // Texas Hold'em
            Phase::SmallBlind    => "Small Blind",
            Phase::BigBlind      => "Big Blind",
            Phase::PreFlop       => "Pre-Flop Betting",
            Phase::Flop          => "Flop",
            Phase::Turn          => "Turn",
            Phase::River         => "River",
            // Seven-Card Stud
            Phase::ThirdStreet   => "Third Street",
            Phase::FourthStreet  => "Fourth Street",
            Phase::FifthStreet   => "Fifth Street",
            Phase::SixthStreet   => "Sixth Street",
            Phase::SeventhStreet => "Seventh Street",
        }
    }
}

/// Per-player state sent to all clients each broadcast.
/// `cards` is the player's private hand (only sent to the owning player, except at showdown).
/// `cards_up` holds face-up cards visible to all (used in Seven-Card Stud).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerMsg {
    pub name: String,
    pub balance: i32,
    pub bet: i32,
    pub folded: bool,
    pub left: bool,
    pub is_dealer: bool,
    pub cards: Option<Vec<Card>>,
    pub cards_up: Option<Vec<Card>>,
}

/// Full game state broadcast to all clients after every action.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameStateMsg {
    pub phase: Phase,
    pub game_mode: GameMode,
    pub pot: i32,
    pub current_bet: i32,
    pub hand_number: u32,
    pub dealer_idx: usize,
    pub active_player_idx: usize,
    pub message: String,
    pub winners: Vec<String>,
    pub players: Vec<PlayerMsg>,
    pub community_cards: Vec<Card>,
}

/// House rules
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameConfig {
    pub ante: i32,
    pub max_bet: i32,
    pub max_redraws: usize,
    pub small_blind: i32,
    pub big_blind: i32,
}

impl Default for GameConfig {
    fn default() -> Self {
        Self {
            ante: 10,
            max_bet: 100,
            max_redraws: 3,
            small_blind: 10,
            big_blind: 20,
        }
    }
}