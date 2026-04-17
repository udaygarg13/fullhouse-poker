
use crate::ws::WsHandle;
use common::cards::{Rank, Suit};
use common::game_models::{GameMode, GameStateMsg, Phase};
use dioxus::prelude::*;

fn rank_label(rank: &Rank) -> &'static str {
    match rank {
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
    }
}

fn suit_symbol(suit: &Suit) -> &'static str {
    match suit {
        Suit::Spades => "♠",
        Suit::Hearts => "♥",
        Suit::Diamonds => "♦",
        Suit::Clubs => "♣",
    }
}

fn suit_is_red(suit: &Suit) -> bool {
    matches!(suit, Suit::Hearts | Suit::Diamonds)
}

fn card_sort_key(rank: &Rank, suit: &Suit) -> (u8, u8) {
    let r = match rank {
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
    };
    let s = match suit {
        Suit::Spades => 0,
        Suit::Hearts => 1,
        Suit::Diamonds => 2,
        Suit::Clubs => 3,
    };
    (r, s)
}

fn parse_round_over_timeout(msg: &str) -> Option<u32> {
    let marker = "You have ";
    let pos = msg.find(marker)?;
    let rest = &msg[pos + marker.len()..];
    let end = rest.find('s')?;
    rest[..end].trim().parse().ok()
}

#[component]
pub fn GameScreen(
    ws_handle: Signal<Option<WsHandle>>,
    server_msgs: Signal<Vec<String>>,
    username: Signal<String>,
    screen: Signal<crate::Screen>,
) -> Element {
    let mut game: Signal<Option<GameStateMsg>> = use_signal(|| None);
    let mut selected_discards: Signal<Vec<usize>> = use_signal(|| vec![]);
    let mut raise_input = use_signal(|| String::new());
    let mut status = use_signal(|| "Waiting for server...".to_string());
    let mut continued = use_signal(|| false);

    use dioxus::prelude::*;
    use gloo_timers::future::TimeoutFuture;

    let mut round_over_secs = use_signal(|| Some(60));

    use_future(move || async move {
        loop {
            TimeoutFuture::new(1_000).await;
            if let Some(n) = round_over_secs() {
                if n > 0 {
                    round_over_secs.set(Some(n - 1));
                }
            }
        }
    });

    use_effect(move || {
        let msgs = server_msgs();
        if msgs.is_empty() {
            return;
        }

        let mut keep: Vec<String> = vec![];
        for msg in &msgs {
            if msg.trim() == "GAME_ENDED" {
                screen.set(crate::Screen::MenuScreen);
                return;
            }
            if msg.trim() == "GAME_IN_PROGRESS" {
                keep.push(msg.clone());
                continue;
            }
            if msg.starts_with("GAME_LEFT") {
                screen.set(crate::Screen::MenuScreen);
                return;
            }

            if let Some(rest) = msg.strip_prefix("GAME_STATE ") {
                match serde_json::from_str::<GameStateMsg>(rest.trim()) {
                    Ok(state) => {
                        status.set(state.message.clone());

                        if game().map(|g| g.phase) != Some(state.phase.clone()) {
                            selected_discards.set(vec![]);
                        }

                        if matches!(state.phase, Phase::RoundOver) {
                            if let Some(secs) = parse_round_over_timeout(&state.message) {
                                round_over_secs.set(Some(secs));
                            }
                        } else {
                            round_over_secs.set(None);
                        }

                        if !matches!(state.phase, Phase::RoundOver) {
                            continued.set(false);
                        }

                        game.set(Some(state));
                    }
                    Err(e) => status.set(format!("ERR parsing state: {e}")),
                }
                continue;
            }
            if msg.starts_with("ERR") {
                status.set(msg.clone());
            }
        }

        *server_msgs.write() = keep;
    });

    let send = move |cmd: String| {
        if let Some(ws) = ws_handle().clone() {
            ws.send(format!("GAME {}", cmd));
        }
    };
    let send_menu = move |cmd: String| {
        if let Some(ws) = ws_handle().clone() {
            ws.send(cmd);
        }
    };

    let me = username();

    let (local_player, local_idx, is_my_turn, phase, _game_mode, is_dealer) = match game() {
        None => (
            None,
            0usize,
            false,
            Phase::Ante,
            GameMode::FiveCardDraw,
            false,
        ),
        Some(ref g) => {
            if let Some(idx) = g.players.iter().position(|p| p.name == me) {
                let i_am_dealer = g.dealer_idx == idx;
                let is_turn = g.active_player_idx == idx
                    && !g.players[idx].folded
                    && matches!(
                        g.phase,
                        Phase::Ante
                            | Phase::FirstBetting
                            | Phase::SecondBetting
                            | Phase::Draw
                            | Phase::PreFlop
                            | Phase::Flop
                            | Phase::Turn
                            | Phase::River
                            | Phase::ThirdStreet
                            | Phase::FourthStreet
                            | Phase::FifthStreet
                            | Phase::SixthStreet
                            | Phase::SeventhStreet
                            | Phase::DealerChoice
                    );
                (
                    Some(g.players[idx].clone()),
                    idx,
                    is_turn,
                    g.phase.clone(),
                    g.game_mode.clone(),
                    i_am_dealer,
                )
            } else {
                (
                    None,
                    0usize,
                    false,
                    g.phase.clone(),
                    g.game_mode.clone(),
                    false,
                )
            }
        }
    };

    let card_base = "relative w-14 h-20 rounded-xl border-2 flex flex-col items-center \
                     justify-center font-bold text-xl shadow-lg select-none cursor-pointer \
                     transition-all duration-150";

    rsx! {
        div {
            class: "flex flex-col h-screen bg-gray-950 font-sans text-gray-100 overflow-hidden",

            div {
                class: "relative flex items-center justify-between px-8 py-3 bg-gray-900 border-b border-gray-800 shrink-0",
                div { class: "flex items-center gap-3",
                    span { class: "text-gray-400 text-xs uppercase tracking-widest mr-1", "Phase" }
                    span { class: "text-indigo-400 text-sm font-semibold", "{phase.label()}" }
                    if let Some(ref g) = game() {
                        span {
                            class: "text-xs px-2 py-0.5 rounded-full bg-indigo-900 text-indigo-300 font-medium",
                            "{g.game_mode.label()}"
                        }
                    }
                }
                if let Some(ref g) = game() {
                    div { class: "absolute left-1/2 -translate-x-1/2 flex items-center gap-6",
                        div {
                            span { class: "text-gray-400 text-xs uppercase tracking-widest mr-2", "Pot" }
                            span { class: "text-yellow-400 font-bold text-lg", "${g.pot}" }
                        }
                        div {
                            span { class: "text-gray-400 text-xs uppercase tracking-widest mr-2", "Bet" }
                            span { class: "text-gray-200 font-semibold", "${g.current_bet}" }
                        }
                        div {
                            span { class: "text-gray-400 text-xs uppercase tracking-widest mr-2", "Hand" }
                            span { class: "text-gray-400 text-sm", "#{g.hand_number}" }
                        }
                    }
                }
                div {
                    class: "text-gray-500 text-xs max-w-xs text-right truncate",
                    "{status}"
                }
            }

            if phase == Phase::DealerChoice {
                div {
                    class: "relative flex flex-col items-center justify-center flex-1 gap-6",
                    button {
                        class: "absolute bottom-4 left-4 flex items-center gap-1.5 px-3 py-2 bg-gray-800 hover:bg-red-900 border border-gray-700 hover:border-red-700 text-gray-500 hover:text-red-300 rounded-lg text-xs font-semibold uppercase tracking-wider transition-colors duration-150",
                        title: "Leave game",
                        onclick: move |_| {
                            if local_player.is_some() {
                                send("LEAVE".to_string());
                            } else {
                                send_menu("STOP_WATCH".to_string());
                            }
                            screen.set(crate::Screen::MenuScreen);
                        },
                        span { "Leave" }
                    }
                    p { class: "text-gray-400 text-sm uppercase tracking-widest", "Dealer's Choice" }
                    if is_dealer && is_my_turn {
                        p { class: "text-gray-200 text-base font-semibold", "Choose the game variant for this hand:" }
                        div { class: "flex gap-4",
                            button {
                                class: "px-6 py-3 bg-indigo-800 hover:bg-indigo-700 text-white rounded-lg font-bold text-sm uppercase tracking-wider",
                                onclick: move |_| send("MODE FiveCardDraw".to_string()),
                                "Five-Card Draw"
                            }
                            button {
                                class: "px-6 py-3 bg-indigo-800 hover:bg-indigo-700 text-white rounded-lg font-bold text-sm uppercase tracking-wider",
                                onclick: move |_| send("MODE TexasHoldem".to_string()),
                                "Texas Hold'em"
                            }
                            button {
                                class: "px-6 py-3 bg-indigo-800 hover:bg-indigo-700 text-white rounded-lg font-bold text-sm uppercase tracking-wider",
                                onclick: move |_| send("MODE SevenCardStud".to_string()),
                                "Seven-Card Stud"
                            }
                        }
                    } else {
                        if let Some(ref g) = game() {
                            p {
                                class: "text-gray-500 text-sm",
                                "Waiting for {g.players[g.dealer_idx].name} to choose…"
                            }
                        }
                    }
                }
            } else {
                if let Some(ref g) = game() {
                    div {
                        class: "flex gap-6 px-5 py-5 bg-gray-900/50 border-b border-gray-800 overflow-x-auto min-w-0 custom-scrollbar",
                        style: "justify-content: safe center",
                        for (idx, player) in g.players.iter().enumerate() {
                            {
                                let is_me = player.name == me;
                                let is_active_turn = g.active_player_idx == idx
                                    && matches!(g.phase,
                                        Phase::Ante
                                        | Phase::FirstBetting | Phase::SecondBetting | Phase::Draw
                                        | Phase::PreFlop | Phase::Flop | Phase::Turn | Phase::River
                                        | Phase::ThirdStreet | Phase::FourthStreet | Phase::FifthStreet
                                        | Phase::SixthStreet | Phase::SeventhStreet);

                                let total_hand_size: usize = match g.game_mode {
                                    GameMode::FiveCardDraw => 5,
                                    GameMode::TexasHoldem => 2,
                                    GameMode::SevenCardStud => match g.phase {
                                        Phase::ThirdStreet => 3,
                                        Phase::FourthStreet => 4,
                                        Phase::FifthStreet => 5,
                                        Phase::SixthStreet => 6,
                                        _ => 7,
                                    },
                                };

                                let face_down_count: usize = if is_me {
                                    0
                                } else {
                                    total_hand_size.saturating_sub(player.cards_up.as_ref().map(|c| c.len()).unwrap_or(0))
                                };
                                let has_cards = player.cards.as_ref().map(|c| !c.is_empty()).unwrap_or(false);
                                let should_show = matches!(phase, Phase::Showdown | Phase::RoundOver)
                                    && has_cards
                                    && (!player.folded || matches!(g.game_mode, GameMode::SevenCardStud));
                                let player_count = g.players.len();
                                let is_small_blind = matches!(g.game_mode, GameMode::TexasHoldem)
                                    && player_count > 1
                                    && idx == (g.dealer_idx + 1) % player_count;
                                let is_big_blind = matches!(g.game_mode, GameMode::TexasHoldem)
                                    && player_count > 2
                                    && idx == (g.dealer_idx + 2) % player_count;

                                let is_stud = matches!(g.game_mode, GameMode::SevenCardStud);
                                let sorted_up_cards: Vec<_> = {
                                    let mut v: Vec<_> = player.cards_up
                                        .as_ref()
                                        .map(|uc| uc.iter().collect())
                                        .unwrap_or_default();
                                    if is_stud {
                                        v.sort_by(|a, b| {
                                            card_sort_key(&b.rank, &b.suit)
                                                .cmp(&card_sort_key(&a.rank, &a.suit))
                                        });
                                    }
                                    v
                                };

                                rsx! {
                                    div {
                                        class: if is_me && is_active_turn {
                                            "flex flex-col items-center gap-2 p-3 rounded-xl border-2 border-green-500 bg-gray-900 ring-2 ring-green-500/30 min-w-[145px] shrink-0"
                                        } else if is_me {
                                            "flex flex-col items-center gap-2 p-3 rounded-xl border-2 border-green-800 bg-gray-900 min-w-[145px] shrink-0"
                                        } else if is_active_turn {
                                            "flex flex-col items-center gap-2 p-3 rounded-xl border border-indigo-600 bg-gray-900 min-w-[145px] shrink-0"
                                        } else if player.folded || player.left {
                                            "flex flex-col items-center gap-2 p-3 rounded-xl border border-gray-800 bg-gray-900 opacity-50 min-w-[145px] shrink-0"
                                        } else {
                                            "flex flex-col items-center gap-2 p-3 rounded-xl border border-gray-800 bg-gray-900 min-w-[145px] shrink-0"
                                        },

                                        div { class: "flex items-center gap-1.5",
                                            span {
                                                class: if is_me { "text-green-300 font-semibold text-sm" } else { "text-gray-200 font-semibold text-sm" },
                                                "{player.name}"
                                            }
                                            if is_me {
                                                span { class: "text-xs bg-green-900 text-green-300 px-1.5 py-0.5 rounded font-bold", "You" }
                                            }
                                            if player.is_dealer {
                                                span { class: "text-xs bg-yellow-700 text-yellow-200 px-1.5 py-0.5 rounded font-bold", "D" }
                                            }
                                            if is_small_blind {
                                                span { class: "text-xs bg-blue-800 text-blue-200 px-1.5 py-0.5 rounded font-bold", "SB" }
                                            }
                                            if is_big_blind {
                                                span { class: "text-xs bg-purple-800 text-purple-200 px-1.5 py-0.5 rounded font-bold", "BB" }
                                            }
                                            if player.folded {
                                                span { class: "text-xs bg-red-900 text-red-300 px-1.5 py-0.5 rounded", "F" }
                                            }
                                            if player.left {
                                                span { class: "text-xs bg-red-900 text-red-300 px-1.5 py-0.5 rounded", "L" }
                                            }
                                        }

                                        div { class: "flex gap-1",
                                            if is_me && !player.folded && !player.left {
                                                if should_show {
                                                    if let Some(ref cards) = player.cards {
                                                        {
                                                            let mut display_cards: Vec<_> = cards.iter().collect();
                                                            if is_stud {
                                                                display_cards.sort_by(|a, b| {
                                                                    card_sort_key(&b.rank, &b.suit)
                                                                        .cmp(&card_sort_key(&a.rank, &a.suit))
                                                                });
                                                            }
                                                            let prepared: Vec<bool> = display_cards.iter().map(|card| suit_is_red(&card.suit)).collect();
                                                            rsx! {
                                                                for (is_red, card) in prepared.iter().zip(display_cards.iter()) {
                                                                    div {
                                                                        class: "w-10 h-14 rounded-lg border-2 border-green-600 bg-gray-800 flex flex-col items-center justify-center",
                                                                        span { class: if *is_red { "text-red-400 text-xs font-bold leading-none" } else { "text-gray-100 text-xs font-bold leading-none" }, "{rank_label(&card.rank)}" }
                                                                        span { class: if *is_red { "text-red-400 text-xs leading-none" } else { "text-gray-100 text-xs leading-none" }, "{suit_symbol(&card.suit)}" }
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    } else {
                                                        div {
                                                            class: "h-14 flex items-center justify-center px-2",
                                                            span { class: "text-gray-600 text-xs font-bold uppercase tracking-wider", "No cards" }
                                                        }
                                                    }
                                                } else {
                                                    if let Some(ref cards) = player.cards {
                                                        if cards.is_empty() {
                                                            div {
                                                                class: "h-14 flex items-center justify-center px-2",
                                                                span { class: "text-gray-600 text-xs font-bold uppercase tracking-wider", "No cards" }
                                                            }
                                                        } else {
                                                            {
                                                                let up_cards: Vec<_> = if is_stud {
                                                                    sorted_up_cards.iter().map(|c| (suit_is_red(&c.suit), rank_label(&c.rank), suit_symbol(&c.suit))).collect()
                                                                } else {
                                                                    player.cards_up.as_ref()
                                                                        .map(|uc| uc.iter().map(|c| (suit_is_red(&c.suit), rank_label(&c.rank), suit_symbol(&c.suit))).collect())
                                                                        .unwrap_or_default()
                                                                };
                                                                let hole_count = cards.len().saturating_sub(up_cards.len());
                                                                rsx! {
                                                                    for (is_red, rank, suit) in up_cards.iter() {
                                                                        div {
                                                                            class: "w-10 h-14 rounded-lg border-2 border-green-600 bg-gray-800 flex flex-col items-center justify-center",
                                                                            span { class: if *is_red { "text-red-400 text-xs font-bold leading-none" } else { "text-gray-100 text-xs font-bold leading-none" }, "{rank}" }
                                                                            span { class: if *is_red { "text-red-400 text-xs leading-none" } else { "text-gray-100 text-xs leading-none" }, "{suit}" }
                                                                        }
                                                                    }
                                                                    for _ in 0..hole_count {
                                                                        div {
                                                                            class: "w-10 h-14 rounded-lg border-2 border-indigo-900 bg-indigo-950 shadow"
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    } else {
                                                        div {
                                                            class: "h-14 flex items-center justify-center px-2",
                                                            span { class: "text-gray-600 text-xs font-bold uppercase tracking-wider", "No cards" }
                                                        }
                                                    }
                                                }
                                            } else if is_me {
                                                div {
                                                    class: "h-14 flex items-center justify-center px-2",
                                                    span { class: "text-gray-600 text-xs font-bold uppercase tracking-wider", "No cards" }
                                                }
                                            } else if should_show {
    if let Some(ref cards) = player.cards {
        {
            let hide_hole_cards = matches!(phase, Phase::RoundOver)
                && g.winners.get(0).map(|w| w.contains("uncontested")).unwrap_or(false);
            let mut display_cards: Vec<_> = if hide_hole_cards {
                player.cards_up.as_ref()
                    .map(|uc| uc.iter().collect())
                    .unwrap_or_default()
            } else {
                cards.iter().collect()
            };
            if is_stud {
                display_cards.sort_by(|a, b| {
                    card_sort_key(&b.rank, &b.suit)
                        .cmp(&card_sort_key(&a.rank, &a.suit))
                });
            }
            let prepared: Vec<bool> = display_cards.iter().map(|card| suit_is_red(&card.suit)).collect();
                                                        rsx! {
                                                            for (is_red, card) in prepared.iter().zip(display_cards.iter()) {
                                                                div {
                                                                    class: "w-10 h-14 rounded-lg border-2 border-gray-600 bg-gray-800 flex flex-col items-center justify-center",
                                                                    span { class: if *is_red { "text-red-400 text-xs font-bold leading-none" } else { "text-gray-100 text-xs font-bold leading-none" }, "{rank_label(&card.rank)}" }
                                                                    span { class: if *is_red { "text-red-400 text-xs leading-none" } else { "text-gray-100 text-xs leading-none" }, "{suit_symbol(&card.suit)}" }
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            } else if phase == Phase::Ante || (matches!(phase, Phase::Showdown | Phase::RoundOver) && !has_cards) || player.folded || player.left {
                                                div {
                                                    class: "h-14 flex items-center justify-center px-2",
                                                    span { class: "text-gray-600 text-xs font-bold uppercase tracking-wider", "No cards" }
                                                }
                                            } else {
                                                {
                                                    let prepared: Vec<bool> = sorted_up_cards.iter().map(|card| suit_is_red(&card.suit)).collect();
                                                    rsx! {
                                                        for (is_red, card) in prepared.iter().zip(sorted_up_cards.iter()) {
                                                            div {
                                                                class: "w-10 h-14 rounded-lg border-2 border-green-600 bg-gray-800 flex flex-col items-center justify-center",
                                                                span { class: if *is_red { "text-red-400 text-xs font-bold leading-none" } else { "text-gray-100 text-xs font-bold leading-none" }, "{rank_label(&card.rank)}" }
                                                                span { class: if *is_red { "text-red-400 text-xs leading-none" } else { "text-gray-100 text-xs leading-none" }, "{suit_symbol(&card.suit)}" }
                                                            }
                                                        }
                                                        for _ in 0..face_down_count {
                                                            div {
                                                                class: "w-10 h-14 rounded-lg border-2 border-indigo-900 bg-indigo-950 shadow"
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        div { class: "flex gap-3 text-xs text-gray-400",
                                            span { "${player.balance}" }
                                            if player.bet > 0 {
                                                span { class: "text-yellow-500", "Bet: ${player.bet}" }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                if let Some(ref g) = game() {
                    if !g.community_cards.is_empty() {
                        div {
                            class: "flex items-center justify-center gap-3 py-4 border-b border-gray-800 bg-gray-900/30 shrink-0",
                            span { class: "text-gray-500 text-xs uppercase tracking-widest mr-2", "Board" }
                            for card in g.community_cards.iter() {
                                {
                                    let color = if suit_is_red(&card.suit) { "text-red-400" } else { "text-gray-100" };
                                    rsx! {
                                        div {
                                            class: "w-14 h-20 rounded-xl border-2 border-green-700 bg-gray-800 flex flex-col items-center justify-center shadow-lg",
                                            span { class: "{color} text-lg font-bold leading-none", "{rank_label(&card.rank)}" }
                                            span { class: "{color} text-sm leading-none", "{suit_symbol(&card.suit)}" }
                                        }
                                    }
                                }
                            }
                            if matches!(g.game_mode, GameMode::TexasHoldem) {
                                for _ in 0..(5usize.saturating_sub(g.community_cards.len())) {
                                    div {
                                        class: "w-14 h-20 rounded-xl border-2 border-dashed border-gray-700 bg-gray-900/50 flex items-center justify-center",
                                        span { class: "text-gray-700 text-xs", "?" }
                                    }
                                }
                            }
                        }
                    }
                }

                div {
                    class: "flex-1 flex items-center justify-center",
                    if let Some(ref g) = game() {
                        div {
                            class: "w-40 h-20 rounded-full border-2 border-green-900 bg-green-950/40 flex items-center justify-center",
                            span { class: "text-green-400 font-bold text-2xl", "${g.pot}" }
                        }
                    } else {
                        div {
                            class: "flex flex-col items-center gap-3 text-center",
                            div { class: "w-8 h-8 border-2 border-indigo-500 border-t-transparent rounded-full animate-spin" }
                            p { class: "text-gray-600 text-xs uppercase tracking-widest", "Waiting for game..." }
                        }
                    }
                }

                div {
                    class: "relative shrink-0 flex flex-col items-center gap-4 px-8 py-5 bg-gray-900 border-t border-gray-800 min-h-[220px] justify-center",

                    button {
                        class: "absolute bottom-4 left-4 flex items-center gap-1.5 px-3 py-2 bg-gray-800 hover:bg-red-900 border border-gray-700 hover:border-red-700 text-gray-500 hover:text-red-300 rounded-lg text-xs font-semibold uppercase tracking-wider transition-colors duration-150",
                        title: "Leave game",
                        onclick: move |_| {
                            if local_player.is_some() {
                                send("LEAVE".to_string());
                            } else {
                                send_menu("STOP_WATCH".to_string());
                            }
                            screen.set(crate::Screen::MenuScreen);
                        },
                        span { "Leave" }
                    }

                    if let Some(ref local) = local_player {
                        if local.folded || local.left {}
                        else if let Some(ref cards) = local.cards {
                            {
                                let is_stud_bottom = matches!(
                                    game().map(|g| g.game_mode),
                                    Some(GameMode::SevenCardStud)
                                );
                                let display_cards: Vec<_> = if is_stud_bottom {
                                    let mut v: Vec<_> = cards.iter().enumerate().collect();
                                    v.sort_by(|(_, a), (_, b)| {
                                        card_sort_key(&b.rank, &b.suit)
                                            .cmp(&card_sort_key(&a.rank, &a.suit))
                                    });
                                    v
                                } else {
                                    cards.iter().enumerate().collect()
                                };

                                rsx! {
                                    div { class: "flex gap-3",
                                        for (i, card) in display_cards.iter() {
                                            {
                                                let i = *i;
                                                let is_selected = selected_discards().contains(&i);
                                                let color = if suit_is_red(&card.suit) { "text-red-400" } else { "text-gray-100" };
                                                let border = if is_selected {
                                                    "border-yellow-400 bg-gray-700 -translate-y-3"
                                                } else {
                                                    "border-gray-600 bg-gray-800 hover:border-gray-400"
                                                };
                                                let is_draw = phase == Phase::Draw;
                                                rsx! {
                                                    div {
                                                        class: "{card_base} {border}",
                                                        onclick: move |_| {
                                                            if is_draw && is_my_turn {
                                                                let mut d = selected_discards();
                                                                if d.contains(&i) { d.retain(|&x| x != i); }
                                                                else if d.len() < 3 { d.push(i); }
                                                                selected_discards.set(d);
                                                            }
                                                        },
                                                        span { class: "{color} leading-none text-lg font-bold", "{rank_label(&card.rank)}" }
                                                        span { class: "{color} text-sm leading-none", "{suit_symbol(&card.suit)}" }
                                                        if is_selected {
                                                            div {
                                                                class: "absolute -bottom-5 text-yellow-400 text-xs font-bold",
                                                                "Discard"
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        if is_my_turn {
                            div { class: "flex gap-3 mt-2 flex-wrap justify-center",
                                match phase {
                                    Phase::FirstBetting | Phase::SecondBetting
                                    | Phase::PreFlop | Phase::Flop | Phase::Turn | Phase::River
                                    | Phase::ThirdStreet | Phase::FourthStreet | Phase::FifthStreet
                                    | Phase::SixthStreet | Phase::SeventhStreet => {
                                        let to_call = game().map(|g| {
                                            g.current_bet - g.players.get(local_idx).map(|p| p.bet).unwrap_or(0)
                                        }).unwrap_or(0);
                                        let can_check = to_call == 0;
                                        rsx! {
                                            button {
                                                class: "px-5 py-2 bg-red-800 hover:bg-red-700 text-white rounded-lg font-bold text-sm uppercase tracking-wider",
                                                onclick: move |_| send("FOLD".to_string()),
                                                "Fold"
                                            }
                                            if can_check {
                                                button {
                                                    class: "px-5 py-2 bg-gray-700 hover:bg-gray-600 text-white rounded-lg font-bold text-sm uppercase tracking-wider",
                                                    onclick: move |_| send("CHECK".to_string()),
                                                    "Check"
                                                }
                                            } else {
                                                button {
                                                    class: "px-5 py-2 bg-blue-800 hover:bg-blue-700 text-white rounded-lg font-bold text-sm uppercase tracking-wider",
                                                    onclick: move |_| send("CALL".to_string()),
                                                    "Call ${to_call}"
                                                }
                                            }
                                            div { class: "flex rounded-lg overflow-hidden border border-gray-700",
                                                input {
                                                    class: "w-20 px-3 py-2 bg-gray-950 text-gray-100 text-sm focus:outline-none",
                                                    placeholder: "Amount",
                                                    value: "{raise_input}",
                                                    oninput: move |e| raise_input.set(e.value()),
                                                }
                                                button {
                                                    class: "px-4 py-2 bg-indigo-700 hover:bg-indigo-600 text-white font-bold text-sm uppercase tracking-wider border-l border-gray-700",
                                                    onclick: move |_| {
                                                        if let Ok(v) = raise_input().trim().parse::<i32>() {
                                                            if can_check {
                                                                send(format!("BET {}", v));
                                                            } else {
                                                                send(format!("RAISE {}", v));
                                                            }
                                                            raise_input.set(String::new());
                                                        }
                                                    },
                                                    if can_check { "Bet" } else { "Raise" }
                                                }
                                            }
                                            button {
                                                class: "px-4 py-2 bg-gray-600 hover:bg-gray-500 text-white rounded-lg font-bold text-sm uppercase tracking-wider",
                                                onclick: move |_| send("PASS".to_string()),
                                                "Pass"
                                            }
                                        }
                                    },

                                    Phase::Draw => rsx! {
                                        button {
                                            class: "px-6 py-2 bg-indigo-700 hover:bg-indigo-600 text-white rounded-lg font-bold text-sm uppercase tracking-wider",
                                            onclick: move |_| {
                                                let discards = selected_discards();
                                                let cmd = if discards.is_empty() {
                                                    "DRAW".to_string()
                                                } else {
                                                    format!("DRAW {}", discards.iter().map(|i| i.to_string()).collect::<Vec<_>>().join(","))
                                                };
                                                send(cmd);
                                                selected_discards.set(vec![]);
                                            },
                                            "Confirm Draw ({selected_discards().len()} discarded)"
                                        }
                                    },
                                    Phase::Ante => {
                                        rsx! {
                                            button {
                                                class: "px-5 py-2 bg-red-800 hover:bg-red-700 text-white rounded-lg font-bold text-sm uppercase tracking-wider",
                                                onclick: move |_| send("FOLD".to_string()),
                                                "Fold"
                                            }
                                            button {
                                                class: "px-5 py-2 bg-blue-800 hover:bg-blue-700 text-white rounded-lg font-bold text-sm uppercase tracking-wider",
                                                onclick: move |_| send("CALL".to_string()),
                                                "Call"
                                            }
                                        }
                                    },

                                    _ => rsx! { span {} },
                                }
                            }
                        } else if matches!(phase,
                            Phase::Ante| Phase::FirstBetting | Phase::SecondBetting | Phase::Draw
                            | Phase::PreFlop | Phase::Flop | Phase::Turn | Phase::River
                            | Phase::ThirdStreet | Phase::FourthStreet | Phase::FifthStreet
                            | Phase::SixthStreet | Phase::SeventhStreet)
                        {
                            p { class: "text-gray-600 text-xs uppercase tracking-widest mt-1", "Waiting for other players..." }
                        }
                    } else {
                        p { class: "text-gray-500 text-l font-bold uppercase tracking-widest mt-1", "Watching game" }
                    }

                    if matches!(phase, Phase::RoundOver | Phase::GameOver) {
                        if let Some(ref g) = game() {
                            div { class: "flex flex-col items-center gap-3 mt-2",
                                if !g.winners.is_empty() {
                                    p { class: "text-green-400 font-bold text-lg",
                                        "{g.winners[0]}"
                                    }
                                }
                                if phase == Phase::RoundOver && local_player.is_some() {
                                    if let Some(secs) = round_over_secs() {
                                        {
                                            let pct = (secs as f32 / 60.0 * 100.0).min(100.0) as u32;
                                            rsx! {
                                                div { class: "flex flex-col items-center gap-1 w-48",
                                                    div { class: "flex items-center justify-between w-full text-xs",
                                                        span {
                                                            class: "text-gray-400",
                                                            "Respond within"
                                                        }
                                                        span {
                                                            class: "text-gray-200 font-semibold text-base",
                                                            "{secs}s"
                                                        }
                                                    }
                                                    div { class: "w-full h-1.5 rounded-full bg-gray-700 overflow-hidden",
                                                        div {
                                                            class: "h-full rounded-full bg-indigo-500 transition-all duration-1000",
                                                            style: "width: {pct}%",
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    if local_player.is_some() {
                                        div { class: "flex gap-3",
                                            button {
                                                class: if continued() {
                                                    "px-8 py-2 bg-gray-700 text-gray-500 rounded-lg font-bold text-sm uppercase tracking-wider cursor-not-allowed"
                                                } else {
                                                    "px-8 py-2 bg-green-700 hover:bg-green-600 text-white rounded-lg font-bold text-sm uppercase tracking-wider"
                                                },
                                                disabled: continued(),
                                                onclick: move |_| {
                                                    if !continued() {
                                                        continued.set(true);
                                                        round_over_secs.set(None);
                                                        send("CONTINUE".to_string());
                                                    }
                                                },
                                                if continued() { "Waiting for other players..." } else { "Continue to Next Hand" }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    if phase == Phase::Showdown {
                        if let Some(ref g) = game() {
                            if !g.winners.is_empty() {
                                
                                p { class: "text-green-400 font-bold", "{g.winners[0]}" }
                            }
                        }
                    }
                }
            }
        }
    }
}
