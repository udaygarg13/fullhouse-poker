use crate::db::PlayerDatabase;
use crate::player::Player;
use common::cards::{Card, Deck, Rank};
use common::game_models::{GameConfig, GameMode, GameStateMsg, Phase, PlayerMsg};
use common::ranking;
use rand::thread_rng;
use std::net::SocketAddr;
use std::sync::Mutex;
use std::sync::{Arc, OnceLock};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

static VIEWER_SENDERS: OnceLock<tokio::sync::Mutex<Vec<UnboundedSender<String>>>> = OnceLock::new();
static LAST_VIEWER_STATE: OnceLock<Mutex<Option<String>>> = OnceLock::new();

pub fn viewer_senders() -> &'static tokio::sync::Mutex<Vec<UnboundedSender<String>>> {
    VIEWER_SENDERS.get_or_init(|| tokio::sync::Mutex::new(Vec::new()))
}

pub fn last_viewer_state() -> &'static Mutex<Option<String>> {
    LAST_VIEWER_STATE.get_or_init(|| Mutex::new(None))
}

pub async fn add_viewer_sender(tx: UnboundedSender<String>) {
    let mut viewers = viewer_senders().lock().await;
    if !viewers.iter().any(|v| v.same_channel(&tx)) {
        viewers.push(tx);
    }
}

pub async fn send_cached_viewer_state(tx: &UnboundedSender<String>) {
    if let Ok(guard) = last_viewer_state().lock() {
        if let Some(msg) = guard.clone() {
            let _ = tx.send(msg);
        }
    }
}

pub async fn remove_viewer_sender(tx: &UnboundedSender<String>) {
    viewer_senders()
        .lock()
        .await
        .retain(|v| !v.same_channel(tx));
}

pub async fn clear_viewers() {
    viewer_senders().lock().await.clear();
    if let Ok(mut guard) = last_viewer_state().lock() {
        *guard = None;
    }
}

static FINAL_PLAYER_SENDERS: OnceLock<tokio::sync::Mutex<Vec<UnboundedSender<String>>>> =
    OnceLock::new();

fn final_player_senders() -> &'static tokio::sync::Mutex<Vec<UnboundedSender<String>>> {
    FINAL_PLAYER_SENDERS.get_or_init(|| tokio::sync::Mutex::new(Vec::new()))
}

pub async fn set_final_player_senders(txs: Vec<UnboundedSender<String>>) {
    *final_player_senders().lock().await = txs;
}

pub async fn take_final_player_senders() -> Vec<UnboundedSender<String>> {
    std::mem::take(&mut *final_player_senders().lock().await)
}

#[derive(Debug, Clone)]
pub struct GamePlayer {
    pub name: String,
    pub balance: i32,
    pub is_active: bool,
    pub hand: Vec<Card>,
    pub cards_up: Vec<Card>,
    pub redraws_used: usize,
    pub folded_this_hand: bool,
    pub left_mid_hand: bool,
}

impl GamePlayer {
    pub fn new(name: String, balance: i32) -> Self {
        Self {
            name,
            balance,
            is_active: true,
            hand: Vec::new(),
            cards_up: Vec::new(),
            redraws_used: 0,
            folded_this_hand: false,
            left_mid_hand: false,
        }
    }

    pub fn from_db(db_player: &Player) -> Self {
        Self::new(db_player.name.clone(), db_player.balance)
    }

    pub fn credit(&mut self, amount: i32) {
        self.balance += amount;
    }

    pub fn deduct(&mut self, amount: i32) -> bool {
        if self.balance >= amount {
            self.balance -= amount;
            true
        } else {
            false
        }
    }

    pub fn reset_for_new_hand(&mut self) {
        self.is_active = true;
        self.hand.clear();
        self.cards_up.clear();
        self.redraws_used = 0;
        self.folded_this_hand = false;
        self.left_mid_hand = false;
    }

    pub fn deal_cards(&mut self, deck: &mut Deck, count: usize) {
        for _ in 0..count {
            if let Some(card) = deck.deal() {
                self.hand.push(card);
                self.hand.sort_by(|a, b| b.cmp(a));
            }
        }
    }

    pub fn deal_card_down(&mut self, deck: &mut Deck) {
        if let Some(card) = deck.deal() {
            self.hand.push(card);
        }
    }

    pub fn deal_card_up(&mut self, deck: &mut Deck) {
        if let Some(card) = deck.deal() {
            self.hand.push(card);
            self.cards_up.push(card);
        }
    }

    pub fn discard_and_draw(&mut self, indices: Vec<usize>, deck: &mut Deck) {
        let mut sorted = indices.clone();
        sorted.sort_by(|a, b| b.cmp(a));
        let mut removed = 0;
        for i in sorted {
            if i < self.hand.len() {
                self.hand.remove(i);
                removed += 1;
            }
        }
        self.deal_cards(deck, removed);
        self.redraws_used += 1;
    }

    pub fn can_redraw(&self, max_redraws: usize) -> bool {
        self.redraws_used < max_redraws
    }
}


#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    Bet(i32),
    Check,
    Call,
    Raise(i32),
    Fold,
    Draw(Vec<usize>),
    Continue,
    Leave,
    DealerChoice(GameMode),
}

pub fn parse_action(s: &str) -> Option<Action> {
    let s = s.trim();
    let parts: Vec<&str> = s.splitn(2, ' ').collect();
    match parts.as_slice() {
        ["FOLD"] => Some(Action::Fold),
        ["CHECK"] => Some(Action::Check),
        ["CALL"] => Some(Action::Call),
        ["CONTINUE"] => Some(Action::Continue),
        ["LEAVE"] => Some(Action::Leave),
        ["BET", v] => v.parse().ok().map(Action::Bet),
        ["RAISE", v] => v.parse().ok().map(Action::Raise),
        ["DRAW"] => Some(Action::Draw(vec![])),
        ["DRAW", v] => {
            if v.is_empty() || *v == "none" {
                Some(Action::Draw(vec![]))
            } else {
                v.split(',')
                    .map(|i| i.trim().parse().ok())
                    .collect::<Option<Vec<usize>>>()
                    .map(Action::Draw)
            }
        }
        ["MODE", variant] => match *variant {
            "FiveCardDraw" => Some(Action::DealerChoice(GameMode::FiveCardDraw)),
            "TexasHoldem" => Some(Action::DealerChoice(GameMode::TexasHoldem)),
            "SevenCardStud" => Some(Action::DealerChoice(GameMode::SevenCardStud)),
            _ => None,
        },
        _ => None,
    }
}

pub struct SessionPlayer {
    pub game: GamePlayer,
    pub addr: SocketAddr,
    pub tx: UnboundedSender<String>,
}

struct HandContext {
    pub last_msg: String,
    pub winners: Vec<String>,
    pub community_cards: Vec<Card>,
}

impl HandContext {
    fn new() -> Self {
        Self {
            last_msg: String::new(),
            winners: vec![],
            community_cards: vec![],
        }
    }
}

fn send_to(tx: &UnboundedSender<String>, msg: &str) {
    let _ = tx.send(msg.to_string());
}

fn active_count(players: &[SessionPlayer]) -> usize {
    players.iter().filter(|sp| sp.game.is_active).count()
}

fn active_indices(players: &[SessionPlayer]) -> Vec<usize> {
    (0..players.len())
        .filter(|&i| players[i].game.is_active)
        .collect()
}

fn build_state(
    players: &[SessionPlayer],
    phase: Phase,
    game_mode: GameMode,
    pot: i32,
    current_bet: i32,
    contributions: &[i32],
    dealer_idx: usize,
    active_player_idx: usize,
    hand_number: u32,
    ctx: &HandContext,
    community_cards: Vec<Card>,
) -> GameStateMsg {
    GameStateMsg {
        phase,
        game_mode,
        pot,
        current_bet,
        hand_number,
        dealer_idx,
        active_player_idx,
        message: ctx.last_msg.clone(),
        winners: ctx.winners.clone(),
        community_cards,
        players: players
            .iter()
            .enumerate()
            .map(|(i, sp)| PlayerMsg {
                name: sp.game.name.clone(),
                balance: sp.game.balance,
                bet: contributions.get(i).copied().unwrap_or(0),
                folded: sp.game.folded_this_hand,
                left: sp.game.left_mid_hand,
                is_dealer: i == dealer_idx,
                cards: Some(sp.game.hand.clone()),
                cards_up: if sp.game.cards_up.is_empty() {
                    None
                } else {
                    Some(sp.game.cards_up.clone())
                },
            })
            .collect(),
    }
}

fn broadcast_state(players: &[SessionPlayer], state: &GameStateMsg) {
    for (i, sp) in players.iter().enumerate() {
        let personalised_players: Vec<PlayerMsg> = state
            .players
            .iter()
            .enumerate()
            .map(|(j, pm)| {
                if j == i || matches!(state.phase, Phase::Showdown | Phase::RoundOver) {
                    pm.clone()
                } else {
                    PlayerMsg {
                        cards: None,
                        cards_up: pm.cards_up.clone(),
                        ..pm.clone()
                    }
                }
            })
            .collect();

        let msg = GameStateMsg {
            players: personalised_players,
            ..state.clone()
        };

        if let Ok(json) = serde_json::to_string(&msg) {
            send_to(&sp.tx, &format!("GAME_STATE {}", json));
        }
    }

    let viewer_players: Vec<PlayerMsg> = state
        .players
        .iter()
        .map(|pm| {
            if matches!(
                state.phase,
                Phase::Showdown | Phase::RoundOver | Phase::GameOver
            ) {
                pm.clone()
            } else {
                PlayerMsg {
                    cards: None,
                    cards_up: pm.cards_up.clone(),
                    ..pm.clone()
                }
            }
        })
        .collect();

    let viewer_state = GameStateMsg {
        players: viewer_players,
        ..state.clone()
    };

    if let Ok(json) = serde_json::to_string(&viewer_state) {
        let msg = format!("GAME_STATE {}", json);
        if let Ok(mut cache) = last_viewer_state().lock() {
            *cache = Some(msg.clone());
        }
        if let Ok(mut vs) = viewer_senders().try_lock() {
            vs.retain(|tx| !tx.is_closed());
            for tx in vs.iter() {
                let _ = tx.send(msg.clone());
            }
        }
    }
}

async fn wait_for_action(
    action_rx: &mut UnboundedReceiver<(String, String)>,
    active_username: &str,
    active_tx: &UnboundedSender<String>,
    valid_verbs: &[&str],
    pending_leaves: &mut std::collections::HashSet<String>,
) -> Action {
    loop {
        match action_rx.recv().await {
            None => {
                return if valid_verbs.contains(&"FOLD") {
                    Action::Fold
                } else {
                    Action::Leave
                };
            }
            Some((username, cmd)) => {
                if username != active_username {
                    if cmd.trim() == "LEAVE" {
                        pending_leaves.insert(username);
                    }
                    continue;
                }
                let verb = cmd.splitn(2, ' ').next().unwrap_or("");
                if !valid_verbs.contains(&verb) {
                    send_to(
                        active_tx,
                        &format!("ERR Valid actions here: {}", valid_verbs.join(", ")),
                    );
                    continue;
                }
                if let Some(action) = parse_action(&cmd) {
                    return action;
                }
                send_to(active_tx, "ERR Could not parse action");
            }
        }
    }
}

fn handle_early_end(
    players: &mut Vec<SessionPlayer>,
    pot: &mut i32,
    dealer_idx: usize,
    hand_number: u32,
    game_mode: GameMode,
    db: &PlayerDatabase,
    ctx: &mut HandContext,
) {
    let wi = active_indices(players);

    if let Some(&winner) = wi.first() {
        players[winner].game.balance += *pot;
        let winner_name = players[winner].game.name.clone();
        ctx.last_msg = format!("Round Over.");
        ctx.winners = vec![
            format!("{} wins ${} uncontested!", winner_name, *pot),
            winner_name,
        ];
        let state = build_state(
            players,
            Phase::RoundOver,
            game_mode,
            *pot,
            0,
            &vec![0; players.len()],
            dealer_idx,
            0,
            hand_number,
            ctx,
            vec![],
        );
        broadcast_state(players, &state);
    }

    update_stats(players, &wi, db, "Uncontested", *pot);
    *pot = 0;
}

fn update_stats(
    players: &[SessionPlayer],
    winner_indices: &[usize],
    db: &PlayerDatabase,
    winning_hand: &str,
    amount_won: i32,
) {
    for &idx in winner_indices {
        let _ = db.insert_round_result(&players[idx].game.name, winning_hand, amount_won);
    }
    for (idx, sp) in players.iter().enumerate() {
        let _ = db.increment_player_stats(
            &sp.game.name,
            1,
            if winner_indices.contains(&idx) { 1 } else { 0 },
            if sp.game.folded_this_hand { 1 } else { 0 },
        );
        let _ = db.update_balance(&sp.game.name, sp.game.balance);
    }
}

async fn run_ante_round(
    players: &mut Vec<SessionPlayer>,
    pot: &mut i32,
    contributions: &mut Vec<i32>,
    dealer_idx: usize,
    hand_number: u32,
    game_mode: GameMode,
    action_rx: &mut UnboundedReceiver<(String, String)>,
    config: &GameConfig,
    db: &PlayerDatabase,
    ctx: &mut HandContext,
) -> bool {
    let n = players.len();
    let ante = config.ante;

    if players[dealer_idx].game.balance >= ante {
        players[dealer_idx].game.balance -= ante;
        contributions[dealer_idx] += ante;
        *pot += ante;
    } else {
        players[dealer_idx].game.is_active = false;
        players[dealer_idx].game.folded_this_hand = true;
    }

    ctx.last_msg = format!("{} posts ante ${}", players[dealer_idx].game.name, ante);
    ctx.winners = vec![];
    let state = build_state(
        players,
        Phase::Ante,
        game_mode.clone(),
        *pot,
        ante,
        contributions,
        dealer_idx,
        dealer_idx,
        hand_number,
        ctx,
        vec![],
    );
    broadcast_state(players, &state);

    let mut pending_leaves: std::collections::HashSet<String> = std::collections::HashSet::new();

    for offset in 1..n {
        let turn_idx = (dealer_idx + offset) % n;

        for name in pending_leaves.drain() {
            if let Some(i) = players.iter().position(|sp| sp.game.name == name) {
                players[i].game.is_active = false;
                players[i].game.left_mid_hand = true;
                send_to(&players[i].tx, "GAME_LEFT You have left the game");
                let _ = db.update_balance(&name, players[i].game.balance);
                let _ = db.increment_player_stats(&name, 1, 0, 1);
                ctx.last_msg = format!("{} disconnected.", name);
            }
        }

        if active_count(players) <= 1 {
            return true;
        }

        if !players[turn_idx].game.is_active {
            continue;
        }

        if players[turn_idx].game.balance < ante {
            players[turn_idx].game.is_active = false;
            players[turn_idx].game.folded_this_hand = true;
            ctx.last_msg = format!(
                "{} can't afford ante and sits out",
                players[turn_idx].game.name
            );
            let state = build_state(
                players,
                Phase::Ante,
                game_mode.clone(),
                *pot,
                ante,
                contributions,
                dealer_idx,
                turn_idx,
                hand_number,
                ctx,
                vec![],
            );
            broadcast_state(players, &state);
            continue;
        }

        let active_name = players[turn_idx].game.name.clone();
        let active_tx = players[turn_idx].tx.clone();

        ctx.last_msg = format!("{} must call ante ${} or fold", active_name, ante);
        let state = build_state(
            players,
            Phase::Ante,
            game_mode.clone(),
            *pot,
            ante,
            contributions,
            dealer_idx,
            turn_idx,
            hand_number,
            ctx,
            vec![],
        );
        broadcast_state(players, &state);

        let action = wait_for_action(
            action_rx,
            &active_name,
            &active_tx,
            &["CALL", "FOLD", "LEAVE"],
            &mut pending_leaves,
        )
        .await;

        ctx.last_msg = match &action {
            Action::Call => {
                let cost = ante.min(players[turn_idx].game.balance);
                players[turn_idx].game.balance -= cost;
                contributions[turn_idx] += cost;
                *pot += cost;
                format!("{} posts ante ${}", active_name, cost)
            }
            Action::Fold => {
                players[turn_idx].game.is_active = false;
                players[turn_idx].game.folded_this_hand = true;
                format!("{} folds to ante", active_name)
            }
            Action::Leave => {
                players[turn_idx].game.is_active = false;
                players[turn_idx].game.left_mid_hand = true;
                send_to(&players[turn_idx].tx, "GAME_LEFT You have left the game");
                let _ = db.update_balance(&active_name, players[turn_idx].game.balance);
                let _ = db.increment_player_stats(&active_name, 1, 0, 1);
                format!("{} left during ante", active_name)
            }
            _ => format!("{} posts ante ${}", active_name, ante), 
        };

        let state = build_state(
            players,
            Phase::Ante,
            game_mode.clone(),
            *pot,
            ante,
            contributions,
            dealer_idx,
            turn_idx,
            hand_number,
            ctx,
            vec![],
        );
        broadcast_state(players, &state);

        if active_count(players) <= 1 {
            return true;
        }
    }

    for name in pending_leaves.drain() {
        if let Some(i) = players.iter().position(|sp| sp.game.name == name) {
            players[i].game.is_active = false;
            players[i].game.left_mid_hand = true;
            send_to(&players[i].tx, "GAME_LEFT You have left the game");
            let _ = db.update_balance(&name, players[i].game.balance);
            let _ = db.increment_player_stats(&name, 1, 0, 1);
            ctx.last_msg = format!("{} disconnected.", name);
        }
    }

    if active_count(players) <= 1 {
        return true;
    }

    false
}

async fn run_betting_round(
    players: &mut Vec<SessionPlayer>,
    pot: &mut i32,
    contributions: &mut Vec<i32>,
    start_idx: usize,
    hand_number: u32,
    dealer_idx: usize,
    phase: Phase,
    game_mode: GameMode,
    action_rx: &mut UnboundedReceiver<(String, String)>,
    config: &GameConfig,
    db: &PlayerDatabase,
    ctx: &mut HandContext,
    community_cards: Vec<Card>,
    initial_bet: i32,
) -> bool {
    let n = players.len();
    let mut current_bet = initial_bet;
    let mut turn_idx = start_idx;

    let mut has_acted = vec![false; n];
    let mut pending_leaves: std::collections::HashSet<String> = std::collections::HashSet::new();

    loop {
        for name in pending_leaves.drain() {
            if let Some(i) = players.iter().position(|sp| sp.game.name == name) {
                players[i].game.is_active = false;
                players[i].game.left_mid_hand = true;
                send_to(&players[i].tx, "GAME_LEFT You have left the game");
                let _ = db.update_balance(&name, players[i].game.balance);
                let _ = db.increment_player_stats(&name, 1, 0, 1);
                ctx.last_msg = format!("{} disconnected.", name);
            }
        }

        if active_count(players) <= 1 {
            return true;
        }

        let all_matched_and_acted = players
            .iter()
            .enumerate()
            .all(|(i, sp)| !sp.game.is_active || (contributions[i] == current_bet && has_acted[i]));

        if all_matched_and_acted {
            break;
        }

        if !players[turn_idx].game.is_active {
            turn_idx = (turn_idx + 1) % n;
            continue;
        }

        let active_name = players[turn_idx].game.name.clone();
        let active_tx = players[turn_idx].tx.clone();

        let to_call = current_bet - contributions[turn_idx];
        let can_check = to_call == 0;

        let valid: &[&str] = if can_check {
            &["CHECK", "FOLD", "BET", "LEAVE"]
        } else {
            &["CALL", "FOLD", "RAISE", "LEAVE"]
        };

        let state = build_state(
            players,
            phase.clone(),
            game_mode.clone(),
            *pot,
            current_bet,
            contributions,
            dealer_idx,
            turn_idx,
            hand_number,
            ctx,
            community_cards.clone(),
        );
        broadcast_state(players, &state);

        let action = loop {
            let a = wait_for_action(
                action_rx,
                &active_name,
                &active_tx,
                valid,
                &mut pending_leaves,
            )
            .await;
            match &a {
                Action::Bet(amt) if *amt > config.max_bet => {
                    send_to(&active_tx, &format!("ERR Max bet is ${}", config.max_bet));
                }
                Action::Raise(amt) => {
                    let new_level = current_bet + amt;
                    if new_level > config.max_bet {
                        send_to(
                            &active_tx,
                            &format!("ERR Raise would exceed max bet ${}", config.max_bet),
                        );
                    } else if players[turn_idx].game.balance < to_call + amt {
                        send_to(&active_tx, "ERR Insufficient funds");
                    } else {
                        break a;
                    }
                }
                Action::Bet(amt) if players[turn_idx].game.balance < *amt => {
                    send_to(&active_tx, "ERR Insufficient funds");
                }
                _ => break a,
            }
        };

        has_acted[turn_idx] = true;

        ctx.last_msg = match &action {
            Action::Fold => {
                players[turn_idx].game.is_active = false;
                players[turn_idx].game.folded_this_hand = true;
                format!("{} folds", active_name)
            }
            Action::Check => format!("{} checks", active_name),
            Action::Call => {
                let cost = to_call.min(players[turn_idx].game.balance);
                players[turn_idx].game.balance -= cost;
                contributions[turn_idx] += cost;
                *pot += cost;
                format!("{} calls ${}", active_name, cost)
            }
            Action::Bet(amt) => {
                players[turn_idx].game.balance -= amt;
                contributions[turn_idx] += *amt;
                *pot += amt;
                current_bet = contributions[turn_idx];
                format!("{} bets ${}", active_name, amt)
            }
            Action::Raise(amt) => {
                let cost = to_call + amt;
                players[turn_idx].game.balance -= cost;
                contributions[turn_idx] += cost;
                *pot += cost;
                current_bet = contributions[turn_idx];
                format!("{} raises ${}", active_name, amt)
            }
            Action::Leave => {
                players[turn_idx].game.is_active = false;
                players[turn_idx].game.left_mid_hand = true;
                send_to(&players[turn_idx].tx, "GAME_LEFT You have left the game");
                let _ = db.update_balance(&active_name, players[turn_idx].game.balance);
                let _ = db.increment_player_stats(&active_name, 1, 0, 1);
                let remaining = active_count(players);
                if remaining <= 1 {
                    format!(
                        "{} left. Game ended due to insufficient players.",
                        active_name
                    )
                } else {
                    format!("{} left. {} players remaining.", active_name, remaining)
                }
            }
            _ => String::new(),
        };

        let state = build_state(
            players,
            phase.clone(),
            game_mode.clone(),
            *pot,
            current_bet,
            contributions,
            dealer_idx,
            turn_idx,
            hand_number,
            ctx,
            community_cards.clone(),
        );
        broadcast_state(players, &state);

        if active_count(players) <= 1 {
            return true;
        }

        turn_idx = (turn_idx + 1) % n;
    }

    false
}

fn run_showdown(
    players: &mut Vec<SessionPlayer>,
    pot: &mut i32,
    dealer_idx: usize,
    hand_number: u32,
    game_mode: GameMode,
    ctx: &mut HandContext,
    community_cards: &[Card],
) -> (Vec<usize>, String, i32) {
    let ai = active_indices(players);
    let mut best: Option<common::ranking::HandValue> = None;
    let mut winner_indices: Vec<usize> = vec![];

    for &idx in &ai {
        let hv = match game_mode {
            GameMode::TexasHoldem => {
                best_of_n(&players[idx].game.hand, community_cards)
            }
            GameMode::SevenCardStud => {
                best_of_n(&players[idx].game.hand, &[])
            }
            GameMode::FiveCardDraw => ranking::evaluate(&players[idx].game.hand),
        };

        match &best {
            None => {
                best = Some(hv);
                winner_indices = vec![idx];
            }
            Some(b) => {
                if hv > *b {
                    best = Some(hv);
                    winner_indices = vec![idx];
                } else if hv == *b {
                    winner_indices.push(idx);
                }
            }
        }
    }

    let share = *pot / winner_indices.len().max(1) as i32;
    let remainder = *pot % winner_indices.len().max(1) as i32;
    for (i, &idx) in winner_indices.iter().enumerate() {
        let bonus = if i == 0 { remainder } else { 0 };
        players[idx].game.balance += share + bonus;
    }

    let winner_names: Vec<String> = winner_indices
        .iter()
        .map(|&i| players[i].game.name.clone())
        .collect();

    let winning_hand_name = best
        .as_ref()
        .map(|hv| hv.to_string())
        .unwrap_or_else(|| "Unknown".to_string());

    ctx.last_msg = format!("Round Over.");
    ctx.winners = winner_names.clone();
    ctx.winners.insert(
        0,
        if winner_names.len() == 1 {
            format!(
                "{} wins ${} with {}!",
                winner_names[0], share, winning_hand_name
            )
        } else {
            let names_str = match winner_names.len() {
                2 => format!("{} and {}", winner_names[0], winner_names[1]),
                _ => format!(
                    "{}, and {}",
                    winner_names[..winner_names.len() - 1].join(", "),
                    winner_names.last().unwrap()
                ),
            };
            format!(
                "{} split the pot: ${} each with {}!",
                names_str, share, winning_hand_name
            )
        },
    );

    let player_msgs: Vec<PlayerMsg> = players
        .iter()
        .enumerate()
        .map(|(i, sp)| PlayerMsg {
            name: sp.game.name.clone(),
            balance: sp.game.balance,
            bet: 0,
            folded: sp.game.folded_this_hand,
            left: sp.game.left_mid_hand,
            is_dealer: i == dealer_idx,
            cards: if sp.game.is_active {
                Some(sp.game.hand.clone())
            } else {
                None
            },
            cards_up: if sp.game.cards_up.is_empty() {
                None
            } else {
                Some(sp.game.cards_up.clone())
            },
        })
        .collect();

    let state = GameStateMsg {
        phase: Phase::Showdown,
        game_mode,
        pot: *pot,
        current_bet: 0,
        hand_number,
        dealer_idx,
        active_player_idx: 0,
        message: ctx.last_msg.clone(),
        winners: ctx.winners.clone(),
        players: player_msgs,
        community_cards: community_cards.to_vec(),
    };

    for sp in players.iter() {
        if let Ok(json) = serde_json::to_string(&state) {
            send_to(&sp.tx, &format!("GAME_STATE {}", json));
        }
    }

    let amount_won = share;
    *pot = 0;

    (winner_indices, winning_hand_name, amount_won)
}


fn best_of_n(hand: &[Card], extra: &[Card]) -> common::ranking::HandValue {
    let mut all: Vec<Card> = hand.to_vec();
    all.extend_from_slice(extra);
    let n = all.len();
    let mut best: Option<common::ranking::HandValue> = None;

    for i in 0..n {
        for j in (i + 1)..n {
            for k in (j + 1)..n {
                for l in (k + 1)..n {
                    for m in (l + 1)..n {
                        let five = vec![all[i], all[j], all[k], all[l], all[m]];
                        let hv = ranking::evaluate(&five);
                        match &best {
                            None => best = Some(hv),
                            Some(b) if hv > *b => best = Some(hv),
                            _ => {}
                        }
                    }
                }
            }
        }
    }

    best.expect("No cards to evaluate")
}


fn card_label(card: &Card) -> String {
    let rank = match card.rank {
        Rank::Two => "2",
        Rank::Three => "3",
        Rank::Four => "4",
        Rank::Five => "5",
        Rank::Six => "6",
        Rank::Seven => "7",
        Rank::Eight => "8",
        Rank::Nine => "9",
        Rank::Ten => "10",
        Rank::Jack => "J",
        Rank::Queen => "Q",
        Rank::King => "K",
        Rank::Ace => "A",
    };
    let suit = match card.suit {
        common::cards::Suit::Spades => "♠",
        common::cards::Suit::Hearts => "♥",
        common::cards::Suit::Diamonds => "♦",
        common::cards::Suit::Clubs => "♣",
    };
    format!("{}{}", rank, suit)
}


fn rank_value(rank: &Rank) -> u8 {
    match rank {
        Rank::Two => 0,
        Rank::Three => 1,
        Rank::Four => 2,
        Rank::Five => 3,
        Rank::Six => 4,
        Rank::Seven => 5,
        Rank::Eight => 6,
        Rank::Nine => 7,
        Rank::Ten => 8,
        Rank::Jack => 9,
        Rank::Queen => 10,
        Rank::King => 11,
        Rank::Ace => 12,
    }
}

fn player_with_lowest_upcard(players: &[SessionPlayer]) -> usize {
    let ai = active_indices(players);
    let mut best_idx = ai[0];
    let mut best_val = 255u8;

    for &idx in &ai {
        if let Some(card) = players[idx].game.cards_up.first() {
            let v = rank_value(&card.rank);
            if v < best_val {
                best_val = v;
                best_idx = idx;
            }
        }
    }
    best_idx
}

fn evaluate_partial(cards: &[Card]) -> common::ranking::HandValue {
    use common::ranking::{HandKind, HandValue};
    use std::collections::HashMap;

    let mut counts: HashMap<Rank, u8> = HashMap::new();
    for c in cards {
        *counts.entry(c.rank).or_insert(0) += 1;
    }

    let mut quads: Vec<Rank> = counts.iter().filter(|&(_, &v)| v >= 4).map(|(&r, _)| r).collect();
    let mut trips: Vec<Rank> = counts.iter().filter(|&(_, &v)| v == 3).map(|(&r, _)| r).collect();
    let mut pairs: Vec<Rank> = counts.iter().filter(|&(_, &v)| v == 2).map(|(&r, _)| r).collect();

    quads.sort_by(|a, b| rank_value(b).cmp(&rank_value(a)));
    trips.sort_by(|a, b| rank_value(b).cmp(&rank_value(a)));
    pairs.sort_by(|a, b| rank_value(b).cmp(&rank_value(a)));

    let high = cards.iter().max_by_key(|c| rank_value(&c.rank)).unwrap().rank;

    if let Some(&q) = quads.first() {
        return HandValue { kind: HandKind::FourKind, primary: q, secondary: None };
    }
    if let (Some(&t), Some(&p)) = (trips.first(), pairs.first()) {
        return HandValue { kind: HandKind::FullHouse, primary: t, secondary: Some(p) };
    }
    if let Some(&t) = trips.first() {
        return HandValue { kind: HandKind::ThreeKind, primary: t, secondary: None };
    }
    if pairs.len() >= 2 {
        return HandValue { kind: HandKind::TwoPair, primary: pairs[0], secondary: Some(pairs[1]) };
    }
    if let Some(&p) = pairs.first() {
        return HandValue { kind: HandKind::OnePair, primary: p, secondary: None };
    }

    HandValue { kind: HandKind::HighCard, primary: high, secondary: None }
}

fn player_with_best_uphand(players: &[SessionPlayer]) -> usize {
    let ai = active_indices(players);
    let mut best_idx = *ai.iter()
        .find(|&&idx| !players[idx].game.cards_up.is_empty())
        .unwrap_or(&ai[0]);
    let mut best: Option<common::ranking::HandValue> = None;

    for &idx in &ai {
        let up = &players[idx].game.cards_up;
        if up.is_empty() {
            continue;
        }

        let hv = if up.len() >= 5 {
            best_of_n(up, &[])
        } else {
            evaluate_partial(up)
        };

        match &best {
            None => { best = Some(hv); best_idx = idx; }
            Some(b) if hv > *b => { best = Some(hv); best_idx = idx; }
            _ => {}
        }
    }
    best_idx
}


async fn run_draw_phase(
    players: &mut Vec<SessionPlayer>,
    deck: &mut Deck,
    pot: i32,
    dealer_idx: usize,
    start_idx: usize,
    hand_number: u32,
    action_rx: &mut UnboundedReceiver<(String, String)>,
    config: &GameConfig,
    db: &PlayerDatabase,
    ctx: &mut HandContext,
) {
    let n = players.len();
    let indices = active_indices(players);
    let start_pos = indices.iter().position(|&i| i == start_idx).unwrap_or(0);
    let contributions = vec![0; n];
    let mut pending_leaves: std::collections::HashSet<String> = std::collections::HashSet::new();

    for offset in 0..indices.len() {
        let idx = indices[(start_pos + offset) % indices.len()];

        for name in pending_leaves.drain() {
            if let Some(i) = players.iter().position(|sp| sp.game.name == name) {
                players[i].game.is_active = false;
                players[i].game.left_mid_hand = true;
                send_to(&players[i].tx, "GAME_LEFT You have left the game");
                let _ = db.update_balance(&name, players[i].game.balance);
                let _ = db.increment_player_stats(&name, 1, 0, 1);
                ctx.last_msg = format!("{} disconnected.", name);
            }
        }

        if active_count(players) <= 1 {
            return;
        }

        if !players[idx].game.is_active {
            continue;
        }

        let active_name = players[idx].game.name.clone();
        let active_tx = players[idx].tx.clone();

        let state = build_state(
            players,
            Phase::Draw,
            GameMode::FiveCardDraw,
            pot,
            0,
            &contributions,
            dealer_idx,
            idx,
            hand_number,
            ctx,
            vec![],
        );
        broadcast_state(players, &state);

        let action = wait_for_action(
            action_rx,
            &active_name,
            &active_tx,
            &["DRAW", "LEAVE"],
            &mut pending_leaves,
        )
        .await;

        ctx.last_msg = match action {
            Action::Draw(draw_indices) => {
                let count = draw_indices.len();
                if count == 0 || !players[idx].game.can_redraw(config.max_redraws) {
                    format!("{} stands pat", active_name)
                } else {
                    players[idx].game.discard_and_draw(draw_indices, deck);
                    format!("{} draws {} card(s)", active_name, count)
                }
            }
            Action::Leave => {
                players[idx].game.is_active = false;
                players[idx].game.left_mid_hand = true;
                send_to(&players[idx].tx, "GAME_LEFT You have left the game");
                let _ = db.update_balance(&active_name, players[idx].game.balance);
                let _ = db.increment_player_stats(&active_name, 1, 0, 1);
                let remaining = active_count(players);
                if remaining <= 1 {
                    format!(
                        "{} left. Game ended due to insufficient players.",
                        active_name
                    )
                } else {
                    format!("{} left. {} players remaining.", active_name, remaining)
                }
            }
            _ => format!("{} stands pat", active_name),
        };

        let state = build_state(
            players,
            Phase::Draw,
            GameMode::FiveCardDraw,
            pot,
            0,
            &contributions,
            dealer_idx,
            idx,
            hand_number,
            ctx,
            vec![],
        );
        broadcast_state(players, &state);

        if active_count(players) <= 1 {
            return;
        }
    }
}

async fn run_five_card_draw_hand(
    players: &mut Vec<SessionPlayer>,
    dealer_idx: usize,
    hand_number: u32,
    action_rx: &mut UnboundedReceiver<(String, String)>,
    config: &GameConfig,
    db: &PlayerDatabase,
    ctx: &mut HandContext,
) {
    let n = players.len();

    let mut pot = 0i32;
    let mut contributions = vec![0i32; n];

    let early = run_ante_round(
        players,
        &mut pot,
        &mut contributions,
        dealer_idx,
        hand_number,
        GameMode::FiveCardDraw,
        action_rx,
        config,
        db,
        ctx,
    )
    .await;
    if early {
        handle_early_end(
            players,
            &mut pot,
            dealer_idx,
            hand_number,
            GameMode::FiveCardDraw,
            db,
            ctx,
        );
        return;
    }
    let mut deck = Deck::new();
    deck.shuffle(&mut thread_rng());
    for sp in players.iter_mut() {
        if sp.game.is_active {
            sp.game.deal_cards(&mut deck, 5);
        }
    }
    contributions = vec![0; n];

    let first = (dealer_idx + 1) % n;
    ctx.last_msg = "Cards dealt".to_string();
    let state = build_state(
        players,
        Phase::FirstBetting,
        GameMode::FiveCardDraw,
        pot,
        0,
        &contributions,
        dealer_idx,
        first,
        hand_number,
        ctx,
        vec![],
    );
    broadcast_state(players, &state);

    contributions = vec![0; n];
    let early = run_betting_round(
        players,
        &mut pot,
        &mut contributions,
        first,
        hand_number,
        dealer_idx,
        Phase::FirstBetting,
        GameMode::FiveCardDraw,
        action_rx,
        config,
        db,
        ctx,
        vec![],
        0,
    )
    .await;
    if early {
        handle_early_end(
            players,
            &mut pot,
            dealer_idx,
            hand_number,
            GameMode::FiveCardDraw,
            db,
            ctx,
        );
        return;
    }

    run_draw_phase(
        players,
        &mut deck,
        pot,
        dealer_idx,
        first,
        hand_number,
        action_rx,
        config,
        db,
        ctx,
    )
    .await;

    contributions = vec![0; n];
    let early = run_betting_round(
        players,
        &mut pot,
        &mut contributions,
        first,
        hand_number,
        dealer_idx,
        Phase::SecondBetting,
        GameMode::FiveCardDraw,
        action_rx,
        config,
        db,
        ctx,
        vec![],
        0,
    )
    .await;
    if early {
        handle_early_end(
            players,
            &mut pot,
            dealer_idx,
            hand_number,
            GameMode::FiveCardDraw,
            db,
            ctx,
        );
        return;
    }

    let (wi, winning_hand, amount_won) = run_showdown(
        players,
        &mut pot,
        dealer_idx,
        hand_number,
        GameMode::FiveCardDraw,
        ctx,
        &[],
    );
    update_stats(players, &wi, db, &winning_hand, amount_won);
}


async fn run_holdem_hand(
    players: &mut Vec<SessionPlayer>,
    dealer_idx: usize,
    hand_number: u32,
    action_rx: &mut UnboundedReceiver<(String, String)>,
    config: &GameConfig,
    db: &PlayerDatabase,
    ctx: &mut HandContext,
) {
    let n = players.len();

    let sb_idx = (dealer_idx + 1) % n;
    let bb_idx = (dealer_idx + 2) % n;

    let mut pot = 0;
    let mut contributions = vec![0i32; n];

    let sb_amount = config.small_blind.min(players[sb_idx].game.balance);
    players[sb_idx].game.balance -= sb_amount;
    contributions[sb_idx] += sb_amount;
    pot += sb_amount;

    ctx.last_msg = format!(
        "{} posts small blind ${}",
        players[sb_idx].game.name, sb_amount
    );
    ctx.winners = vec![];
    let state = build_state(
        players,
        Phase::SmallBlind,
        GameMode::TexasHoldem,
        pot,
        sb_amount,
        &contributions,
        dealer_idx,
        sb_idx,
        hand_number,
        ctx,
        vec![],
    );
    broadcast_state(players, &state);

    let bb_amount = config.big_blind.min(players[bb_idx].game.balance);
    players[bb_idx].game.balance -= bb_amount;
    contributions[bb_idx] += bb_amount;
    pot += bb_amount;

    ctx.last_msg = format!(
        "{} posts big blind ${}",
        players[bb_idx].game.name, bb_amount
    );
    let state = build_state(
        players,
        Phase::BigBlind,
        GameMode::TexasHoldem,
        pot,
        bb_amount,
        &contributions,
        dealer_idx,
        bb_idx,
        hand_number,
        ctx,
        vec![],
    );
    broadcast_state(players, &state);

    if active_count(players) <= 1 {
        handle_early_end(
            players,
            &mut pot,
            dealer_idx,
            hand_number,
            GameMode::TexasHoldem,
            db,
            ctx,
        );
        return;
    }

    let mut deck = Deck::new();
    deck.shuffle(&mut thread_rng());
    for sp in players.iter_mut() {
        if sp.game.is_active {
            sp.game.deal_cards(&mut deck, 2);
        }
    }

    let utg = (bb_idx + 1) % n;
    ctx.last_msg = "Hole cards dealt - pre-flop betting".to_string();

    let early = run_betting_round(
        players,
        &mut pot,
        &mut contributions,
        utg,
        hand_number,
        dealer_idx,
        Phase::PreFlop,
        GameMode::TexasHoldem,
        action_rx,
        config,
        db,
        ctx,
        vec![],
        config.big_blind,
    )
    .await;
    if early {
        handle_early_end(
            players,
            &mut pot,
            dealer_idx,
            hand_number,
            GameMode::TexasHoldem,
            db,
            ctx,
        );
        return;
    }

    let mut community: Vec<Card> = Vec::new();
    for _ in 0..3 {
        if let Some(c) = deck.deal() {
            community.push(c);
        }
    }

    contributions = vec![0; n];
    ctx.last_msg = format!(
        "Flop: {} {} {}",
        card_label(&community[0]),
        card_label(&community[1]),
        card_label(&community[2])
    );

    let early = run_betting_round(
        players,
        &mut pot,
        &mut contributions,
        (dealer_idx + 1) % n,
        hand_number,
        dealer_idx,
        Phase::Flop,
        GameMode::TexasHoldem,
        action_rx,
        config,
        db,
        ctx,
        community.clone(),
        0,
    )
    .await;
    if early {
        handle_early_end(
            players,
            &mut pot,
            dealer_idx,
            hand_number,
            GameMode::TexasHoldem,
            db,
            ctx,
        );
        return;
    }

    if let Some(c) = deck.deal() {
        community.push(c);
    }

    contributions = vec![0; n];
    ctx.last_msg = format!("Turn: {}", card_label(community.last().unwrap()));

    let early = run_betting_round(
        players,
        &mut pot,
        &mut contributions,
        (dealer_idx + 1) % n,
        hand_number,
        dealer_idx,
        Phase::Turn,
        GameMode::TexasHoldem,
        action_rx,
        config,
        db,
        ctx,
        community.clone(),
        0,
    )
    .await;
    if early {
        handle_early_end(
            players,
            &mut pot,
            dealer_idx,
            hand_number,
            GameMode::TexasHoldem,
            db,
            ctx,
        );
        return;
    }

    if let Some(c) = deck.deal() {
        community.push(c);
    }

    contributions = vec![0; n];
    ctx.last_msg = format!("River: {}", card_label(community.last().unwrap()));

    let early = run_betting_round(
        players,
        &mut pot,
        &mut contributions,
        (dealer_idx + 1) % n,
        hand_number,
        dealer_idx,
        Phase::River,
        GameMode::TexasHoldem,
        action_rx,
        config,
        db,
        ctx,
        community.clone(),
        0,
    )
    .await;
    if early {
        handle_early_end(
            players,
            &mut pot,
            dealer_idx,
            hand_number,
            GameMode::TexasHoldem,
            db,
            ctx,
        );
        return;
    }
    ctx.community_cards = community.clone();
    let (wi, winning_hand, amount_won) = run_showdown(
        players,
        &mut pot,
        dealer_idx,
        hand_number,
        GameMode::TexasHoldem,
        ctx,
        &community,
    );
    update_stats(players, &wi, db, &winning_hand, amount_won);
}


async fn run_stud_hand(
    players: &mut Vec<SessionPlayer>,
    dealer_idx: usize,
    hand_number: u32,
    action_rx: &mut UnboundedReceiver<(String, String)>,
    config: &GameConfig,
    db: &PlayerDatabase,
    ctx: &mut HandContext,
) {
    let n = players.len();

    let mut pot = 0i32;
    let mut contributions = vec![0i32; n];

    let early = run_ante_round(
        players,
        &mut pot,
        &mut contributions,
        dealer_idx,
        hand_number,
        GameMode::SevenCardStud,
        action_rx,
        config,
        db,
        ctx,
    )
    .await;
    if early {
        handle_early_end(
            players,
            &mut pot,
            dealer_idx,
            hand_number,
            GameMode::SevenCardStud,
            db,
            ctx,
        );
        return;
    }

    let mut deck = Deck::new();
    deck.shuffle(&mut thread_rng());

    for sp in players.iter_mut() {
        if sp.game.is_active {
            sp.game.deal_card_down(&mut deck); 
            sp.game.deal_card_down(&mut deck); 
            sp.game.deal_card_up(&mut deck); 
        }
    }

    let third_street_opener = player_with_lowest_upcard(players);
    contributions = vec![0; n];
    ctx.last_msg = format!(
        "Third Street: {} opens",
        players[third_street_opener].game.name
    );

    let early = run_betting_round(
        players,
        &mut pot,
        &mut contributions,
        third_street_opener,
        hand_number,
        dealer_idx,
        Phase::ThirdStreet,
        GameMode::SevenCardStud,
        action_rx,
        config,
        db,
        ctx,
        vec![],
        0,
    )
    .await;
    if early {
        handle_early_end(
            players,
            &mut pot,
            dealer_idx,
            hand_number,
            GameMode::SevenCardStud,
            db,
            ctx,
        );
        return;
    }

    for sp in players.iter_mut() {
        if sp.game.is_active {
            sp.game.deal_card_up(&mut deck);
        }
    }

    let opener = player_with_best_uphand(players);
    contributions = vec![0; n];
    ctx.last_msg = format!("Fourth Street: {} opens", players[opener].game.name);

    let early = run_betting_round(
        players,
        &mut pot,
        &mut contributions,
        opener,
        hand_number,
        dealer_idx,
        Phase::FourthStreet,
        GameMode::SevenCardStud,
        action_rx,
        config,
        db,
        ctx,
        vec![],
        0,
    )
    .await;
    if early {
        handle_early_end(
            players,
            &mut pot,
            dealer_idx,
            hand_number,
            GameMode::SevenCardStud,
            db,
            ctx,
        );
        return;
    }

    for sp in players.iter_mut() {
        if sp.game.is_active {
            sp.game.deal_card_up(&mut deck);
        }
    }

    let opener = player_with_best_uphand(players);
    contributions = vec![0; n];
    ctx.last_msg = format!("Fifth Street: {} opens", players[opener].game.name);

    let early = run_betting_round(
        players,
        &mut pot,
        &mut contributions,
        opener,
        hand_number,
        dealer_idx,
        Phase::FifthStreet,
        GameMode::SevenCardStud,
        action_rx,
        config,
        db,
        ctx,
        vec![],
        0,
    )
    .await;
    if early {
        handle_early_end(
            players,
            &mut pot,
            dealer_idx,
            hand_number,
            GameMode::SevenCardStud,
            db,
            ctx,
        );
        return;
    }

    for sp in players.iter_mut() {
        if sp.game.is_active {
            sp.game.deal_card_up(&mut deck);
        }
    }

    let opener = player_with_best_uphand(players);
    contributions = vec![0; n];
    ctx.last_msg = format!("Sixth Street: {} opens", players[opener].game.name);

    let early = run_betting_round(
        players,
        &mut pot,
        &mut contributions,
        opener,
        hand_number,
        dealer_idx,
        Phase::SixthStreet,
        GameMode::SevenCardStud,
        action_rx,
        config,
        db,
        ctx,
        vec![],
        0,
    )
    .await;
    if early {
        handle_early_end(
            players,
            &mut pot,
            dealer_idx,
            hand_number,
            GameMode::SevenCardStud,
            db,
            ctx,
        );
        return;
    }

    for sp in players.iter_mut() {
        if sp.game.is_active {
            sp.game.deal_card_down(&mut deck); 
        }
    }

    let opener = player_with_best_uphand(players);
    contributions = vec![0; n];
    ctx.last_msg = format!("Seventh Street: {} opens", players[opener].game.name);

    let early = run_betting_round(
        players,
        &mut pot,
        &mut contributions,
        opener,
        hand_number,
        dealer_idx,
        Phase::SeventhStreet,
        GameMode::SevenCardStud,
        action_rx,
        config,
        db,
        ctx,
        vec![],
        0,
    )
    .await;
    if early {
        handle_early_end(
            players,
            &mut pot,
            dealer_idx,
            hand_number,
            GameMode::SevenCardStud,
            db,
            ctx,
        );
        return;
    }

    let (wi, winning_hand, amount_won) = run_showdown(
        players,
        &mut pot,
        dealer_idx,
        hand_number,
        GameMode::SevenCardStud,
        ctx,
        &[],
    );
    update_stats(players, &wi, db, &winning_hand, amount_won);
}


async fn ask_dealer_choice(
    players: &mut Vec<SessionPlayer>,
    dealer_idx: usize,
    hand_number: u32,
    action_rx: &mut UnboundedReceiver<(String, String)>,
    ctx: &mut HandContext,
    db: &PlayerDatabase,
) -> Option<GameMode> {
    let dealer_name = players[dealer_idx].game.name.clone();
    let dealer_tx = players[dealer_idx].tx.clone();

    ctx.last_msg = format!("{} is choosing the game variant…", dealer_name);
    ctx.winners = vec![];

    let state = build_state(
        players,
        Phase::DealerChoice,
        GameMode::FiveCardDraw,
        0,
        0,
        &vec![0; players.len()],
        dealer_idx,
        dealer_idx,
        hand_number,
        ctx,
        vec![],
    );
    broadcast_state(players, &state);

    loop {
        match action_rx.recv().await {
            None => return None,
            Some((username, cmd)) => {
                if cmd.trim() == "LEAVE" {
                    let is_dealer = username == dealer_name;
                    if let Some(i) = players.iter().position(|sp| sp.game.name == username) {
                        let _ = db.update_balance(&username, players[i].game.balance);
                        send_to(&players[i].tx, "GAME_LEFT You have left the game");
                        players.remove(i);
                    }
                    if players.len() < 2 {
                        return None;
                    }
                    if is_dealer {
                        let modes = [
                            GameMode::FiveCardDraw,
                            GameMode::TexasHoldem,
                            GameMode::SevenCardStud,
                        ];
                        let mode = modes[rand::random::<usize>() % 3].clone();
                        ctx.last_msg = format!("Dealer left. {} chosen at random", mode.label());
                        let state = build_state(
                            players,
                            Phase::DealerChoice,
                            mode.clone(),
                            0,
                            0,
                            &vec![0; players.len()],
                            dealer_idx.min(players.len() - 1),
                            dealer_idx.min(players.len() - 1),
                            hand_number,
                            ctx,
                            vec![],
                        );
                        broadcast_state(players, &state);
                        return Some(mode);
                    } else {
                        ctx.last_msg =
                            format!("{} left. Waiting for {} to choose…", username, dealer_name);
                        let state = build_state(
                            players,
                            Phase::DealerChoice,
                            GameMode::FiveCardDraw,
                            0,
                            0,
                            &vec![0; players.len()],
                            dealer_idx.min(players.len() - 1),
                            dealer_idx.min(players.len() - 1),
                            hand_number,
                            ctx,
                            vec![],
                        );
                        broadcast_state(players, &state);
                        continue;
                    }
                }

                if username != dealer_name {
                    continue;
                }
                if let Some(Action::DealerChoice(mode)) = parse_action(&cmd) {
                    ctx.last_msg = format!("{} chose {}", dealer_name, mode.label());
                    let state = build_state(
                        players,
                        Phase::DealerChoice,
                        mode.clone(),
                        0,
                        0,
                        &vec![0; players.len()],
                        dealer_idx,
                        dealer_idx,
                        hand_number,
                        ctx,
                        vec![],
                    );
                    broadcast_state(players, &state);
                    return Some(mode);
                }
                send_to(
                    &dealer_tx,
                    "ERR Choose a variant: MODE FiveCardDraw | MODE TexasHoldem | MODE SevenCardStud",
                );
            }
        }
    }
}

pub async fn run_game(
    session_players: Vec<(String, SocketAddr, UnboundedSender<String>)>,
    action_rx: UnboundedReceiver<(String, String)>,
    config: GameConfig,
    db: Arc<PlayerDatabase>,
) {
    let mut action_rx = action_rx;

    let mut players: Vec<SessionPlayer> = session_players
        .into_iter()
        .filter_map(|(username, addr, tx)| {
            db.get_player(&username).ok().map(|dbp| SessionPlayer {
                game: GamePlayer::from_db(&dbp),
                addr,
                tx,
            })
        })
        .collect();

    if players.len() < 2 {
        for sp in &players {
            send_to(&sp.tx, "ERR Not enough players to start");
        }
        set_final_player_senders(players.iter().map(|sp| sp.tx.clone()).collect()).await;
        return;
    }

    let mut dealer_idx = 0usize;
    let mut hand_number = 1u32;

    loop {
        let min_required = config.ante.min(config.small_blind);
        let broke_txs: Vec<_> = players
            .iter()
            .filter(|sp| sp.game.balance < min_required)
            .map(|sp| sp.tx.clone())
            .collect();
        players.retain(|sp| sp.game.balance >= min_required);

        if !broke_txs.is_empty() {
            let broke_ctx = HandContext {
                last_msg: "You have run out of funds and have been removed from the game."
                    .to_string(),
                winners: vec![],
                community_cards: vec![],
            };
            let state = build_state(
                &players,
                Phase::GameOver,
                GameMode::FiveCardDraw,
                0,
                0,
                &[],
                0,
                0,
                hand_number,
                &broke_ctx,
                vec![],
            );
            if let Ok(json) = serde_json::to_string(&state) {
                let msg = format!("GAME_STATE {}", json);
                for tx in &broke_txs {
                    send_to(tx, &msg);
                }
            }
        }

        if players.len() < 2 {
            let ctx = HandContext {
                last_msg: "Not enough players to continue. Game ended.".to_string(),
                winners: vec![],
                community_cards: vec![],
            };
            let state = build_state(
                &players,
                Phase::GameOver,
                GameMode::FiveCardDraw,
                0,
                0,
                &[],
                0,
                0,
                hand_number,
                &ctx,
                vec![],
            );
            broadcast_state(&players, &state);
            break;
        }

        if dealer_idx >= players.len() {
            dealer_idx = 0;
        }

        for sp in players.iter_mut() {
            sp.game.reset_for_new_hand();
        }

        let mut ctx = HandContext::new();
        let mode = match ask_dealer_choice(
            &mut players,
            dealer_idx,
            hand_number,
            &mut action_rx,
            &mut ctx,
            &db,
        )
        .await
        {
            Some(m) => m,
            None => {
                let ctx = HandContext {
                    last_msg: "Not enough players to continue. Game ended.".to_string(),
                    winners: vec![],
                    community_cards: vec![],
                };
                let state = build_state(
                    &players,
                    Phase::GameOver,
                    GameMode::FiveCardDraw,
                    0,
                    0,
                    &[],
                    0,
                    0,
                    hand_number,
                    &ctx,
                    vec![],
                );
                broadcast_state(&players, &state);
                break;
            }
        };

        match mode {
            GameMode::FiveCardDraw => {
                run_five_card_draw_hand(
                    &mut players,
                    dealer_idx,
                    hand_number,
                    &mut action_rx,
                    &config,
                    &db,
                    &mut ctx,
                )
                .await;
            }
            GameMode::TexasHoldem => {
                run_holdem_hand(
                    &mut players,
                    dealer_idx,
                    hand_number,
                    &mut action_rx,
                    &config,
                    &db,
                    &mut ctx,
                )
                .await;
            }
            GameMode::SevenCardStud => {
                run_stud_hand(
                    &mut players,
                    dealer_idx,
                    hand_number,
                    &mut action_rx,
                    &config,
                    &db,
                    &mut ctx,
                )
                .await;
            }
        }

        players = round_over_collect(
            players,
            &mut dealer_idx,
            hand_number,
            &mut action_rx,
            &mut ctx,
            &db,
        )
        .await;

        if players.len() < 2 {
            let ctx = HandContext {
                last_msg: "Not enough players to continue. Game ended.".to_string(),
                winners: vec![],
                community_cards: vec![],
            };
            let state = build_state(
                &players,
                Phase::GameOver,
                GameMode::FiveCardDraw,
                0,
                0,
                &[],
                0,
                0,
                hand_number,
                &ctx,
                vec![],
            );
            broadcast_state(&players, &state);
            break;
        }

        dealer_idx = (dealer_idx + 1) % players.len();
        hand_number += 1;
    }
    set_final_player_senders(players.iter().map(|sp| sp.tx.clone()).collect()).await;
}

const ROUND_OVER_TIMEOUT_SECS: u64 = 60;

async fn round_over_collect(
    mut players: Vec<SessionPlayer>,
    dealer_idx: &mut usize,
    hand_number: u32,
    action_rx: &mut UnboundedReceiver<(String, String)>,
    ctx: &mut HandContext,
    db: &Arc<PlayerDatabase>,
) -> Vec<SessionPlayer> {
    let mut leavers: std::collections::HashSet<String> = players
        .iter()
        .filter(|sp| sp.game.left_mid_hand)
        .map(|sp| sp.game.name.clone())
        .collect();

    let mut pending: std::collections::HashMap<String, tokio::time::Instant> = players
        .iter()
        .filter(|sp| !sp.game.left_mid_hand)
        .map(|sp| {
            (
                sp.game.name.clone(),
                tokio::time::Instant::now()
                    + tokio::time::Duration::from_secs(ROUND_OVER_TIMEOUT_SECS),
            )
        })
        .collect();

    loop {
        match action_rx.try_recv() {
            Ok((username, cmd)) if cmd.trim() == "LEAVE" => {
                if let Some(sp) = players.iter_mut().find(|sp| sp.game.name == username) {
                    if !sp.game.left_mid_hand {
                        sp.game.left_mid_hand = true;
                        send_to(&sp.tx, "GAME_LEFT You have left the game");
                        let _ = db.update_balance(&username, sp.game.balance);
                    }
                }
                leavers.insert(username);
            }
            Ok(_) => {}
            Err(_) => break,
        }
    }

    broadcast_round_over_state(
        &players,
        *dealer_idx,
        hand_number,
        ctx,
        ROUND_OVER_TIMEOUT_SECS,
    );

    while !pending.is_empty() {
        let now = tokio::time::Instant::now();
        let next_deadline = pending.values().copied().min().unwrap_or(now);
        let wait = next_deadline.saturating_duration_since(now);

        tokio::select! {
            msg = action_rx.recv() => {
                match msg {
                    None => {
                        leavers.extend(pending.keys().cloned());
                        pending.clear();
                    }
                    Some((username, cmd)) => {
                        if !pending.contains_key(&username) {
                            continue; 
                        }
                        match cmd.trim() {
                            "CONTINUE" => {
                                pending.remove(&username);
                            }
                            "LEAVE" => {
                                pending.remove(&username);
                                leavers.insert(username.clone());
                                apply_leave(&mut players, *dealer_idx, hand_number, ctx, &username);
                            }
                            _ => {
                                if let Some(sp) = players.iter().find(|sp| sp.game.name == username) {
                                    send_to(&sp.tx, "ERR Send CONTINUE or LEAVE");
                                }
                            }
                        }
                    }
                }
            }
            _ = tokio::time::sleep(wait) => {
                let now = tokio::time::Instant::now();
                let timed_out: Vec<String> = pending
                    .iter()
                    .filter(|(_, deadline)| **deadline <= now)
                    .map(|(name, _)| name.clone())
                    .collect();

                for username in timed_out {
                    pending.remove(&username);
                    leavers.insert(username.clone());
                    apply_leave(&mut players, *dealer_idx, hand_number, ctx, &username);
                }
            }
        }
    }

    remove_leavers(&mut players, dealer_idx, &leavers, db);

    players
}

fn apply_leave(
    players: &mut Vec<SessionPlayer>,
    dealer_idx: usize,
    hand_number: u32,
    ctx: &mut HandContext,
    username: &str,
) {
    if let Some(pos) = players.iter().position(|sp| sp.game.name == username) {
        players[pos].game.is_active = false;

        send_to(&players[pos].tx, "GAME_LEFT You have left the game");

        let remaining_after = players
            .iter()
            .filter(|sp| !sp.game.left_mid_hand && sp.game.name != username)
            .count();
        ctx.last_msg = if remaining_after <= 1 {
            format!("{} left. Game ended due to insufficient players.", username)
        } else {
            format!("{} left. {} players remaining.", username, remaining_after)
        };

        broadcast_round_over_state(players, dealer_idx, hand_number, ctx, 0);
    }
}

fn remove_leavers(
    players: &mut Vec<SessionPlayer>,
    dealer_idx: &mut usize,
    leavers: &std::collections::HashSet<String>,
    db: &Arc<PlayerDatabase>,
) {
    if leavers.is_empty() {
        return;
    }

    for sp in players.iter().filter(|sp| leavers.contains(&sp.game.name)) {
        let _ = db.update_balance(&sp.game.name, sp.game.balance);
    }

    let original_dealer_idx = *dealer_idx;
    let dealer_name = players.get(*dealer_idx).map(|sp| sp.game.name.clone());

    players.retain(|sp| !leavers.contains(&sp.game.name));

    if players.is_empty() {
        *dealer_idx = 0;
        return;
    }

    *dealer_idx = if let Some(name) = dealer_name {
        players
            .iter()
            .position(|sp| sp.game.name == name)
            .unwrap_or_else(|| {
                original_dealer_idx.min(players.len() - 1)
            })
    } else {
        0
    };
}

fn broadcast_round_over_state(
    players: &[SessionPlayer],
    dealer_idx: usize,
    hand_number: u32,
    ctx: &HandContext,
    timeout_secs: u64,
) {
    let enriched_ctx = HandContext {
    last_msg: if timeout_secs > 0 {
        format!("{} You have {}s to continue.", ctx.last_msg, timeout_secs)
    } else {
        ctx.last_msg.clone()
    },
    winners: ctx.winners.clone(),
    community_cards: ctx.community_cards.clone(), // add this line
};

    let state = build_state(
    players,
    Phase::RoundOver,
    GameMode::FiveCardDraw,
    0,
    0,
    &vec![0; players.len()],
    dealer_idx,
    0,
    hand_number,
    &enriched_ctx,
    enriched_ctx.community_cards.clone(), // was vec![]
);
    broadcast_state(players, &state);
}
