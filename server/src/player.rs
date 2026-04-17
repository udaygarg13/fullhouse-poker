//! Player data model.
//!
//! Defines [`Player`], which stores the basic info we track for a user
//! (name, balance, and a few stats).

#[derive(Debug, Clone)]
pub struct Player {
    pub name: String,
    pub balance: i32,
    pub rounds_played: i32,
    pub pots_won: i32,
    pub folds: i32,
}

impl Player {
    /// Creates a new `Player` with the given arguments.
    ///
    /// # Arguments
    ///
    /// * `name` - The player's name.
    /// * `balance` - Starting chip balance.
    /// * `rounds_played` - Number of rounds played.
    /// * `pots_won` - Number of pots won.
    /// * `folds` - Number of times folded.
    ///
    /// # Returns
    ///
    /// A new `Player` instance
    pub fn new(name: String, balance: i32, rounds_played: i32, pots_won: i32, folds: i32) -> Self {
        Self {
            name,
            balance,
            rounds_played,
            pots_won,
            folds,
        }
    }
}
