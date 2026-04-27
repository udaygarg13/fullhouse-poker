export enum GameMode {
  FiveCardDraw = "FiveCardDraw",
  TexasHoldem = "TexasHoldem",
  SevenCardStud = "SevenCardStud",
}

export enum Phase {
  Ante = "Ante",
  Showdown = "Showdown",
  RoundOver = "RoundOver",
  GameOver = "GameOver",
  DealerChoice = "DealerChoice",
  FirstBetting = "FirstBetting",
  Draw = "Draw",
  SecondBetting = "SecondBetting",
  SmallBlind = "SmallBlind",
  BigBlind = "BigBlind",
  PreFlop = "PreFlop",
  Flop = "Flop",
  Turn = "Turn",
  River = "River",
  ThirdStreet = "ThirdStreet",
  FourthStreet = "FourthStreet",
  FifthStreet = "FifthStreet",
  SixthStreet = "SixthStreet",
  SeventhStreet = "SeventhStreet",
}

export enum Suit {
  Clubs = "Clubs",
  Diamonds = "Diamonds",
  Hearts = "Hearts",
  Spades = "Spades",
}

export enum Rank {
  Two = "Two",
  Three = "Three",
  Four = "Four",
  Five = "Five",
  Six = "Six",
  Seven = "Seven",
  Eight = "Eight",
  Nine = "Nine",
  Ten = "Ten",
  Jack = "Jack",
  Queen = "Queen",
  King = "King",
  Ace = "Ace",
}

export interface Card {
  rank: Rank;
  suit: Suit;
}

export interface PlayerMsg {
  name: string;
  balance: number;
  bet: number;
  folded: boolean;
  left: boolean;
  is_dealer: boolean;
  cards?: Card[];
  cards_up?: Card[];
}

export interface GameStateMsg {
  phase: Phase;
  game_mode: GameMode;
  pot: number;
  current_bet: number;
  hand_number: number;
  dealer_idx: number;
  active_player_idx: number;
  message: string;
  winners: string[];
  players: PlayerMsg[];
  community_cards: Card[];
}

export interface GameConfig {
  ante: number;
  max_bet: number;
  max_redraws: number;
  small_blind: number;
  big_blind: number;
}

export type Screen = 'ConnectScreen' | 'LoginScreen' | 'MenuScreen' | 'GameScreen';
