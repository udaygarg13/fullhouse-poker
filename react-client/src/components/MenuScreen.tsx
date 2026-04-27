import {
  useState,
  useEffect,
  useRef,
  type Dispatch,
  type SetStateAction,
} from "react";
import type { Screen, GameConfig } from "../types";
import { Card, CardContent } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
} from "@/components/ui/dialog";
import {
  Play,
  BookText,
  BarChart3,
  History,
  Download,
  Upload,
  LogOut,
  Crown,
  Users,
  AlertTriangle,
  CheckCircle,
  Menu,
  ChevronLeft,
} from "lucide-react";
import { cn } from "@/lib/utils";

interface MenuScreenProps {
  wsHandle: { send: (message: string) => void } | null;
  serverMessages: string[];
  setServerMessages: Dispatch<SetStateAction<string[]>>;
  setScreen: (screen: Screen) => void;
  username: string;
  setUsername: (username: string) => void;
}

const formatDatetime = (input: string): string => {
  const dt = new Date(input);
  if (isNaN(dt.getTime())) return "Invalid date";

  return dt.toLocaleString("en-US", {
    hour: "numeric",
    minute: "2-digit",
    hour12: true,
    day: "numeric",
    month: "short",
    year: "numeric",
  });
};

const menuItems = [
  { id: "play", label: "Play Game", icon: Play },
  { id: "rules", label: "Rules", icon: BookText },
  { id: "stats", label: "Statistics", icon: BarChart3 },
  { id: "results", label: "Results", icon: History },
  { id: "deposit", label: "Deposit", icon: Download },
  { id: "withdraw", label: "Withdraw", icon: Upload },
];

export const MenuScreen = ({
  wsHandle,
  serverMessages,
  setServerMessages,
  setScreen,
  username,
  setUsername,
}: MenuScreenProps) => {
  const [activePanel, setActivePanel] = useState("play");
  const [responseMsg, setResponseMsg] = useState("");
  const [amountInput, setAmountInput] = useState("");
  const [sidebarOpen, setSidebarOpen] = useState(false);

  const [configAnte, setConfigAnte] = useState("");
  const [configMaxBet, setConfigMaxBet] = useState("");
  const [configRedraws, setConfigRedraws] = useState("");
  const [configSb, setConfigSb] = useState("");
  const [configBb, setConfigBb] = useState("");

  const [statsRows, setStatsRows] = useState<
    Array<[string, number, number, number, number]>
  >([]);
  const [resultsRows, setResultsRows] = useState<
    Array<[string, string, string, number]>
  >([]);

  const [queueHost, setQueueHost] = useState("");
  const [queuePlayers, setQueuePlayers] = useState<string[]>([]);
  const [gameInProgress, setGameInProgress] = useState(false);
  const [isWatching, setIsWatching] = useState(false);
  const [watchPending, setWatchPending] = useState(false);
  const [showGameEndedModal, setShowGameEndedModal] = useState(false);

  // Use a ref for activePanel so the message-handling effect can read it
  // without re-firing when the user just navigates between panels.
  const activePanelRef = useRef(activePanel);
  activePanelRef.current = activePanel;

  // Refs for state that is read inside the message-handling effect.
  // Using refs avoids stale-closure bugs where the effect captures an old value.
  const gameInProgressRef = useRef(gameInProgress);
  gameInProgressRef.current = gameInProgress;
  const watchPendingRef = useRef(watchPending);
  watchPendingRef.current = watchPending;

  useEffect(() => {
    if (serverMessages.length === 0) return;
    const snapshotLength = serverMessages.length;

    let transitionIndex = -1;

    for (let i = 0; i < serverMessages.length; i++) {
      const resp = serverMessages[i];
      if (resp.startsWith("QUEUE ")) {
        if (gameInProgressRef.current) continue;
        // Use splitn(3)-equivalent: split into at most 3 parts
        const spaceIdx1 = resp.indexOf(" ");
        const spaceIdx2 = resp.indexOf(" ", spaceIdx1 + 1);
        const host =
          spaceIdx1 >= 0
            ? spaceIdx2 >= 0
              ? resp.slice(spaceIdx1 + 1, spaceIdx2)
              : resp.slice(spaceIdx1 + 1)
            : "NONE";
        const playersStr =
          spaceIdx2 >= 0 ? resp.slice(spaceIdx2 + 1).trim() : "EMPTY";

        setQueueHost(host === "NONE" ? "" : host);
        setQueuePlayers(playersStr === "EMPTY" ? [] : playersStr.split(","));
        continue;
      }

      if (resp.trim() === "GAME_IN_PROGRESS") {
        setGameInProgress(true);
        continue;
      }

      if (resp.trim() === "GAME_START") {
        transitionIndex = i;
        break;
      }

      if (resp.trim() === "GAME_ENDED") {
        setGameInProgress(false);
        setIsWatching(false);
        setQueueHost("");
        setQueuePlayers([]);
        setShowGameEndedModal(true);
        continue;
      }

      if (resp.startsWith("OK")) {
        const body = resp.slice(3).trim();
        if (body === "Watching") {
          setWatchPending(false);
          setIsWatching(true);
          transitionIndex = i;
          break;
        }
        const panel = activePanelRef.current;
        if (panel === "stats") {
          const rows = body
            .split("\n")
            .map((line) => {
              const p = line.split("|");
              if (p.length === 5) {
                return [
                  p[0],
                  parseInt(p[1]) || 0,
                  parseInt(p[2]) || 0,
                  parseInt(p[3]) || 0,
                  parseInt(p[4]) || 0,
                ] as const;
              }
              return null;
            })
            .filter(
              (r): r is [string, number, number, number, number] => r !== null,
            );
          setStatsRows(rows);
        } else if (panel === "results") {
          const rows = body
            .split("\n")
            .map((line) => {
              const p = line.split("|");
              if (p.length === 4) {
                return [p[0], p[1], p[2], parseInt(p[3]) || 0] as const;
              }
              return null;
            })
            .filter((r): r is [string, string, string, number] => r !== null);
          setResultsRows(rows);
        } else if (panel === "logout") {
          setUsername("");
          setScreen("LoginScreen");
        } else {
          setResponseMsg(body);
        }
      } else if (resp.startsWith("ERR")) {
        const body = resp.slice(4).trim();
        if (watchPendingRef.current) {
          setWatchPending(false);
        }
        setResponseMsg(body);
      } else {
        // RAW message - e.g. JSON config for rules panel
        const panel = activePanelRef.current;
        if (panel === "rules") {
          try {
            const cfg = JSON.parse(resp.trim()) as GameConfig;
            setConfigAnte(cfg.ante.toString());
            setConfigMaxBet(cfg.max_bet.toString());
            setConfigRedraws(cfg.max_redraws.toString());
            setConfigSb(cfg.small_blind.toString());
            setConfigBb(cfg.big_blind.toString());
          } catch (e) {
            console.error("Failed to parse game config:", e);
          }
        }
      }
    }

    if (transitionIndex >= 0) {
      // Consume only the frames we handled from this snapshot, preserving any
      // GAME_STATE frames that were already queued after the transition signal
      // and any newer frames that arrived while this effect was running.
      setServerMessages((prev) =>
        prev.slice(Math.min(transitionIndex + 1, prev.length)),
      );
      setScreen("GameScreen");
    } else {
      setServerMessages((prev) =>
        prev.slice(Math.min(snapshotLength, prev.length)),
      );
    }
  }, [serverMessages]); // eslint-disable-line react-hooks/exhaustive-deps
  // gameInProgress/watchPending are read via refs. setScreen/clearMessages/setUsername
  // are stable useCallback refs from App.

  const send = (msg: string) => {
    setResponseMsg("");
    if (wsHandle) {
      wsHandle.send(msg);
    }
  };

  const handleMenuItemClick = (id: string) => {
    setActivePanel(id);
    setResponseMsg("");
    setAmountInput("");
    setSidebarOpen(false);
    if (id === "stats") send("STATS");
    if (id === "results") send("RESULTS");
    if (id === "rules") send("RULES");
  };

  const me = username;
  const host = queueHost;
  const players = queuePlayers;
  const inProgress = gameInProgress;

  const iAmHost = host === me;
  const iAmInQueue = players.includes(me);
  const hasHost = host.length > 0;
  const playerCount = players.length;

  return (
    <div className="flex h-svh md:h-screen bg-zinc-950 overflow-hidden">
      {/* Mobile Header */}
      <header className="fixed top-0 left-0 right-0 z-40 flex items-center justify-between px-4 py-3 pt-[max(0.75rem,env(safe-area-inset-top))] bg-zinc-900/95 backdrop-blur border-b border-zinc-800 md:hidden">
        <button
          onClick={() => setSidebarOpen(true)}
          className="p-2 -ml-2 text-zinc-400 hover:text-zinc-100"
        >
          <Menu className="w-5 h-5" />
        </button>
        <h1 className="text-lg font-bold text-zinc-100">Poker</h1>
        <div className="w-9" />
      </header>

      {/* Mobile Sidebar Overlay */}
      {sidebarOpen && (
        <div
          className="fixed inset-0 z-50 bg-black/60 md:hidden"
          onClick={() => setSidebarOpen(false)}
        />
      )}

      {/* Sidebar */}
      <aside
        className={cn(
          "fixed inset-y-0 left-0 z-50 w-64 bg-zinc-900 border-r border-zinc-800 flex min-h-0 flex-col transform transition-transform duration-300 md:relative md:translate-x-0",
          sidebarOpen ? "translate-x-0" : "-translate-x-full",
        )}
      >
        <div className="p-4 md:p-6 border-b border-zinc-800 flex items-center justify-between">
          <div>
            <h1 className="text-xl md:text-2xl font-bold text-zinc-100 tracking-tight">
              Poker
            </h1>
            <p className="text-zinc-500 text-xs tracking-widest uppercase mt-0.5">
              Team Fullhouse
            </p>
          </div>
          <button
            onClick={() => setSidebarOpen(false)}
            className="p-2 -mr-2 text-zinc-400 hover:text-zinc-100 md:hidden"
          >
            <ChevronLeft className="w-5 h-5" />
          </button>
        </div>

        <div className="px-4 md:px-6 py-3 md:py-4 border-b border-zinc-800">
          <p className="text-zinc-500 text-xs uppercase tracking-wider mb-1">
            Logged in as
          </p>
          <p className="text-blue-400 text-sm font-semibold truncate">
            {username}
          </p>
        </div>

        <nav className="flex-1 p-2 md:p-3 space-y-1 overflow-y-auto">
          {menuItems.map(({ id, label, icon: Icon }) => (
            <button
              key={id}
              className={cn(
                "w-full flex items-center gap-3 px-3 md:px-4 py-2.5 rounded-lg text-sm font-medium transition-all cursor-pointer",
                activePanel === id
                  ? "bg-blue-600 text-white shadow-lg shadow-blue-500/20"
                  : "text-zinc-400 hover:bg-zinc-800 hover:text-zinc-100",
              )}
              onClick={() => handleMenuItemClick(id)}
            >
              <Icon className="w-4 h-4" />
              {label}
            </button>
          ))}
        </nav>

        <div className="p-2 md:p-3 border-t border-zinc-800">
          <button
            className="w-full flex items-center gap-3 px-3 md:px-4 py-2.5 rounded-lg text-sm font-medium text-zinc-400 hover:bg-zinc-800 hover:text-zinc-100 transition-all cursor-pointer"
            onClick={() => {
              setActivePanel("logout");
              setSidebarOpen(false);
              send("LOGOUT");
            }}
          >
            <LogOut className="w-4 h-4" />
            Sign Out
          </button>
        </div>
      </aside>

      {/* Main Content */}
      <main className="flex-1 min-h-0 overflow-y-auto pt-14 md:pt-0">
        <div className="p-4 md:p-8 max-w-2xl mx-auto md:mx-0">
          {activePanel === "play" && (
            <div className="animate-fade-in">
              <h2 className="text-xl md:text-2xl font-bold text-zinc-100 mb-1">
                Play Game
              </h2>
              <p className="text-zinc-500 text-sm mb-4 md:mb-6">
                Host or join a poker game
              </p>

              <Card>
                <CardContent className="p-4 md:p-6">
                  {inProgress ? (
                    <div className="space-y-4">
                      <div className="flex items-center justify-center py-6 md:py-8">
                        <div className="text-center">
                          <div className="w-12 h-12 rounded-full bg-zinc-800 border border-zinc-700 flex items-center justify-center mx-auto mb-3">
                            <Play className="w-6 h-6 text-zinc-500" />
                          </div>
                          <p className="text-zinc-400 text-sm">
                            A game is currently in progress
                          </p>
                        </div>
                      </div>
                      <div className="border-t border-zinc-800 pt-4 space-y-2">
                        {!isWatching && (
                          <Button
                            className="w-full"
                            onClick={() => {
                              setWatchPending(true);
                              send("WATCH");
                            }}
                          >
                            Watch Game
                          </Button>
                        )}
                      </div>
                    </div>
                  ) : (
                    <div className="space-y-4">
                      {players.length === 0 ? (
                        <div className="flex items-center justify-center py-6 md:py-8">
                          <div className="text-center">
                            <div className="w-12 h-12 rounded-full bg-zinc-800 border border-zinc-700 flex items-center justify-center mx-auto mb-3">
                              <Users className="w-6 h-6 text-zinc-500" />
                            </div>
                            <p className="text-zinc-500 text-sm">
                              No game hosted yet
                            </p>
                          </div>
                        </div>
                      ) : (
                        <div className="space-y-3">
                          <div className="flex items-center justify-between">
                            <span className="text-zinc-500 text-xs uppercase tracking-wider">
                              Players
                            </span>
                            <span className="text-zinc-400 text-xs">
                              {playerCount} joined
                            </span>
                          </div>
                          <div className="space-y-2 max-h-48 overflow-y-auto">
                            {players.map((player) => (
                              <div
                                key={player}
                                className={cn(
                                  "flex items-center gap-3 px-3 md:px-4 py-2.5 md:py-3 rounded-lg",
                                  player === me
                                    ? "bg-blue-500/10 border border-blue-500/20"
                                    : "bg-zinc-800/50",
                                )}
                              >
                                <span
                                  className={cn(
                                    "text-sm font-medium flex-1 truncate",
                                    player === me
                                      ? "text-blue-400"
                                      : "text-zinc-200",
                                  )}
                                >
                                  {player}
                                </span>
                                {player === host && (
                                  <Crown
                                    className={cn(
                                      "w-3.5 h-3.5 md:w-4 md:h-4",
                                      player === me
                                        ? "text-blue-400"
                                        : "text-zinc-200",
                                    )}
                                  />
                                )}
                              </div>
                            ))}
                          </div>
                        </div>
                      )}

                      <div className="border-t border-zinc-800 pt-4 space-y-2">
                        {!hasHost && (
                          <Button
                            className="w-full"
                            onClick={() => {
                              send("HOST");
                              setQueueHost(username);
                              setQueuePlayers([username]);
                            }}
                          >
                            Host Game
                          </Button>
                        )}

                        {hasHost && !iAmInQueue && (
                          <Button
                            className="w-full"
                            onClick={() => {
                              send("JOIN");
                              // Optimistic update: immediately show in queue
                              setQueuePlayers([...queuePlayers, username]);
                            }}
                          >
                            Join Game
                          </Button>
                        )}

                        {iAmInQueue && !iAmHost && (
                          <Button
                            variant="outline"
                            className="w-full"
                            onClick={() => send("LEAVE")}
                          >
                            Leave Queue
                          </Button>
                        )}

                        {iAmHost && (
                          <>
                            <Button
                              variant="outline"
                              className="w-full"
                              onClick={() => send("LEAVE")}
                            >
                              Cancel Game
                            </Button>
                            <Button
                              className="w-full"
                              disabled={playerCount < 2}
                              onClick={() => send("START")}
                            >
                              Start Game
                            </Button>
                          </>
                        )}
                      </div>
                    </div>
                  )}
                </CardContent>
              </Card>
            </div>
          )}

          {activePanel === "rules" && (
            <div className="animate-fade-in">
              <h2 className="text-xl md:text-2xl font-bold text-zinc-100 mb-1">
                House Rules
              </h2>
              <p className="text-zinc-500 text-sm mb-4 md:mb-6">
                Configure house rules
              </p>

              <Card>
                <CardContent className="p-4 md:p-6 space-y-3">
                  {[
                    {
                      label: "Ante",
                      value: configAnte,
                      setter: setConfigAnte,
                      hint: "e.g. 10",
                    },
                    {
                      label: "Max Bet",
                      value: configMaxBet,
                      setter: setConfigMaxBet,
                      hint: "e.g. 200",
                    },
                    {
                      label: "Max Redraws",
                      value: configRedraws,
                      setter: setConfigRedraws,
                      hint: "e.g. 3",
                    },
                    {
                      label: "Small Blind",
                      value: configSb,
                      setter: setConfigSb,
                      hint: "e.g. 10",
                    },
                    {
                      label: "Big Blind",
                      value: configBb,
                      setter: setConfigBb,
                      hint: "e.g. 20",
                    },
                  ].map(({ label, value, setter, hint }) => (
                    <div key={label} className="flex flex-col">
                      <label className="text-sm font-medium text-zinc-300 mb-1.5">
                        {label}
                      </label>
                      <Input
                        placeholder={hint}
                        value={value}
                        className="focus:ring-0 focus:border-zinc-700"
                        disabled={gameInProgress}
                        onChange={(e) => setter(e.target.value)}
                      />
                    </div>
                  ))}

                  <div className="pt-3">
                    <Button
                      className="w-full"
                      disabled={gameInProgress}
                      onClick={() => {
                        const cfg: GameConfig = {
                          ante: parseInt(configAnte.trim()) || 0,
                          max_bet: parseInt(configMaxBet.trim()) || 0,
                          max_redraws: parseInt(configRedraws.trim()) || 0,
                          small_blind: parseInt(configSb.trim()) || 0,
                          big_blind: parseInt(configBb.trim()) || 0,
                        };
                        send(`SAVE_RULES ${JSON.stringify(cfg)}`);
                      }}
                    >
                      Save Rules
                    </Button>
                    {responseMsg && (
                      <p className="text-sm text-blue-400 mt-3 flex items-center gap-2">
                        <CheckCircle className="w-4 h-4" />
                        {responseMsg}
                      </p>
                    )}
                  </div>
                </CardContent>
              </Card>
            </div>
          )}

          {activePanel === "stats" && (
            <div className="animate-fade-in">
              <h2 className="text-xl md:text-2xl font-bold text-zinc-100 mb-1">
                Statistics
              </h2>
              <p className="text-zinc-500 text-sm mb-4 md:mb-6">
                Player statistics
              </p>

              <Card>
                <CardContent className="p-0">
                  <div className="overflow-hidden rounded-xl">
                    {/* Mobile Card View */}
                    <div className="md:hidden max-h-[65vh] overflow-auto divide-y divide-zinc-800">
                      {statsRows.length === 0 ? (
                        <div className="px-4 py-12 text-center text-zinc-500">
                          No statistics available
                        </div>
                      ) : (
                        statsRows.map(([name, bal, rounds, wins, folds]) => (
                          <div key={name} className="p-4 space-y-2">
                            <div className="flex items-center justify-between">
                              <span className="text-zinc-200 font-medium">
                                {name}
                              </span>
                              <span className="text-blue-400 font-semibold">
                                ${bal}
                              </span>
                            </div>
                            <div className="flex gap-4 text-xs text-zinc-500">
                              <span>
                                Rounds:{" "}
                                <span className="text-zinc-300">{rounds}</span>
                              </span>
                              <span>
                                Wins:{" "}
                                <span className="text-zinc-300">{wins}</span>
                              </span>
                              <span>
                                Folds:{" "}
                                <span className="text-zinc-300">{folds}</span>
                              </span>
                            </div>
                          </div>
                        ))
                      )}
                    </div>

                    {/* Desktop Table View */}
                    <div className="hidden md:block">
                      {/* Fixed Header */}
                      <table className="w-full text-sm table-fixed">
                        <thead className="bg-zinc-900">
                          <tr className="border-b border-zinc-800">
                            {[
                              "Player",
                              "Balance",
                              "Rounds",
                              "Wins",
                              "Folds",
                            ].map((col) => (
                              <th
                                key={col}
                                className="text-left px-6 py-4 text-zinc-500 text-xs font-medium uppercase tracking-wider"
                              >
                                {col}
                              </th>
                            ))}
                          </tr>
                        </thead>
                      </table>

                      {/* Scrollable Body */}
                      <div className="max-h-[calc(75vh-48px)] overflow-auto">
                        <table className="w-full text-sm table-fixed">
                          <tbody>
                            {statsRows.length === 0 ? (
                              <tr>
                                <td
                                  colSpan={5}
                                  className="px-6 py-12 text-center text-zinc-500"
                                >
                                  No statistics available
                                </td>
                              </tr>
                            ) : (
                              statsRows.map(
                                ([name, bal, rounds, wins, folds], i) => (
                                  <tr
                                    key={name}
                                    className={`border-b border-zinc-800/50 ${i % 2 === 1 ? "bg-zinc-800/20" : ""}`}
                                  >
                                    <td className="px-6 py-4 text-zinc-200 font-medium">
                                      {name}
                                    </td>
                                    <td className="px-6 py-4 text-blue-400 font-semibold">
                                      ${bal}
                                    </td>
                                    <td className="px-6 py-4 text-zinc-400">
                                      {rounds}
                                    </td>
                                    <td className="px-6 py-4 text-zinc-400">
                                      {wins}
                                    </td>
                                    <td className="px-6 py-4 text-zinc-400">
                                      {folds}
                                    </td>
                                  </tr>
                                ),
                              )
                            )}
                          </tbody>
                        </table>
                      </div>
                    </div>
                  </div>
                </CardContent>
              </Card>
            </div>
          )}

          {activePanel === "results" && (
            <div className="animate-fade-in">
              <h2 className="text-xl md:text-2xl font-bold text-zinc-100 mb-1">
                Round Results
              </h2>
              <p className="text-zinc-500 text-sm mb-4 md:mb-6">
                History of completed rounds
              </p>

              <Card>
                <CardContent className="p-0">
                  <div className="overflow-hidden rounded-xl">
                    {/* Mobile Card View */}
                    <div className="md:hidden max-h-[65vh] overflow-auto divide-y divide-zinc-800">
                      {resultsRows.length === 0 ? (
                        <div className="px-4 py-12 text-center text-zinc-500">
                          No results available
                        </div>
                      ) : (
                        resultsRows.map(
                          ([endedAt, winner, winningHand, amountWon], i) => (
                            <div key={i} className="p-4 space-y-2">
                              <div className="flex items-center justify-between">
                                <span className="text-zinc-200 font-medium">
                                  {winner}
                                </span>
                                <span className="text-blue-400 font-semibold">
                                  ${amountWon}
                                </span>
                              </div>
                              <div className="flex flex-col gap-1 text-xs text-zinc-500">
                                <span>
                                  Time:{" "}
                                  <span className="text-zinc-300">
                                    {formatDatetime(endedAt)}
                                  </span>
                                </span>
                                <span>
                                  Hand:{" "}
                                  <span className="text-zinc-300">
                                    {winningHand}
                                  </span>
                                </span>
                              </div>
                            </div>
                          ),
                        )
                      )}
                    </div>

                    {/* Desktop Table View */}
                    <div className="hidden md:block">
                      {/* Fixed Header */}
                      <table className="w-full text-sm table-fixed">
                        <thead className="bg-zinc-900">
                          <tr className="border-b border-zinc-800">
                            {["Time", "Winner", "Winning Hand", "Pot"].map(
                              (col) => (
                                <th
                                  key={col}
                                  className="text-left px-6 py-4 text-zinc-500 text-xs font-medium uppercase tracking-wider"
                                >
                                  {col}
                                </th>
                              ),
                            )}
                          </tr>
                        </thead>
                      </table>

                      {/* Scrollable Body */}
                      <div className="max-h-[calc(75vh-48px)] overflow-auto">
                        <table className="w-full text-sm table-fixed">
                          <tbody>
                            {resultsRows.length === 0 ? (
                              <tr>
                                <td
                                  colSpan={4}
                                  className="px-6 py-12 text-center text-zinc-500"
                                >
                                  No results available
                                </td>
                              </tr>
                            ) : (
                              resultsRows.map(
                                (
                                  [endedAt, winner, winningHand, amountWon],
                                  i,
                                ) => (
                                  <tr
                                    key={i}
                                    className={`border-b border-zinc-800/50 ${
                                      i % 2 === 1 ? "bg-zinc-800/20" : ""
                                    }`}
                                  >
                                    <td className="px-6 py-4 text-zinc-400 whitespace-nowrap">
                                      {formatDatetime(endedAt)}
                                    </td>
                                    <td className="px-6 py-4 text-zinc-200 font-medium">
                                      {winner}
                                    </td>
                                    <td className="px-6 py-4 text-zinc-300">
                                      {winningHand}
                                    </td>
                                    <td className="px-6 py-4 text-blue-400 font-semibold">
                                      ${amountWon}
                                    </td>
                                  </tr>
                                ),
                              )
                            )}
                          </tbody>
                        </table>
                      </div>
                    </div>
                  </div>
                </CardContent>
              </Card>
            </div>
          )}

          {activePanel === "deposit" && (
            <div className="animate-fade-in">
              <h2 className="text-xl md:text-2xl font-bold text-zinc-100 mb-1">
                Deposit
              </h2>
              <p className="text-zinc-500 text-sm mb-4 md:mb-6">
                Add funds to your account
              </p>

              <Card>
                <CardContent className="p-4 md:p-6 space-y-4">
                  <div className="flex flex-col">
                    <label className="text-sm font-medium text-zinc-300 mb-1.5">
                      Amount
                    </label>
                    <Input
                      placeholder="e.g. 500"
                      value={amountInput}
                      className="focus:ring-0 focus:border-zinc-700"
                      onChange={(e) => setAmountInput(e.target.value)}
                    />
                  </div>
                  <Button
                    className="w-full"
                    onClick={() => send(`DEPOSIT ${amountInput.trim()}`)}
                  >
                    <Download className="w-4 h-4" />
                    Confirm Deposit
                  </Button>
                  {responseMsg && (
                    <p className="text-sm text-blue-400 flex items-center gap-2">
                      <CheckCircle className="w-4 h-4" />
                      {responseMsg}
                    </p>
                  )}
                </CardContent>
              </Card>
            </div>
          )}

          {activePanel === "withdraw" && (
            <div className="animate-fade-in">
              <h2 className="text-xl md:text-2xl font-bold text-zinc-100 mb-1">
                Withdraw
              </h2>
              <p className="text-zinc-500 text-sm mb-4 md:mb-6">
                Remove funds from your account
              </p>

              <Card>
                <CardContent className="p-4 md:p-6 space-y-4">
                  <div className="flex flex-col">
                    <label className="text-sm font-medium text-zinc-300 mb-1.5">
                      Amount
                    </label>
                    <Input
                      placeholder="e.g. 200"
                      value={amountInput}
                      className="focus:ring-0 focus:border-zinc-700"
                      onChange={(e) => setAmountInput(e.target.value)}
                    />
                  </div>
                  <Button
                    className="w-full"
                    onClick={() => send(`WITHDRAW ${amountInput.trim()}`)}
                  >
                    <Upload className="w-4 h-4" />
                    Confirm Withdrawal
                  </Button>
                  {responseMsg && (
                    <p className="text-sm text-red-400 flex items-center gap-2">
                      <AlertTriangle className="w-4 h-4" />
                      {responseMsg}
                    </p>
                  )}
                </CardContent>
              </Card>
            </div>
          )}
        </div>
      </main>

      {/* Game Ended Modal */}
      <Dialog open={showGameEndedModal} onOpenChange={setShowGameEndedModal}>
        <DialogContent className="w-[90vw] max-w-sm sm:max-w-md p-4 sm:p-6 rounded-xl">
          <DialogHeader>
            <div className="w-16 h-16 rounded-full bg-red-500/10 border border-red-500/20 flex items-center justify-center mx-auto mb-4">
              <AlertTriangle className="w-8 h-8 text-red-500" />
            </div>
            <DialogTitle className="text-center">Game Over</DialogTitle>
            <DialogDescription className="text-center">
              Insufficient players to continue the game.
            </DialogDescription>
          </DialogHeader>
          <Button
            className="w-full"
            onClick={() => setShowGameEndedModal(false)}
          >
            Dismiss
          </Button>
        </DialogContent>
      </Dialog>
    </div>
  );
};
