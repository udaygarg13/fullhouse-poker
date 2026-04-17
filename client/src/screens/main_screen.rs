use crate::ws::WsHandle;
use crate::Screen;
use chrono::{Datelike, Local, NaiveDateTime};
use common::game_models::GameConfig;
use dioxus::prelude::*;

fn format_datetime(input: &str) -> String {
    let dt = NaiveDateTime::parse_from_str(input, "%Y-%m-%d %H:%M:%S")
        .ok()
        .and_then(|dt| dt.and_local_timezone(Local).single());

    if let Some(dt) = dt {
        let day = dt.day();
        let suffix = match day {
            1 | 21 | 31 => "st",
            2 | 22 => "nd",
            3 | 23 => "rd",
            _ => "th",
        };

        format!(
            "{}{} {}",
            dt.format("%I:%M %p, %-d"),
            suffix,
            dt.format(" %B %Y")
        )
    } else {
        "Invalid date".to_string()
    }
}

#[component]
pub fn MenuScreen(
    ws_handle: Signal<Option<WsHandle>>,
    server_msgs: Signal<Vec<String>>,
    screen: Signal<Screen>,
    username: Signal<String>,
) -> Element {
    let mut active_panel = use_signal(|| "");
    let mut response_msg = use_signal(|| String::new());
    let mut amount_input = use_signal(|| String::new());
    let mut parsed = use_signal(|| (String::new(), String::new()));

    let mut config_ante = use_signal(|| String::new());
    let mut config_max_bet = use_signal(|| String::new());
    let mut config_redraws = use_signal(|| String::new());
    let mut config_sb = use_signal(|| String::new());
    let mut config_bb = use_signal(|| String::new());

    let mut stats_rows: Signal<Vec<(String, i32, i32, i32, i32)>> = use_signal(|| vec![]);
    let mut results_rows: Signal<Vec<(String, String, String, i32)>> = use_signal(|| vec![]);

    let mut queue_host: Signal<String> = use_signal(|| String::new());
    let mut queue_players: Signal<Vec<String>> = use_signal(|| vec![]);
    let mut game_in_progress = use_signal(|| false);
    let mut is_watching = use_signal(|| false);
    let mut watch_pending = use_signal(|| false);
    let mut show_game_ended_modal = use_signal(|| false);

    use_effect(move || {
        let msgs = server_msgs();
        if msgs.is_empty() {
            return;
        }

        for resp in &msgs {
            if resp.starts_with("QUEUE ") {
                if game_in_progress() {
                    continue;
                }
                let parts: Vec<&str> = resp.trim().splitn(3, ' ').collect();
                let host = parts.get(1).copied().unwrap_or("NONE");
                let players_str = parts.get(2).copied().unwrap_or("EMPTY");

                queue_host.set(if host == "NONE" { String::new() } else { host.to_string() });
                queue_players.set(if players_str == "EMPTY" {
                    vec![]
                } else {
                    players_str.split(',').map(|s| s.to_string()).collect()
                });
                continue;
            }

            if resp.trim() == "GAME_IN_PROGRESS" {
                game_in_progress.set(true);
                continue;
            }

            if resp.trim() == "GAME_START" {
                screen.set(Screen::GameScreen);
                let mut msgs_write = server_msgs.write();
                if let Some(pos) = msgs_write.iter().position(|m| m.trim() == "GAME_START") {
                    msgs_write.drain(..=pos);
                }
                return;
            }

            if resp.trim() == "GAME_ENDED" {
                game_in_progress.set(false);
                is_watching.set(false);
                queue_host.set(String::new());
                queue_players.set(vec![]);
                
                show_game_ended_modal.set(true); 
                continue;
            }

            if resp.starts_with("OK") {
                let body = resp.get(3..).unwrap_or("").trim().to_string();
                if body == "Watching" {
                    watch_pending.set(false);
                    is_watching.set(true);
                    screen.set(Screen::GameScreen);
                    continue;
                }
                parsed.set(("OK".to_string(), body));
            } else if resp.starts_with("ERR") {
                let body = resp.get(4..).unwrap_or("").trim().to_string();
                if watch_pending() {
                    watch_pending.set(false);
                }
                parsed.set(("ERR".to_string(), body));
            } else {
                parsed.set(("RAW".to_string(), resp.trim().to_string()));
            }
        }

        server_msgs.write().clear();
    });

    use_effect(move || {
        let (status, body) = parsed();
        if status.is_empty() {
            return;
        }

        match status.as_str() {
            "OK" => match active_panel.peek().as_ref() {
                "stats" => {
                    let rows = body
                        .lines()
                        .filter_map(|line| {
                            let p: Vec<&str> = line.split('|').collect();
                            if p.len() == 5 {
                                Some((
                                    p[0].to_string(),
                                    p[1].parse().unwrap_or(0),
                                    p[2].parse().unwrap_or(0),
                                    p[3].parse().unwrap_or(0),
                                    p[4].parse().unwrap_or(0),
                                ))
                            } else {
                                None
                            }
                        })
                        .collect();
                    stats_rows.set(rows);
                }
                "results" => {
                    let rows = body
                        .lines()
                        .filter_map(|line| {
                            let p: Vec<&str> = line.split('|').collect();
                            if p.len() == 4 {
                                Some((
                                    p[0].to_string(),
                                    p[1].to_string(),
                                    p[2].to_string(),
                                    p[3].parse().unwrap_or(0),
                                ))
                            } else {
                                None
                            }
                        })
                        .collect();
                    results_rows.set(rows);
                }
                "logout" => {
                    screen.set(Screen::LoginScreen);
                    username.set(String::new());
                }
                "rules" => {
                    response_msg.set(body);
                }
                _ => {
                    response_msg.set(body);
                }
            },

            "RAW" => {
                if *active_panel.peek() == "rules" {
                    if let Ok(cfg) = serde_json::from_str::<GameConfig>(&body) {
                        config_ante.set(cfg.ante.to_string());
                        config_max_bet.set(cfg.max_bet.to_string());
                        config_redraws.set(cfg.max_redraws.to_string());
                        config_sb.set(cfg.small_blind.to_string());
                        config_bb.set(cfg.big_blind.to_string());
                    }
                }
            }

            "ERR" => {
                response_msg.set(body);
            }

            _ => {}
        }
    });

    let mut send = move |msg: String| {
        response_msg.set(String::new());
        if let Some(ws) = ws_handle().clone() {
            ws.send(msg);
        }
    };

    let input_class =
        "w-full p-3 rounded-lg border border-gray-800 bg-gray-950 text-gray-100 text-sm";
    let label_class = "block mb-2 text-gray-400 text-xs font-medium uppercase tracking-wider";
    let title_class = "text-gray-200 text-xl font-bold uppercase tracking-widest mb-1";
    let subtitle_class = "text-gray-500 text-xs tracking-widest uppercase mb-8";
    let primary_btn = "w-full py-3 bg-indigo-800 text-gray-100 rounded-lg font-bold text-sm uppercase tracking-wider disabled:opacity-40 disabled:cursor-not-allowed";
    let card_class =
        "bg-gray-900 border border-gray-800 rounded-lg p-6 max-w-sm flex flex-col gap-5";

    rsx! {
        div {
            class: "flex h-screen bg-gray-950 font-sans text-gray-100 overflow-hidden",

            div {
                class: "w-56 bg-gray-900 border-r border-gray-800 flex flex-col shrink-0",

                div {
                    class: "px-6 py-7 border-b border-gray-800",
                    h1 { class: "text-gray-200 text-xl font-bold uppercase tracking-widest mb-0.5", "Poker" }
                    p  { class: "text-gray-500 text-xs tracking-widest uppercase", "Team Fullhouse" }
                }

                div {
                    class: "px-6 py-4 border-b border-gray-800",
                    p { class: "text-gray-500 text-xs uppercase tracking-wider mb-1", "User" }
                    p { class: "text-gray-300 text-sm font-medium truncate", "{username}" }
                }

                nav {
                    class: "flex-1 px-3 py-4 flex flex-col gap-1",
                    for (label, panel) in [
                        ("Play Game", "play"),
                        ("Rules", "rules"),
                        ("Statistics", "stats"),
                        ("Results", "results"),
                        ("Deposit", "deposit"),
                        ("Withdraw", "withdraw"),
                    ] {
                        button {
                            class: if active_panel() == panel {
                                "w-full text-left px-4 py-2.5 rounded-lg bg-gray-800 border border-gray-700 text-gray-100 text-sm font-semibold"
                            } else {
                                "w-full text-left px-4 py-2.5 rounded-lg border border-transparent text-gray-500 text-sm font-medium"
                            },
                            onclick: move |_| {
                                active_panel.set(panel);
                                response_msg.set(String::new());
                                amount_input.set(String::new());
                                if panel == "stats" { send("STATS".to_string()); }
                                if panel == "results" { send("RESULTS".to_string()); }
                                if panel == "rules" { send("RULES".to_string()); }
                            },
                            "{label}"
                        }
                    }
                }

                div {
                    class: "px-3 py-4 border-t border-gray-800",
                    button {
                        class: "w-full text-left px-4 py-2 rounded-lg border border-transparent text-gray-500 text-sm font-medium",
                        onclick: move |_| {
                            active_panel.set("logout");
                            send("LOGOUT".to_string());
                        },
                        "Sign Out"
                    }
                }
            }

            div {
                class: "flex-1 overflow-y-auto p-8 custom-scrollbar",

                match active_panel() {
                    "play" => {
                        let me = username();
                        let host = queue_host();
                        let players = queue_players();
                        let in_progress = game_in_progress();

                        let i_am_host = host == me;
                        let i_am_in_queue = players.contains(&me);
                        let has_host = !host.is_empty();
                        let player_count = players.len();

                        rsx! {
                            div { class: "max-w-md flex flex-col",
                                div {
                                    p { class: "{title_class}", "Play Game" }
                                    p { class: "{subtitle_class}", "Host or join a game" }
                                }

                                div { class: "bg-gray-900 border border-gray-800 rounded-lg p-6 flex flex-col gap-4",
                                    if in_progress {
                                        p { class: "text-gray-600 text-m text-center py-4", "A game is currently in progress." }
                                        div { class: "flex flex-col gap-2 pt-2 border-t border-gray-800",
                                            if !is_watching() {
                                                button {
                                                    class: "w-full py-3 bg-indigo-800 text-gray-100 rounded-lg font-bold text-sm uppercase tracking-wider",
                                                    onclick: move |_| {
                                                        watch_pending.set(true);
                                                        send("WATCH".to_string());
                                                    },
                                                    "Watch"
                                                }
                                            }
                                        }
                                    } else {
                                        if players.is_empty() {
                                            p { class: "text-gray-600 text-m text-center py-4", "No game hosted yet." }
                                        } else {
                                            div { class: "flex flex-col gap-2",
                                                p { class: "text-gray-500 text-xs uppercase tracking-wider mb-1",
                                                    "Players ({player_count})"
                                                }
                                                for player in players.iter() {
                                                    div {
                                                        class: "flex items-center px-3 py-2 rounded-lg bg-gray-800",
                                                        span {
                                                            class: if *player == me {
                                                                "text-green-400 text-sm font-semibold flex-1"
                                                            } else {
                                                                "text-white text-sm flex-1"
                                                            },
                                                            "{player}"
                                                        }
                                                        if *player == host {
                                                            svg {
                                                                class: if *player == me {
                                                                    "w-5 h-5 text-green-400 ml-auto"
                                                                } else {
                                                                    "w-5 h-5 text-white ml-auto"
                                                                },
                                                                xmlns: "http://www.w3.org/2000/svg",
                                                                view_box: "0 0 24 24",
                                                                fill: "none",
                                                                stroke: "currentColor",
                                                                stroke_width: "2",
                                                                stroke_linecap: "round",
                                                                stroke_linejoin: "round",

                                                                path {
                                                                    d: "M11.562 3.266a.5.5 0 0 1 .876 0L15.39 8.87a1 1 0 0 0 1.516.294L21.183 5.5a.5.5 0 0 1 .798.519l-2.834 10.246a1 1 0 0 1-.956.734H5.81a1 1 0 0 1-.957-.734L2.02 6.02a.5.5 0 0 1 .798-.519l4.276 3.664a1 1 0 0 0 1.516-.294z"
                                                                }
                                                                path { d: "M5 21h14" }
                                                            }
                                                        } else {
                                                            svg {
                                                                class: if *player == me {
                                                                    "w-5 h-5 text-green-400 ml-auto"
                                                                } else {
                                                                    "w-5 h-5 text-white ml-auto"
                                                                },
                                                                xmlns: "http://www.w3.org/2000/svg",
                                                                view_box: "0 0 24 24",
                                                                fill: "currentColor",
                                                                stroke: "none",

                                                                circle {
                                                                    cx: "12",
                                                                    cy: "12",
                                                                    r: "6"
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }

                                        div { class: "flex flex-col gap-2 pt-2 border-t border-gray-800",

                                            if !has_host {
                                                button {
                                                    class: "w-full py-3 bg-indigo-800 text-gray-100 rounded-lg font-bold text-sm uppercase tracking-wider",
                                                    onclick: move |_| { send("HOST".to_string()); },
                                                    "Host"
                                                }
                                            }

                                            if has_host && !i_am_in_queue {
                                                button {
                                                    class: "w-full py-3 bg-indigo-800 text-gray-100 rounded-lg font-bold text-sm uppercase tracking-wider",
                                                    onclick: move |_| { send("JOIN".to_string()); },
                                                    "Join"
                                                }
                                            }

                                            if i_am_in_queue && !i_am_host {
                                                button {
                                                    class: "w-full py-3 bg-gray-800 text-gray-400 rounded-lg font-bold text-sm uppercase tracking-wider border border-gray-700",
                                                    onclick: move |_| { send("LEAVE".to_string()); },
                                                    "Leave"
                                                }
                                            }

                                            if i_am_host {
                                                button {
                                                    class: "w-full py-3 bg-gray-800 text-gray-400 rounded-lg font-bold text-sm uppercase tracking-wider border border-gray-700",
                                                    onclick: move |_| { send("LEAVE".to_string()); },
                                                    "Cancel"
                                                }
                                            }

                                            if i_am_host {
                                                button {
                                                    class: if player_count < 2 {
                                                        "w-full py-3 bg-green-900 text-green-600 rounded-lg font-bold text-sm uppercase tracking-wider cursor-not-allowed opacity-50"
                                                    } else {
                                                        "w-full py-3 bg-green-700 text-white rounded-lg font-bold text-sm uppercase tracking-wider"
                                                    },
                                                    disabled: player_count < 2,
                                                    onclick: move |_| { send("START".to_string()); },
                                                    "Start"
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    },

                    "rules" => rsx! {
                        div {
                            p { class: "{title_class}", "House Rules" }
                            p { class: "{subtitle_class}", "View and edit house rules" }
                            div { class: "{card_class}",
                                for (field_label, sig, hint) in [
                                    ("Ante", config_ante.clone(), "e.g. 10"),
                                    ("Max Bet", config_max_bet.clone(), "e.g. 200"),
                                    ("Max Redraws", config_redraws.clone(), "e.g. 3"),
                                    ("Small Blind", config_sb.clone(), "e.g. 10"),
                                    ("Big Blind", config_bb.clone(), "e.g. 20"),
                                ] {
                                    div {
                                        label { class: "{label_class}", "{field_label}" }
                                        input {
                                            class: "{input_class}",
                                            placeholder: "{hint}",
                                            value: "{sig}",
                                            disabled: game_in_progress(),
                                            oninput: move |e| sig.clone().set(e.value()),
                                        }
                                    }
                                }
                                div { class: "border-t border-gray-800 flex flex-col gap-3",
                                    button {
                                        class: if game_in_progress() {
                                            "w-full py-3 bg-gray-700 text-gray-400 rounded-lg font-bold text-sm uppercase tracking-wider cursor-not-allowed opacity-50"
                                        } else {
                                            primary_btn
                                        },
                                        onclick: move |_| {
                                            let cfg = GameConfig {
                                                ante:         config_ante().trim().parse().unwrap_or(0),
                                                max_bet:      config_max_bet().trim().parse().unwrap_or(0),
                                                max_redraws:  config_redraws().trim().parse().unwrap_or(0),
                                                small_blind: config_sb().trim().parse().unwrap_or(0),
                                                big_blind:   config_bb().trim().parse().unwrap_or(0),
                                            };
                                            if let Ok(json) = serde_json::to_string(&cfg) {
                                                send(format!("SAVE_RULES {}", json));
                                            }
                                        },
                                        disabled: game_in_progress(),
                                        "Save Rules"
                                    }
                                    if !response_msg().is_empty() {
                                        p { class: "text-sm text-green-500", "{response_msg}" }
                                    }
                                }
                            }
                        }
                    },

                    "stats" => rsx! {
                        div {
                            p { class: "{title_class}", "Statistics" }
                            p { class: "{subtitle_class}", "Statistics for all players" }
                            div { class: "bg-gray-900 border border-gray-800 rounded-lg overflow-hidden",
                                div { class: "max-h-[500px] overflow-y-auto custom-scrollbar",
                                    table {
                                        class: "w-full text-sm",
                                        thead {
                                            class: "sticky top-0 bg-gray-900 z-10",
                                            tr { class: "border-b border-gray-800",
                                                for col in ["Player", "Balance", "Rounds", "Wins", "Folds"] {
                                                    th { class: "text-left px-5 py-3 text-gray-500 text-xs font-medium uppercase tracking-wider",
                                                        "{col}"
                                                    }
                                                }
                                            }
                                        }
                                        tbody {
                                            for (i, (name, bal, rounds, wins, folds)) in stats_rows().into_iter().enumerate() {
                                                tr {
                                                    class: if i % 2 == 0 {
                                                        "border-b border-gray-800/60"
                                                    } else {
                                                        "border-b border-gray-800/60 bg-gray-800/20"
                                                    },
                                                    td { class: "px-5 py-3 text-gray-200 font-medium", "{name}" }
                                                    td { class: "px-5 py-3 text-gray-300", "${bal}" }
                                                    td { class: "px-5 py-3 text-gray-400", "{rounds}" }
                                                    td { class: "px-5 py-3 text-gray-400", "{wins}" }
                                                    td { class: "px-5 py-3 text-gray-400", "{folds}" }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    },

                    "results" => rsx! {
                        div {
                            p { class: "{title_class}", "Round Results" }
                            p { class: "{subtitle_class}", "History of completed rounds" }
                            div { class: "bg-gray-900 border border-gray-800 rounded-lg overflow-hidden",
                                div { class: "max-h-[500px] overflow-y-auto custom-scrollbar",
                                    table {
                                        class: "w-full text-sm",
                                        thead {
                                            class: "sticky top-0 bg-gray-900 z-10",
                                            tr { class: "border-b border-gray-800",
                                                for col in ["Time", "Winner", "Winning Hand", "Pot"] {
                                                    th { class: "text-left px-5 py-3 text-gray-500 text-xs font-medium uppercase tracking-wider",
                                                        "{col}"
                                                    }
                                                }
                                            }
                                        }
                                        tbody {
                                            for (i, (ended_at, winner, winning_hand, amount_won)) in results_rows().into_iter().enumerate() {
                                                tr {
                                                    class: if i % 2 == 0 {
                                                        "border-b border-gray-800/60"
                                                    } else {
                                                        "border-b border-gray-800/60 bg-gray-800/20"
                                                    },
                                                    td { class: "px-5 py-3 text-gray-300", "{format_datetime(&ended_at)}" }
                                                    td { class: "px-5 py-3 text-gray-200 font-medium", "{winner}" }
                                                    td { class: "px-5 py-3 text-gray-300", "{winning_hand}" }
                                                    td { class: "px-5 py-3 text-gray-300", "${amount_won}" }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    },
                    "deposit" => rsx! {
                        div {
                            p { class: "{title_class}", "Deposit" }
                            p { class: "{subtitle_class}", "Add funds to your account" }
                            div { class: "{card_class}",
                                div {
                                    label { class: "{label_class}", "Amount" }
                                    input {
                                        class: "{input_class}",
                                        placeholder: "e.g. 500",
                                        value: "{amount_input}",
                                        oninput: move |e| amount_input.set(e.value()),
                                    }
                                }
                                button {
                                    class: "{primary_btn}",
                                    onclick: move |_| send(format!("DEPOSIT {}", amount_input().trim())),
                                    "Confirm Deposit"
                                }
                                if !response_msg().is_empty() {
                                    p { class: "text-sm text-green-500", "{response_msg}" }
                                }
                            }
                        }
                    },

                    "withdraw" => rsx! {
                        div {
                            p { class: "{title_class}", "Withdraw" }
                            p { class: "{subtitle_class}", "Remove funds from your account" }
                            div { class: "{card_class}",
                                div {
                                    label { class: "{label_class}", "Amount" }
                                    input {
                                        class: "{input_class}",
                                        placeholder: "e.g. 200",
                                        value: "{amount_input}",
                                        oninput: move |e| amount_input.set(e.value()),
                                    }
                                }
                                button {
                                    class: "{primary_btn}",
                                    onclick: move |_| send(format!("WITHDRAW {}", amount_input().trim())),
                                    "Confirm Withdrawal"
                                }
                                if !response_msg().is_empty() {
                                    p { class: "text-sm text-red-400", "{response_msg}" }
                                }
                            }
                        }
                    },

                    _ => rsx! {
                        div {
                            class: "flex items-center justify-center h-full",
                            p { class: "text-gray-700 text-xs uppercase tracking-widest", "Select an option from the sidebar" }
                        }
                    },
                }
            }
            
        }
    
        if show_game_ended_modal() {
            div {
                class: "fixed inset-0 bg-black/70 flex items-center justify-center z-50 backdrop-blur-sm",
                div {
                    class: "bg-gray-900 border border-gray-800 rounded-xl p-8 max-w-sm w-full shadow-2xl flex flex-col items-center text-center",
                    div { class: "w-16 h-16 bg-red-900/30 rounded-full flex items-center justify-center mb-4",
                        span { class: "text-red-500 text-3xl", "!" }
                    }
                    h3 { class: "text-xl font-bold text-gray-100 mb-2", "Game Over" }
                    p { class: "text-gray-400 text-sm mb-8", "Insufficient players to continue." }
                    button {
                        class: "w-full py-3 bg-indigo-700 hover:bg-indigo-600 text-white rounded-lg font-bold text-sm uppercase tracking-wider transition-colors",
                        onclick: move |_| show_game_ended_modal.set(false),
                        "Dismiss"
                    }
                }
            }
        }
    
    
        if show_game_ended_modal() {
            div {
                class: "fixed inset-0 bg-black/70 flex items-center justify-center z-50 backdrop-blur-sm",
                div {
                    class: "bg-gray-900 border border-gray-800 rounded-xl p-8 max-w-sm w-full shadow-2xl flex flex-col items-center text-center",
                    div { class: "w-16 h-16 bg-red-900/30 rounded-full flex items-center justify-center mb-4",
                        span { class: "text-red-500 text-3xl", "!" }
                    }
                    h3 { class: "text-xl font-bold text-gray-100 mb-2", "Game Over" }
                    p { class: "text-gray-400 text-sm mb-8", "Insufficient players to continue." }
                    button {
                        class: "w-full py-3 bg-indigo-700 hover:bg-indigo-600 text-white rounded-lg font-bold text-sm uppercase tracking-wider transition-colors",
                        onclick: move |_| show_game_ended_modal.set(false),
                        "Dismiss"
                    }
                }
            }
        }
    }
}
