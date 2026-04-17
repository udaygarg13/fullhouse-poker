//! Player database and login support.
//!
//! Stores players in a SQLite database, supports account creation and login,
//! and updates balances and basic player stats.
//!
//! PINs are not stored directly. We store an Argon2 hash instead.

use crate::player::Player;
use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
};
use rand::rngs::OsRng;
use rusqlite::{Connection, params};
use std::fmt;

/// Errors that might occur when working with the players database.
#[derive(Debug)]
pub enum Error {
    /// SQLite error (open/query/execute failure).
    Database(rusqlite::Error),
    /// No player exists with the given name.
    PlayerNotFound,
    /// Login failed (incorrect PIN).
    AuthFailed,
    /// Withdrawal would make the balance negative.
    InsufficientFunds,
    /// Account with this name already exists.
    PlayerExists,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::Database(e) => write!(f, "Database error: {}", e),
            Error::PlayerNotFound => write!(f, "Player not found"),
            Error::AuthFailed => write!(f, "Invalid PIN"),
            Error::InsufficientFunds => write!(f, "Insufficient funds"),
            Error::PlayerExists => write!(f, "Player name already taken"),
        }
    }
}

impl std::error::Error for Error {}

impl From<rusqlite::Error> for Error {
    fn from(err: rusqlite::Error) -> Error {
        Error::Database(err)
    }
}
/// Wrapper around a SQLite connection for storing players and stats.
pub struct PlayerDatabase {
    conn: std::sync::Mutex<rusqlite::Connection>,
}

impl PlayerDatabase {
    pub fn new(path: &str) -> Result<Self, Error> {
        // Opens (or creates) the database at `path` and sets up the required tables.
        //
        // # Arguments
        //
        // * `path` - Path to the SQLite database file.
        //
        // # Errors
        //
        // Returns an error if the database cannot be opened or if the tables cannot be created.
        let conn = Connection::open(path)?;

        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS players (
                name TEXT PRIMARY KEY,
                pin_hash TEXT NOT NULL,
                balance INTEGER NOT NULL
            );
            
            CREATE TABLE IF NOT EXISTS stats (
                player_name TEXT PRIMARY KEY,
                rounds_played INTEGER DEFAULT 0,
                pots_won INTEGER DEFAULT 0,
                folds INTEGER DEFAULT 0,
                FOREIGN KEY(player_name) REFERENCES players(name)
            );
            
            CREATE TABLE IF NOT EXISTS round_results (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                ended_at TEXT NOT NULL,
                winner TEXT NOT NULL,
                winning_hand TEXT NOT NULL,
                amount_won INTEGER NOT NULL
            );",
        )?;

        Ok(Self {
            conn: std::sync::Mutex::new(conn),
        })
    }
    /// Hashes a PIN using Argon2 and a random salt.
    fn hash_pin(pin: &str) -> Result<String, Error> {
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();

        argon2
            .hash_password(pin.as_bytes(), &salt)
            .map_err(|_| Error::AuthFailed)
            .map(|hashed| hashed.to_string())
    }
    /// Verifies a PIN against a stored Argon2 hash.
    fn authenticate_pin(pin: &str, hash: &str) -> bool {
        let parsed_hash = match PasswordHash::new(hash) {
            Ok(parsed_hash) => parsed_hash,
            Err(_) => return false,
        };

        Argon2::default()
            .verify_password(pin.as_bytes(), &parsed_hash)
            .is_ok()
    }

    /// Creates a new players account
    /// Creates a new player account and inserts default stats.
    ///
    /// # Arguments
    /// * `name` - New player name (must be unique).
    /// * `pin` - Player PIN (stored as a hash).
    /// * `initial_balance` - Starting balance.
    ///
    /// # Errors
    /// Returns [`Error::PlayerExists`] if the name is already taken.
    pub fn create_account(
        &self,
        name: String,
        pin: String,
        initial_balance: i32,
    ) -> Result<Player, Error> {
        let pin_hash = Self::hash_pin(&pin)?;

        self.conn
            .lock()
            .unwrap()
            .execute(
                "INSERT INTO players (name, pin_hash, balance) VALUES (?1, ?2, ?3)",
                params![name, pin_hash, initial_balance],
            )
            .map_err(|_| Error::PlayerExists)?;

        self.conn
            .lock()
            .unwrap()
            .execute("INSERT INTO stats (player_name) VALUES (?1)", params![name])?;

        Ok(Player::new(name, initial_balance, 0, 0, 0))
    }
    /// Logs a player in by verifying their PIN
    /// Returns the full 'player' records on success.
    pub fn login(&self, name: &str, pin: &str) -> Result<Player, Error> {
        // Logs in a player by verifying the PIN against the stored hash.
        //
        // # Errors
        // Returns [`Error::PlayerNotFound`] if the name does not exist.
        // Returns [`Error::AuthFailed`] if the PIN is incorrect.
        let pin_hash: String = self
            .conn
            .lock()
            .unwrap()
            .query_row(
                "SELECT pin_hash FROM players WHERE name = ?1",
                params![name],
                |row| row.get(0),
            )
            .map_err(|_| Error::PlayerNotFound)?;

        if !Self::authenticate_pin(pin, &pin_hash) {
            return Err(Error::AuthFailed);
        }

        self.get_player(name)
    }
    /// Loads a player's profile and stats from the database.
    pub fn get_player(&self, name: &str) -> Result<Player, Error> {
        let (player_name, balance, rounds, pots, folds) = self
            .conn
            .lock()
            .unwrap()
            .query_row(
                "SELECT p.name, p.balance, s.rounds_played, s.pots_won, s.folds 
                FROM players p 
                JOIN stats s ON p.name = s.player_name 
                WHERE p.name = ?1",
                params![name],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                    ))
                },
            )
            .map_err(|_| Error::PlayerNotFound)?;

        Ok(Player::new(player_name, balance, rounds, pots, folds))
    }
    /// Returns all players in the database, sorted by name.
    pub fn get_all_players(&self) -> Result<Vec<Player>, Error> {
        let players = self
            .conn
            .lock()
            .unwrap()
            .prepare(
                "SELECT p.name, p.balance, s.rounds_played, s.pots_won, s.folds
            FROM players p
            JOIN stats s ON p.name = s.player_name
            ORDER BY p.name",
            )?
            .query_map([], |row| {
                Ok(Player::new(
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                ))
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(players)
    }
    /// Updates a player's balance in the database.
    pub fn update_balance(&self, name: &str, updated_balance: i32) -> Result<(), Error> {
        match self.conn.lock().unwrap().execute(
            "UPDATE players SET balance = ?1 WHERE name = ?2",
            params![updated_balance, name],
        )? {
            0 => Err(Error::PlayerNotFound),
            _ => Ok(()),
        }
    }
    /// Adds `amount` to the player's balance and returns the updated balance.
    pub fn deposit(&self, name: &str, amount: i32) -> Result<i32, Error> {
        let updated_balance = self.get_player(name)?.balance + amount;
        self.update_balance(name, updated_balance)?;
        Ok(updated_balance)
    }
    /// Adds `amount` to the player's balance and returns the updated balance.
    pub fn withdraw(&self, name: &str, amount: i32) -> Result<i32, Error> {
        // Subtracts `amount` from the player's balance and returns the updated balance.
        //
        // # Errors
        // Returns [`Error::InsufficientFunds`] if the withdrawal would go below 0.
        let updated_balance = match self.get_player(name)?.balance - amount {
            balance if (balance >= 0) => balance,
            _ => return Err(Error::InsufficientFunds),
        };
        self.update_balance(name, updated_balance)?;
        Ok(updated_balance)
    }
    /// Updates a player's stats by adding given details.
    /// Each change can be positve or negative.
    /// Updates a player's stats by adding the provided deltas.
    ///
    /// # Arguments
    /// * `rounds_delta` - Amount to add to rounds played.
    /// * `pots_delta` - Amount to add to pots won.
    /// * `folds_delta` - Amount to add to folds.
    pub fn increment_player_stats(
        &self,
        name: &str,
        rounds_delta: i32,
        pots_delta: i32,
        folds_delta: i32,
    ) -> Result<(), Error> {
        match self.conn.lock().unwrap().execute(
            "UPDATE stats SET 
             rounds_played = rounds_played + ?1,
             pots_won = pots_won + ?2,
             folds = folds + ?3
             WHERE player_name = ?4",
            params![rounds_delta, pots_delta, folds_delta, name],
        )? {
            0 => Err(Error::PlayerNotFound),
            _ => Ok(()),
        }
    }

    /// Records the result of a completed round.
    pub fn insert_round_result(
        &self,
        winner: &str,
        winning_hand: &str,
        amount_won: i32,
    ) -> Result<(), Error> {
        self.conn.lock().unwrap().execute(
            "INSERT INTO round_results (ended_at, winner, winning_hand, amount_won)
         VALUES (datetime('now'), ?1, ?2, ?3)",
            params![winner, winning_hand, amount_won],
        )?;
        Ok(())
    }

    /// Returns all round results, most recent first.
    pub fn get_round_results(&self) -> Result<Vec<(String, String, String, i32)>, Error> {
        let results = self
            .conn
            .lock()
            .unwrap()
            .prepare(
                "SELECT ended_at, winner, winning_hand, amount_won
             FROM round_results
             ORDER BY id DESC",
            )?
            .query_map([], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(results)
    }
}
