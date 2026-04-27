import { useState, useEffect, useRef, type Dispatch, type SetStateAction } from 'react';
import type { Screen, GameStateMsg } from '../types';
import { Phase, GameMode, Rank, Suit } from '../types';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { cn } from '@/lib/utils';
import {
  LogOut,
  Crown,
  Eye,
  Loader2,
  Trophy,
  Timer,
  ChevronDown,
  ChevronUp
} from 'lucide-react';

interface GameScreenProps {
  wsHandle: { send: (message: string) => void } | null;
  serverMessages: string[];
  setServerMessages: Dispatch<SetStateAction<string[]>>;
  username: string;
  setScreen: (screen: Screen) => void;
}

const rankLabel = (rank: Rank): string => {
  const map: Record<Rank, string> = {
    Two: '2', Three: '3', Four: '4', Five: '5', Six: '6',
    Seven: '7', Eight: '8', Nine: '9', Ten: '10',
    Jack: 'J', Queen: 'Q', King: 'K', Ace: 'A',
  };
  return map[rank];
};

const suitSymbol = (suit: Suit): string => {
  const map: Record<Suit, string> = {
    Spades: '\u2660', Hearts: '\u2665', Diamonds: '\u2666', Clubs: '\u2663',
  };
  return map[suit];
};

const suitIsRed = (suit: Suit): boolean => suit === Suit.Hearts || suit === Suit.Diamonds;

const cardSortKey = (rank: Rank, suit: Suit): [number, number] => {
  const rankMap: Record<Rank, number> = {
    Two: 2, Three: 3, Four: 4, Five: 5, Six: 6, Seven: 7, Eight: 8,
    Nine: 9, Ten: 10, Jack: 11, Queen: 12, King: 13, Ace: 14,
  };
  const suitMap: Record<Suit, number> = { Spades: 0, Hearts: 1, Diamonds: 2, Clubs: 3 };
  return [rankMap[rank], suitMap[suit]];
};

const phaseLabel = (phase: Phase): string => {
  const map: Record<Phase, string> = {
    Ante: 'Ante', Showdown: 'Showdown', RoundOver: 'Round Over', GameOver: 'Game Over',
    DealerChoice: "Dealer's Choice", FirstBetting: '1st Bet', Draw: 'Draw',
    SecondBetting: '2nd Bet', SmallBlind: 'Small Blind', BigBlind: 'Big Blind',
    PreFlop: 'Pre-Flop', Flop: 'Flop', Turn: 'Turn', River: 'River',
    ThirdStreet: '3rd St', FourthStreet: '4th St', FifthStreet: '5th St',
    SixthStreet: '6th St', SeventhStreet: '7th St',
  };
  return map[phase];
};

const gameModeLabel = (mode: GameMode): string => {
  const map: Record<GameMode, string> = {
    FiveCardDraw: '5-Card Draw', TexasHoldem: "Texas Hold'em", SevenCardStud: '7-Card Stud',
  };
  return map[mode];
};

const parseRoundOverTimeout = (msg: string): number | undefined => {
  const marker = 'You have ';
  const pos = msg.indexOf(marker);
  if (pos === -1) return undefined;
  const rest = msg.slice(pos + marker.length);
  const end = rest.indexOf('s');
  if (end === -1) return undefined;
  return parseInt(rest.slice(0, end).trim(), 10);
};

// Active phases where a player can act
const ACTIVE_PHASES: Phase[] = [
  Phase.Ante, Phase.FirstBetting, Phase.SecondBetting, Phase.Draw,
  Phase.PreFlop, Phase.Flop, Phase.Turn, Phase.River,
  Phase.ThirdStreet, Phase.FourthStreet, Phase.FifthStreet,
  Phase.SixthStreet, Phase.SeventhStreet, Phase.DealerChoice,
];

const BETTING_PHASES: Phase[] = [
  Phase.FirstBetting, Phase.SecondBetting,
  Phase.PreFlop, Phase.Flop, Phase.Turn, Phase.River,
  Phase.ThirdStreet, Phase.FourthStreet, Phase.FifthStreet,
  Phase.SixthStreet, Phase.SeventhStreet,
];

export const GameScreen = ({ wsHandle, serverMessages, setServerMessages, username, setScreen }: GameScreenProps) => {
  const [game, setGame] = useState<GameStateMsg | null>(null);
  const [selectedDiscards, setSelectedDiscards] = useState<number[]>([]);
  const [raiseInput, setRaiseInput] = useState('');
  const [status, setStatus] = useState('Waiting for server...');
  const [continued, setContinued] = useState(false);
  // Start as undefined - timer only activates when RoundOver arrives from server
  const [roundOverSecs, setRoundOverSecs] = useState<number | undefined>(undefined);
  const [showOtherPlayers, setShowOtherPlayers] = useState(false);

  // Stable interval-based countdown - a single interval that persists
  useEffect(() => {
    const interval = setInterval(() => {
      setRoundOverSecs((prev) => {
        if (prev === undefined || prev <= 0) return prev;
        return prev - 1;
      });
    }, 1000);
    return () => clearInterval(interval);
  }, []); // Run once, update via functional setState

  // Keep a ref to current game phase to detect phase transitions without
  // putting game in the dependency array
  const gamePhasRef = useRef<Phase | undefined>(undefined);

  useEffect(() => {
    if (serverMessages.length === 0) return;
    const snapshotLength = serverMessages.length;

    let transitionIndex = -1;

    for (let i = 0; i < serverMessages.length; i++) {
      const msg = serverMessages[i];

      if (msg.trim() === 'GAME_START') {
        // Drain GAME_START - don't keep it
        continue;
      }

      if (msg.trim() === 'GAME_ENDED') {
        transitionIndex = i;
        break;
      }
      if (msg.startsWith('GAME_LEFT')) {
        transitionIndex = i;
        break;
      }

      if (msg.startsWith('GAME_STATE ')) {
        const rest = msg.trim().slice('GAME_STATE '.length);
        try {
          const state = JSON.parse(rest) as GameStateMsg;
          setStatus(state.message);

          // Clear discards on phase change
          if (gamePhasRef.current !== state.phase) {
            setSelectedDiscards([]);
          }
          gamePhasRef.current = state.phase;

          if (state.phase === Phase.RoundOver) {
            const secs = parseRoundOverTimeout(state.message);
            if (secs !== undefined) setRoundOverSecs(secs);
          } else {
            setRoundOverSecs(undefined);
          }

          if (state.phase !== Phase.RoundOver) {
            setContinued(false);
          }

          setGame(state);
        } catch (e) {
          setStatus(`ERR parsing state: ${e}`);
        }
        continue;
      }
      if (msg.startsWith('ERR')) {
        setStatus(msg);
      }
    }

    if (transitionIndex >= 0) {
      // Preserve the transition message for MenuScreen, plus anything newer.
      setServerMessages((prev) => prev.slice(Math.min(transitionIndex, prev.length)));
      setScreen('MenuScreen');
    } else {
      setServerMessages((prev) => prev.slice(Math.min(snapshotLength, prev.length)));
    }
  }, [serverMessages, setServerMessages]); // eslint-disable-line react-hooks/exhaustive-deps
  // setScreen is stable from App.

  const send = (cmd: string) => wsHandle?.send(`GAME ${cmd}`);
  const sendMenu = (cmd: string) => wsHandle?.send(cmd);

  const me = username;
  const localPlayer = game?.players.find((p) => p.name === me);
  const localIdx = game?.players.findIndex((p) => p.name === me) ?? -1;
  const phase = game?.phase ?? Phase.Ante;
  const gameMode = game?.game_mode ?? GameMode.FiveCardDraw;
  const isStud = gameMode === GameMode.SevenCardStud;

  // isMyTurn: must be active phase, correct player, not folded
  const isMyTurn = !!(
    localPlayer &&
    localIdx >= 0 &&
    game?.active_player_idx === localIdx &&
    !localPlayer.folded &&
    ACTIVE_PHASES.includes(phase)
  );

  const isDealer = game?.dealer_idx === localIdx;
  const otherPlayers = game?.players.filter((p) => p.name !== me) ?? [];

  const handleLeave = () => {
    if (localPlayer) {
      send('LEAVE');
    } else {
      sendMenu('STOP_WATCH');
    }
    setScreen('MenuScreen');
  };

  // Card Component - Mobile optimized sizes
  const PlayingCard = ({
    rank,
    suit,
    selected = false,
    onClick,
    size = 'normal',
    faceDown = false,
    showDiscard = false
  }: {
    rank?: Rank;
    suit?: Suit;
    selected?: boolean;
    onClick?: () => void;
    size?: 'tiny' | 'small' | 'normal';
    faceDown?: boolean;
    showDiscard?: boolean;
  }) => {
    const sizeClasses = {
      tiny: 'w-7 h-10 text-[10px] rounded',
      small: 'w-9 h-13 text-xs rounded-md',
      normal: 'w-11 h-16 text-sm rounded-md md:w-14 md:h-20 md:text-base md:rounded-lg',
    };

    if (faceDown) {
      return (
        <div className={cn(
          sizeClasses[size],
          "border-2 border-blue-600 bg-gradient-to-br from-blue-800 to-blue-950 shadow-md",
          "flex items-center justify-center"
        )}>
          <div className="w-2/3 h-2/3 rounded-sm border border-blue-500/30 bg-blue-900/50" />
        </div>
      );
    }

    if (!rank || !suit) return null;

    const isRed = suitIsRed(suit);

    return (
      <div
        onClick={onClick}
        className={cn(
          sizeClasses[size],
          "relative border-2 flex flex-col items-center justify-center font-bold shadow-md transition-all duration-200 select-none",
          "bg-gradient-to-br from-white to-zinc-100",
          selected
            ? "border-blue-400 -translate-y-2 md:-translate-y-3 shadow-blue-400/30 shadow-lg"
            : "border-zinc-300",
          onClick && "cursor-pointer active:scale-95"
        )}
      >
        <span className={cn("leading-none font-bold", isRed ? "text-red-600" : "text-zinc-900")}>
          {rankLabel(rank)}
        </span>
        <span className={cn("leading-none", size === 'tiny' ? 'text-xs' : size === 'small' ? 'text-sm' : 'text-base md:text-lg', isRed ? "text-red-600" : "text-zinc-900")}>
          {suitSymbol(suit)}
        </span>
        {showDiscard && selected && (
          <div className="absolute -bottom-5 text-blue-400 text-[10px] md:text-xs font-bold uppercase">Discard</div>
        )}
      </div>
    );
  };

  // Compact Player Badge for mobile
  const PlayerBadge = ({ player, idx }: { player: GameStateMsg['players'][0]; idx: number }) => {
    const isMe = player.name === me;
    // isActiveTurn for badge highlight: only in active betting/action phases
    const isActiveTurn = game?.active_player_idx === idx && ACTIVE_PHASES.includes(phase) && phase !== Phase.DealerChoice;
    const playerCount = game?.players.length ?? 0;
    const isSmallBlind = gameMode === GameMode.TexasHoldem && playerCount > 1 && idx === ((game?.dealer_idx ?? 0) + 1) % playerCount;
    const isBigBlind = gameMode === GameMode.TexasHoldem && playerCount > 2 && idx === ((game?.dealer_idx ?? 0) + 2) % playerCount;

    const totalHandSize = gameMode === GameMode.FiveCardDraw ? 5
      : gameMode === GameMode.TexasHoldem ? 2
      : phase === Phase.ThirdStreet ? 3
      : phase === Phase.FourthStreet ? 4
      : phase === Phase.FifthStreet ? 5
      : phase === Phase.SixthStreet ? 6 : 7;

    const faceDownCount = isMe ? 0 : totalHandSize - (player.cards_up?.length ?? 0);
    const hasCards = player.cards && player.cards.length > 0;

    // Whether to show all cards (showdown/roundover) - with uncontested win logic
    const isUncontested = (phase === Phase.RoundOver) &&
      (game?.winners[0]?.includes('uncontested') ?? false);
    const shouldShow = (phase === Phase.Showdown || phase === Phase.RoundOver) &&
      hasCards &&
      (!player.folded || isStud) &&
      // For non-local players in uncontested wins, only show their face-up cards
      (!isUncontested || isMe);

    const sortedUpCards = [...(player.cards_up ?? [])].sort((a, b) => {
      const keyA = cardSortKey(a.rank, a.suit);
      const keyB = cardSortKey(b.rank, b.suit);
      return keyA[0] !== keyB[0] ? keyB[0] - keyA[0] : keyB[1] - keyA[1];
    });

    return (
      <div className={cn(
        "flex flex-col gap-1.5 p-2 md:p-3 rounded-lg border transition-all",
        isActiveTurn && "border-amber-500 bg-amber-500/10",
        !isActiveTurn && "border-zinc-800 bg-zinc-900/50",
        (player.folded || player.left) && "opacity-50"
      )}>
        <div className="flex items-center gap-1.5 flex-wrap">
          <span className={cn("font-medium text-xs md:text-sm truncate max-w-[80px] md:max-w-none", isMe ? "text-blue-400" : "text-zinc-200")}>
            {player.name}
          </span>
          {player.is_dealer && <Crown className="w-3 h-3 text-amber-500" />}
          {isSmallBlind && <span className="text-[10px] bg-blue-500/20 text-blue-400 px-1 rounded">SB</span>}
          {isBigBlind && <span className="text-[10px] bg-purple-500/20 text-purple-400 px-1 rounded">BB</span>}
          {player.folded && <span className="text-[10px] bg-red-500/20 text-red-400 px-1 rounded">F</span>}
        </div>

        <div className="flex gap-0.5 min-h-[40px] items-center">
          {shouldShow ? (
            // Uncontested: show only face-up cards for others; show all for me
            isUncontested && !isMe ? (
              sortedUpCards.length > 0
                ? sortedUpCards.map((card, i) => <PlayingCard key={i} rank={card.rank} suit={card.suit} size="tiny" />)
                : <span className="text-zinc-600 text-[10px]">No cards</span>
            ) : (
              player.cards?.map((card, i) => <PlayingCard key={i} rank={card.rank} suit={card.suit} size="tiny" />)
            )
          ) : phase === Phase.Ante || ((phase === Phase.Showdown || phase === Phase.RoundOver) && !hasCards) || player.folded || player.left ? (
            <span className="text-zinc-600 text-[10px]">No cards</span>
          ) : (
            <>
              {sortedUpCards.map((card, i) => <PlayingCard key={i} rank={card.rank} suit={card.suit} size="tiny" />)}
              {Array.from({ length: faceDownCount }).map((_, i) => <PlayingCard key={`down-${i}`} faceDown size="tiny" />)}
            </>
          )}
        </div>

        <div className="flex gap-2 text-[10px] md:text-xs">
          <span className="text-blue-400 font-semibold">${player.balance}</span>
          {player.bet > 0 && <span className="text-amber-400">Bet: ${player.bet}</span>}
        </div>
      </div>
    );
  };

  return (
    <div className="flex flex-col h-screen bg-zinc-950 overflow-hidden safe-area-top safe-area-bottom">
      {/* Header - Compact for mobile */}
      <header className="flex items-center justify-between px-3 md:px-6 py-2 md:py-3 bg-zinc-900/90 backdrop-blur border-b border-zinc-800 shrink-0">
        <div className="flex items-center gap-2 md:gap-3">
          <span className="text-blue-400 text-xs md:text-sm font-semibold">{phaseLabel(phase)}</span>
          {game && (
            <span className="text-[10px] md:text-xs px-1.5 md:px-2 py-0.5 md:py-1 rounded-full bg-zinc-800 text-zinc-400 font-medium">
              {gameModeLabel(game.game_mode)}
            </span>
          )}
        </div>

        {game && (
          <div className="flex items-center gap-3 md:gap-6">
            <div className="text-zinc-500 text-xs max-w-[120px] md:max-w-xs text-right truncate hidden sm:block">
              {status}
            </div>
            <div className="text-center">
              <span className="text-amber-400 font-bold text-sm md:text-xl">${game.pot}</span>
              <span className="text-zinc-500 text-[10px] md:text-xs block">Pot</span>
            </div>
            <div className="text-center hidden sm:block">
              <span className="text-zinc-200 font-semibold text-sm">${game.current_bet}</span>
              <span className="text-zinc-500 text-xs block">Bet</span>
            </div>
            <div className="text-center hidden sm:block">
              <span className="text-zinc-400 text-sm">#{game.hand_number}</span>
              <span className="text-zinc-500 text-xs block">Hand</span>
            </div>
          </div>
        )}

        <Button variant="ghost" size="sm" className="text-zinc-500 hover:text-red-400 p-1.5 md:p-2" onClick={handleLeave}>
          <LogOut className="w-4 h-4" />
        </Button>
      </header>

      {/* Dealer's Choice */}
      {phase === Phase.DealerChoice ? (
        <div className="flex-1 flex flex-col items-center justify-center gap-4 md:gap-6 p-4 md:p-8">
          <div className="w-12 h-12 md:w-16 md:h-16 rounded-full bg-amber-500/10 border border-amber-500/20 flex items-center justify-center">
            <Crown className="w-6 h-6 md:w-8 md:h-8 text-amber-500" />
          </div>
          <h2 className="text-zinc-400 text-xs md:text-sm uppercase tracking-widest">Dealer&apos;s Choice</h2>

          {isDealer && isMyTurn ? (
            <>
              <p className="text-zinc-200 text-sm md:text-base font-medium text-center">Choose the game variant:</p>
              <div className="flex flex-col md:flex-row gap-2 md:gap-4 w-full max-w-sm md:max-w-none md:w-auto">
                <Button variant="secondary" size="lg" className="w-full md:w-auto" onClick={() => send('MODE FiveCardDraw')}>
                  Five-Card Draw
                </Button>
                <Button variant="secondary" size="lg" className="w-full md:w-auto" onClick={() => send('MODE TexasHoldem')}>
                  Texas Hold&apos;em
                </Button>
                <Button variant="secondary" size="lg" className="w-full md:w-auto" onClick={() => send('MODE SevenCardStud')}>
                  Seven-Card Stud
                </Button>
              </div>
            </>
          ) : game && (
            <p className="text-zinc-500 text-sm flex items-center gap-2">
              <Loader2 className="w-4 h-4 animate-spin" />
              Waiting for {game.players[game.dealer_idx].name}...
            </p>
          )}
        </div>
      ) : (
        <div className="flex-1 flex flex-col overflow-hidden">
          {/* Other Players Toggle (Mobile) */}
          {game && otherPlayers.length > 0 && (
            <div className="md:hidden border-b border-zinc-800">
              <button
                onClick={() => setShowOtherPlayers(!showOtherPlayers)}
                className="w-full flex items-center justify-between px-3 py-2 text-zinc-400 text-xs"
              >
                <span>Other Players ({otherPlayers.length})</span>
                {showOtherPlayers ? <ChevronUp className="w-4 h-4" /> : <ChevronDown className="w-4 h-4" />}
              </button>
              {showOtherPlayers && (
                <div className="flex gap-2 px-3 pb-3 overflow-x-auto hide-scrollbar">
                  {otherPlayers.map((player) => {
                    const actualIdx = game.players.findIndex(p => p.name === player.name);
                    return <PlayerBadge key={player.name} player={player} idx={actualIdx} />;
                  })}
                </div>
              )}
            </div>
          )}

          {/* Desktop Players Row */}
          {game && (
            <div className="hidden md:flex gap-3 px-4 py-3 bg-zinc-900/30 border-b border-zinc-800 overflow-x-auto hide-scrollbar justify-center">
              {game.players.map((player, idx) => (
                <PlayerBadge key={player.name} player={player} idx={idx} />
              ))}
            </div>
          )}

          {/* Community Cards */}
          {game && game.community_cards.length > 0 && (
            <div className="flex items-center justify-center gap-1.5 md:gap-3 py-3 md:py-4 border-b border-zinc-800 bg-zinc-900/30">
              <span className="text-zinc-500 text-[10px] md:text-xs uppercase tracking-wider mr-1 md:mr-2">Board</span>
              {game.community_cards.map((card, i) => (
                <PlayingCard key={i} rank={card.rank} suit={card.suit} size="small" />
              ))}
              {gameMode === GameMode.TexasHoldem &&
                Array.from({ length: 5 - game.community_cards.length }).map((_, i) => (
                  <div key={i} className="w-9 h-13 md:w-14 md:h-20 rounded-md md:rounded-lg border-2 border-dashed border-zinc-700 bg-zinc-900/50 flex items-center justify-center">
                    <span className="text-zinc-700 text-sm md:text-lg">?</span>
                  </div>
                ))}
            </div>
          )}

          {/* Main Game Area */}
          <div className="flex-1 flex items-center justify-center p-4">
            {game ? (
              <div className="poker-table w-32 h-20 md:w-48 md:h-28 rounded-full flex items-center justify-center">
                <span className="text-white font-bold text-xl md:text-3xl">${game.pot}</span>
              </div>
            ) : (
              <div className="flex flex-col items-center gap-3">
                <Loader2 className="w-6 h-6 md:w-8 md:h-8 animate-spin text-blue-500" />
                <p className="text-zinc-600 text-[10px] md:text-xs uppercase tracking-widest">Waiting for game...</p>
              </div>
            )}
          </div>

          {/* Action Panel */}
          <div className="shrink-0 flex flex-col items-center gap-3 md:gap-4 px-3 md:px-8 py-4 md:py-6 bg-zinc-900/90 backdrop-blur border-t border-zinc-800">
            {/* Player's Hand */}
            {localPlayer && !localPlayer.folded && !localPlayer.left && localPlayer.cards && localPlayer.cards.length > 0 && (
              <div className="flex gap-1.5 md:gap-3 mb-1">
                {localPlayer.cards.map((card, i) => {
                  const isSelected = selectedDiscards.includes(i);
                  const isDraw = phase === Phase.Draw;

                  return (
                    <PlayingCard
                      key={i}
                      rank={card.rank}
                      suit={card.suit}
                      selected={isSelected}
                      showDiscard={isDraw}
                      onClick={isDraw && isMyTurn ? () => {
                        if (selectedDiscards.includes(i)) {
                          setSelectedDiscards(selectedDiscards.filter((x) => x !== i));
                        } else if (selectedDiscards.length < 3) {
                          setSelectedDiscards([...selectedDiscards, i]);
                        }
                      } : undefined}
                    />
                  );
                })}
              </div>
            )}

            {/* Local Player Info */}
            {localPlayer && (
              <div className="flex items-center gap-4 text-xs md:text-sm">
                <span className="text-blue-400 font-semibold">${localPlayer.balance}</span>
                {localPlayer.bet > 0 && <span className="text-amber-400">Bet: ${localPlayer.bet}</span>}
                {game && <span className="text-zinc-500">Call: ${game.current_bet - (localPlayer.bet ?? 0)}</span>}
              </div>
            )}

            {/* Action Buttons */}
            {isMyTurn && (
              <div className="flex gap-2 flex-wrap justify-center">
                {BETTING_PHASES.includes(phase) && (
                  <>
                    <Button variant="destructive" size="sm" onClick={() => send('FOLD')}>Fold</Button>
                    {(() => {
                      const toCall = game ? game.current_bet - (game.players[localIdx]?.bet ?? 0) : 0;
                      return toCall === 0 ? (
                        <Button variant="secondary" size="sm" onClick={() => send('CHECK')}>Check</Button>
                      ) : (
                        <Button size="sm" onClick={() => send('CALL')}>Call ${toCall}</Button>
                      );
                    })()}
                    <div className="flex rounded-lg overflow-hidden border border-zinc-700">
                      <Input
                        className="w-16 md:w-20 rounded-none border-0 bg-zinc-800 h-8 text-sm"
                        placeholder="$"
                        value={raiseInput}
                        onChange={(e) => setRaiseInput(e.target.value)}
                      />
                      <Button
                        className="rounded-none h-8"
                        size="sm"
                        onClick={() => {
                          const v = parseInt(raiseInput.trim(), 10);
                          if (!isNaN(v)) {
                            const toCall = game ? game.current_bet - (game.players[localIdx]?.bet ?? 0) : 0;
                            send(toCall === 0 ? `BET ${v}` : `RAISE ${v}`);
                            setRaiseInput('');
                          }
                        }}
                      >
                        {game && game.current_bet - (game.players[localIdx]?.bet ?? 0) === 0 ? 'Bet' : 'Raise'}
                      </Button>
                    </div>
                    <Button variant="outline" size="sm" onClick={() => send('PASS')}>Pass</Button>
                  </>
                )}

                {phase === Phase.Draw && (
                  <Button size="sm" onClick={() => {
                    send(selectedDiscards.length === 0 ? 'DRAW' : `DRAW ${selectedDiscards.join(',')}`);
                    setSelectedDiscards([]);
                  }}>
                    Confirm Draw ({selectedDiscards.length})
                  </Button>
                )}

                {phase === Phase.Ante && (
                  <>
                    <Button variant="destructive" size="sm" onClick={() => send('FOLD')}>Fold</Button>
                    <Button size="sm" onClick={() => send('CALL')}>Call</Button>
                  </>
                )}
              </div>
            )}

            {/* Waiting Message */}
            {!isMyTurn && [...BETTING_PHASES, Phase.Ante, Phase.Draw].includes(phase) && (
              <p className="text-zinc-500 text-xs md:text-sm flex items-center gap-2">
                <Loader2 className="w-3 h-3 md:w-4 md:h-4 animate-spin" />
                Waiting for other players...
              </p>
            )}

            {/* Spectator Mode */}
            {!localPlayer && (
              <p className="text-zinc-400 text-sm md:text-lg font-semibold flex items-center gap-2">
                <Eye className="w-4 h-4 md:w-5 md:h-5" />
                Watching Game
              </p>
            )}

            {/* Round Over / Game Over */}
            {(phase === Phase.RoundOver || phase === Phase.GameOver) && game && (
              <div className="flex flex-col items-center gap-3 md:gap-4">
                {game.winners.length > 0 && (
                  <div className="flex items-center gap-2 text-blue-400 font-bold text-sm md:text-lg">
                    <Trophy className="w-4 h-4 md:w-5 md:h-5" />
                    {game.winners[0]}
                  </div>
                )}
                {phase === Phase.RoundOver && localPlayer && roundOverSecs !== undefined && (
                  <>
                    <div className="flex flex-col items-center gap-2 w-48 md:w-56">
                      <div className="flex items-center justify-between w-full text-[10px] md:text-xs">
                        <span className="text-zinc-400 flex items-center gap-1">
                          <Timer className="w-3 h-3" />
                          Respond within
                        </span>
                        <span className="text-zinc-200 font-semibold text-sm md:text-base">{roundOverSecs}s</span>
                      </div>
                      <div className="w-full h-1.5 md:h-2 rounded-full bg-zinc-700 overflow-hidden">
                        <div
                          className="h-full rounded-full bg-blue-500 transition-all duration-1000"
                          style={{ width: `${(roundOverSecs / 60) * 100}%` }}
                        />
                      </div>
                    </div>
                    <Button
                      variant={continued ? "secondary" : "gold"}
                      size="sm"
                      disabled={continued}
                      onClick={() => {
                        if (!continued) {
                          setContinued(true);
                          setRoundOverSecs(undefined);
                          send('CONTINUE');
                        }
                      }}
                    >
                      {continued ? (
                        <>
                          <Loader2 className="w-3 h-3 md:w-4 md:h-4 animate-spin" />
                          Waiting...
                        </>
                      ) : (
                        'Continue'
                      )}
                    </Button>
                  </>
                )}
              </div>
            )}

            {phase === Phase.Showdown && game && game.winners.length > 0 && (
              <div className="flex items-center gap-2 text-blue-400 font-bold text-sm">
                <Trophy className="w-4 h-4 md:w-5 md:h-5" />
                {game.winners[0]}
              </div>
            )}
          </div>
        </div>
      )}
    </div>
  );
};
