import {
  useState,
  useEffect,
  useRef,
  type Dispatch,
  type SetStateAction,
} from "react";
import type { Screen, GameStateMsg } from "../types";
import { Phase, GameMode, Rank, Suit } from "../types";
import { Input } from "@/components/ui/input";
import { cn } from "@/lib/utils";
import {
  Eye,
  Loader2,
  Timer,
  ChevronLeft,
  ChevronRight,
  LogOut,
} from "lucide-react";

interface GameScreenProps {
  wsHandle: { send: (message: string) => void } | null;
  serverMessages: string[];
  setServerMessages: Dispatch<SetStateAction<string[]>>;
  username: string;
  setScreen: (screen: Screen) => void;
}

const RANK_LABEL_MAP: Record<Rank, string> = {
  Two: "2", Three: "3", Four: "4", Five: "5", Six: "6", Seven: "7",
  Eight: "8", Nine: "9", Ten: "10", Jack: "J", Queen: "Q", King: "K", Ace: "A",
};

const SUIT_SYMBOL_MAP: Record<Suit, string> = {
  Spades: "\u2660", Hearts: "\u2665", Diamonds: "\u2666", Clubs: "\u2663",
};

const RANK_SORT_MAP: Record<Rank, number> = {
  Two: 2, Three: 3, Four: 4, Five: 5, Six: 6, Seven: 7, Eight: 8,
  Nine: 9, Ten: 10, Jack: 11, Queen: 12, King: 13, Ace: 14,
};

const SUIT_SORT_MAP: Record<Suit, number> = {
  Spades: 0, Hearts: 1, Diamonds: 2, Clubs: 3,
};

const PHASE_LABEL_MAP: Record<Phase, string> = {
  Ante: "Ante", Showdown: "Showdown", RoundOver: "Round Over",
  GameOver: "Game Over", DealerChoice: "Dealer's Choice",
  FirstBetting: "1st Bet", Draw: "Draw", SecondBetting: "2nd Bet",
  SmallBlind: "Small Blind", BigBlind: "Big Blind", PreFlop: "Pre-Flop",
  Flop: "Flop", Turn: "Turn", River: "River", ThirdStreet: "3rd St",
  FourthStreet: "4th St", FifthStreet: "5th St", SixthStreet: "6th St",
  SeventhStreet: "7th St",
};

const GAME_MODE_LABEL_MAP: Record<GameMode, string> = {
  FiveCardDraw: "Five-Card Draw",
  TexasHoldem: "Texas Hold'em",
  SevenCardStud: "Seven-Card Stud",
};

const ACTIVE_PHASES: Phase[] = [
  Phase.Ante, Phase.FirstBetting, Phase.SecondBetting, Phase.Draw,
  Phase.PreFlop, Phase.Flop, Phase.Turn, Phase.River, Phase.ThirdStreet,
  Phase.FourthStreet, Phase.FifthStreet, Phase.SixthStreet, Phase.SeventhStreet,
  Phase.DealerChoice,
];

const BETTING_PHASES: Phase[] = [
  Phase.FirstBetting, Phase.SecondBetting, Phase.PreFlop, Phase.Flop,
  Phase.Turn, Phase.River, Phase.ThirdStreet, Phase.FourthStreet,
  Phase.FifthStreet, Phase.SixthStreet, Phase.SeventhStreet,
];

const rankLabel = (rank: Rank): string => RANK_LABEL_MAP[rank];
const suitSymbol = (suit: Suit): string => SUIT_SYMBOL_MAP[suit];
const suitIsRed = (suit: Suit): boolean => suit === Suit.Hearts || suit === Suit.Diamonds;
const phaseLabel = (phase: Phase): string => PHASE_LABEL_MAP[phase];
const gameModeLabel = (mode: GameMode): string => GAME_MODE_LABEL_MAP[mode];

const cardSortKey = (rank: Rank, suit: Suit): [number, number] => [
  RANK_SORT_MAP[rank],
  SUIT_SORT_MAP[suit],
];

const parseRoundOverTimeout = (msg: string): number | undefined => {
  const marker = "You have ";
  const pos = msg.indexOf(marker);
  if (pos === -1) return undefined;
  const rest = msg.slice(pos + marker.length);
  const end = rest.indexOf("s");
  if (end === -1) return undefined;
  return parseInt(rest.slice(0, end).trim(), 10);
};

const CARD_SIZE_CLASSES = {
  tiny: "w-10 h-14 text-xs rounded-lg",
  small: "w-14 h-20 text-base rounded-xl",
  normal: "w-14 h-20 text-xl rounded-xl",
  compact: "w-9 h-12 text-[10px] rounded-md",
};

interface PlayingCardProps {
  rank?: Rank;
  suit?: Suit;
  selected?: boolean;
  onClick?: () => void;
  size?: "tiny" | "small" | "normal" | "compact";
  faceDown?: boolean;
  showDiscard?: boolean;
}

const PlayingCard = ({
  rank,
  suit,
  selected = false,
  onClick,
  size = "normal",
  faceDown = false,
  showDiscard = false,
}: PlayingCardProps) => {
  if (faceDown) {
    return (
      <div
        className={cn(
          CARD_SIZE_CLASSES[size],
          "border-2 border-indigo-900 bg-indigo-950 shadow shrink-0",
        )}
      />
    );
  }

  if (!rank || !suit) return null;

  const isRed = suitIsRed(suit);

  return (
    <div
      onClick={onClick}
      className={cn(
        CARD_SIZE_CLASSES[size],
        "relative border-2 flex flex-col items-center justify-center font-bold shadow-lg",
        "transition-all duration-150 select-none shrink-0",
        "bg-gradient-to-br from-white to-zinc-100",
        selected
          ? "border-yellow-400 -translate-y-3 shadow-lg"
          : "border-zinc-300 hover:border-zinc-400",
        onClick && "cursor-pointer",
      )}
    >
      <span
        className={cn(
          "leading-none font-bold",
          size === "compact" ? "text-[11px]" : "text-lg",
          isRed ? "text-red-600" : "text-zinc-900",
        )}
      >
        {rankLabel(rank)}
      </span>
      <span
        className={cn(
          "leading-none",
          size === "compact" ? "text-[10px]" : size === "tiny" ? "text-sm" : "text-base",
          isRed ? "text-red-600" : "text-zinc-900",
        )}
      >
        {suitSymbol(suit)}
      </span>
      {showDiscard && selected && (
        <div className="absolute -bottom-5 text-yellow-400 text-[10px] font-bold">
          Discard
        </div>
      )}
    </div>
  );
};

interface PlayerBadgeProps {
  player: GameStateMsg["players"][0];
  idx: number;
  me: string;
  game: GameStateMsg;
  phase: Phase;
  gameMode: GameMode;
  isStud: boolean;
}

const renderPlayerCards = (
  player: GameStateMsg["players"][0],
  isMe: boolean,
  phase: Phase,
  gameMode: GameMode,
  isStud: boolean,
) => {
  const noCardsEl = (
    <span className="text-zinc-600 text-xs font-bold uppercase tracking-wider text-center w-full">
      No cards
    </span>
  );

  const hasCards = !!(player.cards && player.cards.length > 0);
  const shouldShow =
    (phase === Phase.Showdown || phase === Phase.RoundOver) &&
    hasCards &&
    (!player.folded || isStud);

  const totalHandSize = (() => {
    if (gameMode === GameMode.FiveCardDraw) return 5;
    if (gameMode === GameMode.TexasHoldem) return 2;
    const studSizes: Partial<Record<Phase, number>> = {
      [Phase.ThirdStreet]: 3,
      [Phase.FourthStreet]: 4,
      [Phase.FifthStreet]: 5,
      [Phase.SixthStreet]: 6,
    };
    return studSizes[phase] ?? 7;
  })();

  const upCards = [...(player.cards_up ?? [])].sort((a, b) => {
    const [ar, as_] = cardSortKey(a.rank, a.suit);
    const [br, bs] = cardSortKey(b.rank, b.suit);
    return br !== ar ? br - ar : bs - as_;
  });

  const faceDownCount = Math.max(0, totalHandSize - upCards.length);

  if (isMe) {
    if (player.folded || player.left) return noCardsEl;
    if (shouldShow) {
      const display = isStud
        ? [...player.cards!].sort((a, b) => {
            const [ar, as_] = cardSortKey(a.rank, a.suit);
            const [br, bs] = cardSortKey(b.rank, b.suit);
            return br !== ar ? br - ar : bs - as_;
          })
        : player.cards!;
      return (
        <>
          {display.map((c, i) => (
            <PlayingCard key={i} rank={c.rank} suit={c.suit} size="tiny" />
          ))}
        </>
      );
    }
    if (!hasCards) return noCardsEl;
    const localHoleCount = Math.max(0, player.cards!.length - upCards.length);
    if (upCards.length === 0 && localHoleCount === 0) {
      return (
        <>
          {player.cards!.map((c, i) => (
            <PlayingCard key={i} rank={c.rank} suit={c.suit} size="tiny" />
          ))}
        </>
      );
    }
    return (
      <>
        {upCards.map((c, i) => (
          <PlayingCard key={`u-${i}`} rank={c.rank} suit={c.suit} size="tiny" />
        ))}
        {Array.from({ length: localHoleCount }).map((_, i) => (
          <PlayingCard key={`h-${i}`} faceDown size="tiny" />
        ))}
      </>
    );
  }

  if (phase === Phase.Ante || player.folded || player.left || (shouldShow && !hasCards)) {
    return noCardsEl;
  }

  if (shouldShow) {
    const isUncontested = false;
    if (isUncontested) {
      if (upCards.length === 0) return noCardsEl;
      return (
        <>
          {upCards.map((c, i) => (
            <PlayingCard key={i} rank={c.rank} suit={c.suit} size="tiny" />
          ))}
        </>
      );
    }
    const display = isStud
      ? [...player.cards!].sort((a, b) => {
          const [ar, as_] = cardSortKey(a.rank, a.suit);
          const [br, bs] = cardSortKey(b.rank, b.suit);
          return br !== ar ? br - ar : bs - as_;
        })
      : player.cards!;
    return (
      <>
        {display.map((c, i) => (
          <PlayingCard key={i} rank={c.rank} suit={c.suit} size="tiny" />
        ))}
      </>
    );
  }

  if (upCards.length === 0 && faceDownCount === 0) return noCardsEl;

  return (
    <>
      {upCards.map((c, i) => (
        <PlayingCard key={i} rank={c.rank} suit={c.suit} size="tiny" />
      ))}
      {Array.from({ length: faceDownCount }).map((_, i) => (
        <PlayingCard key={`d-${i}`} faceDown size="tiny" />
      ))}
    </>
  );
};

const PlayerBadge = ({ player, idx, me, game, phase, gameMode, isStud }: PlayerBadgeProps) => {
  const isMe = player.name === me;
  const isActiveTurn =
    game.active_player_idx === idx &&
    ACTIVE_PHASES.includes(phase) &&
    phase !== Phase.DealerChoice;

  const playerCount = game.players.length;
  const isSmallBlind =
    gameMode === GameMode.TexasHoldem &&
    playerCount > 1 &&
    idx === ((game.dealer_idx ?? 0) + 1) % playerCount;
  const isBigBlind =
    gameMode === GameMode.TexasHoldem &&
    playerCount > 2 &&
    idx === ((game.dealer_idx ?? 0) + 2) % playerCount;

  const containerCls = cn(
    "flex flex-col items-center gap-2 p-3 rounded-xl border-2 transition-all shrink-0 min-w-[145px]",
    (player.folded || player.left) && "opacity-50",
    isMe && isActiveTurn && "border-green-500 bg-zinc-900 ring-2 ring-green-500/30",
    isMe && !isActiveTurn && "border-green-800 bg-zinc-900",
    !isMe && isActiveTurn && "border-indigo-600 bg-zinc-900",
    !isMe && !isActiveTurn && "border-zinc-800 bg-zinc-900",
  );

  return (
    <div className={containerCls}>
      <div className="flex items-center gap-1.5 flex-wrap justify-center">
        <span className={cn("font-semibold text-sm", isMe ? "text-green-300" : "text-zinc-200")}>
          {player.name}
        </span>
        {isMe && (
          <span className="text-xs bg-green-900 text-green-300 px-1.5 py-0.5 rounded font-bold">
            You
          </span>
        )}
        {player.is_dealer && (
          <span className="text-xs bg-yellow-700 text-yellow-200 px-1.5 py-0.5 rounded font-bold">
            D
          </span>
        )}
        {isSmallBlind && (
          <span className="text-xs bg-blue-800 text-blue-200 px-1.5 py-0.5 rounded font-bold">
            SB
          </span>
        )}
        {isBigBlind && (
          <span className="text-xs bg-purple-800 text-purple-200 px-1.5 py-0.5 rounded font-bold">
            BB
          </span>
        )}
        {player.folded && (
          <span className="text-xs bg-red-900 text-red-300 px-1.5 py-0.5 rounded">F</span>
        )}
        {player.left && (
          <span className="text-xs bg-red-900 text-red-300 px-1.5 py-0.5 rounded">L</span>
        )}
      </div>
      <div className="flex gap-1 min-h-[56px] items-center justify-center flex-wrap">
        {renderPlayerCards(player, isMe, phase, gameMode, isStud)}
      </div>
      <div className="flex gap-3 text-xs text-zinc-400">
        <span>${player.balance}</span>
        {player.bet > 0 && (
          <span className="text-yellow-500">Bet: ${player.bet}</span>
        )}
      </div>
    </div>
  );
};

export const GameScreen = ({
  wsHandle,
  serverMessages,
  setServerMessages,
  username,
  setScreen,
}: GameScreenProps) => {
  const [game, setGame] = useState<GameStateMsg | null>(null);
  const [selectedDiscards, setSelectedDiscards] = useState<number[]>([]);
  const [raiseInput, setRaiseInput] = useState("");
  const [status, setStatus] = useState("Waiting for server...");
  const [continued, setContinued] = useState(false);
  const [roundOverSecs, setRoundOverSecs] = useState<number | undefined>(undefined);
  const [mobilePlayerIdx, setMobilePlayerIdx] = useState(0);

  const roundOverInitialSecs = useRef<number>(60);
  const gamePhasRef = useRef<Phase | undefined>(undefined);

  useEffect(() => {
    const interval = setInterval(() => {
      setRoundOverSecs((prev) => {
        if (prev === undefined || prev <= 0) return prev;
        return prev - 1;
      });
    }, 1000);
    return () => clearInterval(interval);
  }, []);

  useEffect(() => {
    if (serverMessages.length === 0) return;
    const snapshotLength = serverMessages.length;
    let transitionIndex = -1;

    for (let i = 0; i < serverMessages.length; i++) {
      const msg = serverMessages[i];
      if (msg.trim() === "GAME_START") continue;
      if (msg.trim() === "GAME_ENDED") { transitionIndex = i; break; }
      if (msg.startsWith("GAME_LEFT")) { transitionIndex = i; break; }
      if (msg.startsWith("GAME_STATE ")) {
        const rest = msg.trim().slice("GAME_STATE ".length);
        try {
          const state = JSON.parse(rest) as GameStateMsg;
          setStatus(state.message);
          if (gamePhasRef.current !== state.phase) setSelectedDiscards([]);
          gamePhasRef.current = state.phase;
          if (state.phase === Phase.RoundOver) {
            const secs = parseRoundOverTimeout(state.message);
            if (secs !== undefined) {
              roundOverInitialSecs.current = secs;
              setRoundOverSecs(secs);
            }
          } else {
            setRoundOverSecs(undefined);
          }
          if (state.phase !== Phase.RoundOver) setContinued(false);
          setGame(state);
          if (
            state.active_player_idx !== undefined &&
            ACTIVE_PHASES.includes(state.phase) &&
            state.phase !== Phase.DealerChoice
          ) {
            setMobilePlayerIdx(state.active_player_idx);
          }
        } catch (e) {
          setStatus(`ERR parsing state: ${e}`);
        }
        continue;
      }
      if (msg.startsWith("ERR")) setStatus(msg);
    }

    if (transitionIndex >= 0) {
      setServerMessages((prev) => prev.slice(Math.min(transitionIndex, prev.length)));
      setScreen("MenuScreen");
    } else {
      setServerMessages((prev) => prev.slice(Math.min(snapshotLength, prev.length)));
    }
  }, [serverMessages, setServerMessages]);

  const send = (cmd: string) => wsHandle?.send(`GAME ${cmd}`);
  const sendMenu = (cmd: string) => wsHandle?.send(cmd);

  const me = username;
  const localPlayer = game?.players.find((p) => p.name === me);
  const localIdx = game?.players.findIndex((p) => p.name === me) ?? -1;
  const phase = game?.phase ?? Phase.Ante;
  const gameMode = game?.game_mode ?? GameMode.FiveCardDraw;
  const isStud = gameMode === GameMode.SevenCardStud;
  const isMyTurn = !!(
    localPlayer &&
    localIdx >= 0 &&
    game?.active_player_idx === localIdx &&
    !localPlayer.folded &&
    ACTIVE_PHASES.includes(phase)
  );
  const isDealer = game?.dealer_idx === localIdx;
  const toCall = game ? game.current_bet - (game.players[localIdx]?.bet ?? 0) : 0;

  const handleLeave = () => {
    if (localPlayer) send("LEAVE");
    else sendMenu("STOP_WATCH");
    setScreen("MenuScreen");
  };

  const localHandCards = (() => {
    const cards = localPlayer?.cards;
    if (!cards || cards.length === 0) return [];
    return cards.map((card, originalIdx) => ({ card, originalIdx, isFaceUp: true }));
  })();

  return (
    <div className="relative flex flex-col min-h-[100dvh] bg-zinc-950 overflow-hidden">
      <header className="bg-zinc-900 border-b border-zinc-800 shrink-0">
        <div className="hidden md:grid grid-cols-[1fr_auto_1fr] items-center px-3 py-3">
          <div className="flex items-center gap-3 justify-start">
            {game && (
              <span className="text-xs px-2 py-0.5 rounded-sm bg-zinc-800 text-blue-400 font-bold">
                {gameModeLabel(game.game_mode)}
              </span>
            )}
            <span className="text-zinc-200 text-sm font-semibold">{phaseLabel(phase)}</span>
          </div>
          <div className="flex items-center justify-center gap-6">
            <div className="text-zinc-200 font-semibold text-sm">{status}</div>
          </div>
          <div className="flex items-center justify-end">
            <button
              className="p-1.5 rounded-lg bg-zinc-800 hover:bg-red-900 border border-zinc-700 hover:border-red-700 text-zinc-500 hover:text-red-300 transition-colors"
              onClick={handleLeave}
            >
              <LogOut className="w-4 h-4" />
            </button>
          </div>
        </div>
        <div className="md:hidden flex flex-col px-2 md:px-4 py-2 gap-2">
          <div className="relative flex items-center justify-center gap-2 h-6">
            <button
              className="absolute right-0 top-3 p-1.5 rounded-lg bg-zinc-800 hover:bg-red-900 border border-zinc-700 hover:border-red-700 text-zinc-500 hover:text-red-300 transition-colors"
              onClick={handleLeave}
            >
              <LogOut className="w-4 h-4" />
            </button>
            {game && (
              <span className="text-xs px-1.5 py-0.5 rounded-sm bg-zinc-800 text-blue-400 font-bold">
                {gameModeLabel(game.game_mode)}
              </span>
            )}
            <span className="text-zinc-200 text-sm font-semibold">{phaseLabel(phase)}</span>
          </div>
          <div className="h-5 flex items-center justify-center">
            <span className="text-zinc-200 text-sm font-semibold truncate text-center">{status}</span>
          </div>
        </div>
      </header>

      {phase === Phase.DealerChoice ? (
        <div className="flex-1 flex flex-col items-center justify-center gap-3 p-8">
          <p className="text-zinc-400 text-sm uppercase tracking-widest">Dealer&apos;s Choice</p>
          {isDealer && isMyTurn ? (
            <>
              <p className="text-zinc-200 text-base font-semibold text-center">
                Choose the game variant for this hand:
              </p>
              <div className="flex flex-col md:flex-row gap-3 md:gap-4 w-full max-w-xs md:max-w-none md:w-auto">
                <button
                  className="px-6 py-3 bg-zinc-700 hover:bg-zinc-600 text-white rounded-lg font-bold text-sm uppercase tracking-wider"
                  onClick={() => send("MODE FiveCardDraw")}
                >
                  Five-Card Draw
                </button>
                <button
                  className="px-6 py-3 bg-zinc-700 hover:bg-zinc-600 text-white rounded-lg font-bold text-sm uppercase tracking-wider"
                  onClick={() => send("MODE TexasHoldem")}
                >
                  Texas Hold&apos;em
                </button>
                <button
                  className="px-6 py-3 bg-zinc-700 hover:bg-zinc-600 text-white rounded-lg font-bold text-sm uppercase tracking-wider"
                  onClick={() => send("MODE SevenCardStud")}
                >
                  Seven-Card Stud
                </button>
              </div>
            </>
          ) : (
            game && (
              <p className="text-zinc-500 text-sm">
                Waiting for {game.players[game.dealer_idx].name} to choose…
              </p>
            )
          )}
        </div>
      ) : (
        <div className="flex-1 flex flex-col overflow-hidden">
          {game && (
            <div className="md:hidden border-b border-zinc-800 shrink-0 px-3 py-3">
              <div className="flex justify-center">
                <PlayerBadge
                  player={game.players[mobilePlayerIdx]}
                  idx={mobilePlayerIdx}
                  me={me}
                  game={game}
                  phase={phase}
                  gameMode={gameMode}
                  isStud={isStud}
                />
              </div>
              <div className="flex items-center justify-center gap-4 mt-2">
                <button
                  className="p-1.5 rounded-lg bg-zinc-800 hover:bg-zinc-700 text-zinc-400 hover:text-zinc-200 transition-colors shrink-0"
                  onClick={() =>
                    setMobilePlayerIdx((p) => (p - 1 + game.players.length) % game.players.length)
                  }
                >
                  <ChevronLeft className="w-4 h-4" />
                </button>
                <div className="flex gap-1.5">
                  {game.players.map((_, i) => (
                    <button
                      key={i}
                      onClick={() => setMobilePlayerIdx(i)}
                      className={cn(
                        "w-1.5 h-1.5 rounded-full transition-colors",
                        i === mobilePlayerIdx ? "bg-blue-400" : "bg-zinc-700 hover:bg-zinc-500",
                      )}
                    />
                  ))}
                </div>
                <button
                  className="p-1.5 rounded-lg bg-zinc-800 hover:bg-zinc-700 text-zinc-400 hover:text-zinc-200 transition-colors shrink-0"
                  onClick={() => setMobilePlayerIdx((p) => (p + 1) % game.players.length)}
                >
                  <ChevronRight className="w-4 h-4" />
                </button>
              </div>
            </div>
          )}

          {game && (
            <div
              className="hidden md:flex gap-6 px-5 py-5 bg-zinc-900/50 border-b border-zinc-800 overflow-x-auto shrink-0"
              style={{ justifyContent: "safe center" }}
            >
              {game.players.map((player, idx) => (
                <PlayerBadge
                  key={player.name}
                  player={player}
                  idx={idx}
                  me={me}
                  game={game}
                  phase={phase}
                  gameMode={gameMode}
                  isStud={isStud}
                />
              ))}
            </div>
          )}

          {game && game.community_cards.length > 0 && (
            <div className="flex items-center justify-center gap-3 py-4 border-b border-zinc-800 bg-zinc-900/30 shrink-0">
              <div className="flex items-center justify-center gap-3 md:hidden">
                {game.community_cards.map((card, i) => (
                  <PlayingCard key={i} rank={card.rank} suit={card.suit} size="tiny" />
                ))}
                {gameMode === GameMode.TexasHoldem &&
                  Array.from({ length: 5 - game.community_cards.length }).map((_, i) => (
                    <div
                      key={i}
                      className="w-10 h-14 rounded-xl border-2 border-dashed border-zinc-700 bg-zinc-900/50 flex items-center justify-center"
                    >
                      <span className="text-zinc-700 text-xs">?</span>
                    </div>
                  ))}
              </div>
              <div className="hidden md:flex items-center justify-center gap-3">
                {game.community_cards.map((card, i) => (
                  <PlayingCard key={i} rank={card.rank} suit={card.suit} size="small" />
                ))}
                {gameMode === GameMode.TexasHoldem &&
                  Array.from({ length: 5 - game.community_cards.length }).map((_, i) => (
                    <div
                      key={i}
                      className="w-14 h-20 rounded-xl border-2 border-dashed border-zinc-700 bg-zinc-900/50 flex items-center justify-center"
                    >
                      <span className="text-zinc-700 text-xs">?</span>
                    </div>
                  ))}
              </div>
            </div>
          )}

          <div className="flex-1 flex items-center justify-center p-4">
            {game ? (
              <div className="flex flex-col items-center">
                <div className="w-30 h-15 md:w-40 md:h-20 rounded-full border-2 border-green-900 bg-green-950/40 flex flex-col items-center justify-center gap-1">
                  <span className="text-green-400 font-bold text-2xl">${game.pot}</span>
                </div>
                <span className="text-zinc-200 font-bold text-sm text-center mt-2">
                  Current Bet ${game.current_bet}
                </span>
              </div>
            ) : (
              <div className="flex flex-col items-center gap-3">
                <Loader2 className="w-8 h-8 animate-spin text-blue-500" />
                <p className="text-zinc-600 text-xs uppercase tracking-widest">
                  Waiting for game...
                </p>
              </div>
            )}
          </div>

          <div className="relative shrink-0 flex flex-col items-center gap-4 px-8 py-5 bg-zinc-900 border-t border-zinc-800 h-[160px] md:min-h-[250px] justify-center">
            {localPlayer &&
              !localPlayer.folded &&
              !localPlayer.left &&
              localHandCards.length > 0 &&
              phase !== Phase.Showdown &&
              phase !== Phase.RoundOver && (
                <div className="flex gap-3">
                  {localHandCards.map(({ card, originalIdx, isFaceUp }) => {
                    if (!isFaceUp) {
                      return (
                        <>
                          <div key={`m-${originalIdx}`} className="md:hidden">
                            <PlayingCard faceDown size="tiny" />
                          </div>
                          <div key={`d-${originalIdx}`} className="hidden md:block">
                            <PlayingCard faceDown size="normal" />
                          </div>
                        </>
                      );
                    }
                    const isSelected = selectedDiscards.includes(originalIdx);
                    const isDraw = phase === Phase.Draw;
                    const handleClick =
                      isDraw && isMyTurn
                        ? () => {
                            setSelectedDiscards((prev) => {
                              if (prev.includes(originalIdx))
                                return prev.filter((x) => x !== originalIdx);
                              if (prev.length < 3) return [...prev, originalIdx];
                              return prev;
                            });
                          }
                        : undefined;
                    return (
                      <>
                        <div key={`m-${originalIdx}`} className="md:hidden">
                          <PlayingCard
                            rank={card.rank}
                            suit={card.suit}
                            selected={isSelected}
                            showDiscard={isDraw}
                            size="tiny"
                            onClick={handleClick}
                          />
                        </div>
                        <div key={`d-${originalIdx}`} className="hidden md:block">
                          <PlayingCard
                            rank={card.rank}
                            suit={card.suit}
                            selected={isSelected}
                            showDiscard={isDraw}
                            size="normal"
                            onClick={handleClick}
                          />
                        </div>
                      </>
                    );
                  })}
                </div>
              )}

            {isMyTurn && (
              <div className="flex gap-2 mt-2 flex-nowrap overflow-x-auto justify-start md:justify-center">
                {BETTING_PHASES.includes(phase) && (
                  <>
                    <button
                      className="px-3 py-1.5 md:px-5 md:py-2 bg-red-800 hover:bg-red-700 text-white rounded-lg font-bold text-xs md:text-sm uppercase tracking-wider whitespace-nowrap"
                      onClick={() => send("FOLD")}
                    >
                      Fold
                    </button>
                    {toCall === 0 ? (
                      <button
                        className="px-3 py-1.5 md:px-5 md:py-2 bg-zinc-700 hover:bg-zinc-600 text-white rounded-lg font-bold text-xs md:text-sm uppercase tracking-wider whitespace-nowrap"
                        onClick={() => send("CHECK")}
                      >
                        Check
                      </button>
                    ) : (
                      <button
                        className="px-3 py-1.5 md:px-5 md:py-2 bg-blue-800 hover:bg-blue-700 text-white rounded-lg font-bold text-xs md:text-sm uppercase tracking-wider whitespace-nowrap"
                        onClick={() => send("CALL")}
                      >
                        Call ${toCall}
                      </button>
                    )}
                    <div className="flex rounded-lg overflow-hidden border border-zinc-700 shrink-0">
                      <Input
                        className="w-16 md:w-20 px-2 py-1.5 md:px-3 md:py-2 bg-zinc-950 text-zinc-100 text-xs md:text-sm rounded-none border-0 focus-visible:ring-0 h-auto"
                        placeholder="Amount"
                        value={raiseInput}
                        onChange={(e) => setRaiseInput(e.target.value)}
                      />
                      <button
                        className="px-3 md:px-4 py-1.5 md:py-2 bg-blue-700 hover:bg-blue-600 text-white font-bold text-xs md:text-sm uppercase tracking-wider border-l border-zinc-700 whitespace-nowrap"
                        onClick={() => {
                          const v = parseInt(raiseInput.trim(), 10);
                          if (!isNaN(v)) {
                            send(toCall === 0 ? `BET ${v}` : `RAISE ${v}`);
                            setRaiseInput("");
                          }
                        }}
                      >
                        {toCall === 0 ? "Bet" : "Raise"}
                      </button>
                    </div>
                  </>
                )}

                {phase === Phase.Ante && (
                  <>
                    <button
                      className="px-3 py-1.5 md:px-5 md:py-2 bg-red-800 hover:bg-red-700 text-white rounded-lg font-bold text-xs md:text-sm uppercase tracking-wider whitespace-nowrap"
                      onClick={() => send("FOLD")}
                    >
                      Fold
                    </button>
                    <button
                      className="px-3 py-1.5 md:px-5 md:py-2 bg-blue-800 hover:bg-blue-700 text-white rounded-lg font-bold text-xs md:text-sm uppercase tracking-wider whitespace-nowrap"
                      onClick={() => send("CALL")}
                    >
                      Call Ante
                    </button>
                  </>
                )}

                {phase === Phase.Draw && (
                  <button
                    className="px-3 py-1.5 md:px-5 md:py-2 bg-green-700 hover:bg-green-600 text-white rounded-lg font-bold text-xs md:text-sm uppercase tracking-wider whitespace-nowrap"
                    onClick={() => {
                      const discardStr =
                        selectedDiscards.length > 0 ? selectedDiscards.join(",") : "none";
                      send(`DRAW ${discardStr}`);
                      setSelectedDiscards([]);
                    }}
                  >
                    Redraw
                  </button>
                )}
              </div>
            )}

            {!isMyTurn && [...BETTING_PHASES, Phase.Ante, Phase.Draw].includes(phase) && (
              <p className="text-zinc-600 text-xs uppercase tracking-widest mt-2 md:mb-2 py-1.75">
                Waiting for other players...
              </p>
            )}

            {!localPlayer && (
              <p className="text-zinc-500 text-lg font-bold uppercase tracking-widest mt-1 flex items-center gap-2">
                <Eye className="w-5 h-5" />
                Watching game
              </p>
            )}

            {(phase === Phase.RoundOver || phase === Phase.GameOver) && game && (
              <div className="flex flex-col items-center gap-3 mt-2">
                {game.winners.length > 0 && (
                  <p className="text-green-400 font-bold text-sm md:text-lg flex items-center gap-2">
                    {game.winners[0]}
                  </p>
                )}
                {phase === Phase.RoundOver && localPlayer && (
                  <>
                    {!continued && roundOverSecs !== undefined ? (
                      <div className="flex flex-col items-center gap-1 w-48">
                        <div className="flex items-center justify-between w-full text-xs">
                          <span className="text-zinc-400 flex items-center gap-1">
                            <Timer className="w-3 h-3" />
                            Respond within
                          </span>
                          <span className="text-zinc-200 font-semibold text-base">
                            {roundOverSecs}s
                          </span>
                        </div>
                        <div className="w-full h-1.5 rounded-full bg-zinc-700 overflow-hidden">
                          <div
                            className="h-full rounded-full bg-blue-500 transition-all duration-1000"
                            style={{
                              width: `${(roundOverSecs / roundOverInitialSecs.current) * 100}%`,
                            }}
                          />
                        </div>
                      </div>
                    ) : (
                      <div className="h-8.5" />
                    )}
                    <button
                      className={
                        continued
                          ? "px-8 py-2 bg-zinc-700 text-zinc-500 rounded-lg font-bold text-sm uppercase tracking-wider cursor-not-allowed"
                          : "px-8 py-2 bg-green-700 hover:bg-green-600 text-white rounded-lg font-bold text-sm uppercase tracking-wider"
                      }
                      disabled={continued}
                      onClick={() => {
                        if (!continued) {
                          setContinued(true);
                          send("CONTINUE");
                        }
                      }}
                    >
                      {continued ? "Waiting for other players..." : "Continue to Next Hand"}
                    </button>
                  </>
                )}
              </div>
            )}
          </div>
        </div>
      )}
    </div>
  );
};