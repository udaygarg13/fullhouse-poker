import {
  useState,
  useEffect,
  useRef,
  type Dispatch,
  type SetStateAction,
} from "react";
import type { Screen } from "../types";
import { Card, CardContent, CardHeader } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import {
  User,
  Lock,
  Loader2,
  AlertCircle,
  CircleCheck,
  ArrowLeft,
} from "lucide-react";

interface LoginScreenProps {
  wsHandle: { send: (message: string) => void } | null;
  serverMessages: string[];
  setServerMessages: Dispatch<SetStateAction<string[]>>;
  setScreen: (screen: Screen) => void;
  setUsername: (username: string) => void;
  clearMessages: () => void;
}

export const LoginScreen = ({
  wsHandle,
  serverMessages,
  setServerMessages,
  setScreen,
  setUsername,
}: LoginScreenProps) => {
  const [userInput, setUserInput] = useState("");
  const [pinInput, setPinInput] = useState("");
  const [errorMsg, setErrorMsg] = useState("");
  const [tab, setTab] = useState<"login" | "create">("login");
  const [isLoading, setIsLoading] = useState(false);

  // Keep stable refs so the effect closure always sees the latest values
  // without needing them in the dependency array
  const tabRef = useRef(tab);
  const userInputRef = useRef(userInput);
  tabRef.current = tab;
  userInputRef.current = userInput;

  useEffect(() => {
    if (serverMessages.length === 0) return;
    const snapshotLength = serverMessages.length;

    for (const resp of serverMessages) {
      if (resp.startsWith("OK")) {
        const body = resp.slice(3).trim();

        if (body.toLowerCase().includes("account created")) {
          setErrorMsg("Account created!");
          setIsLoading(false);
          continue;
        }

        if (tabRef.current === "login") {
          setUsername(userInputRef.current.trim());
          // Preserve anything that arrived after this snapshot while stripping
          // the login response frames from the snapshot we just handled.
          setServerMessages((prev) => {
            const cutoff = Math.min(snapshotLength, prev.length);
            const processed = prev.slice(0, cutoff);
            const remaining = prev.slice(cutoff);

            return [
              ...processed.filter(
                (msg) => !msg.startsWith("OK") && !msg.startsWith("ERR"),
              ),
              ...remaining,
            ];
          });
          setScreen("MenuScreen");
          return; // Important: return immediately to prevent further processing
        }
      } else if (resp.startsWith("ERR")) {
        const body = resp.slice(4).trim();
        setErrorMsg(body);
        setIsLoading(false);
      }
    }

    // Only consume the snapshot we processed. Later messages stay queued.
    setServerMessages((prev) => {
      const cutoff = Math.min(snapshotLength, prev.length);
      const processed = prev.slice(0, cutoff);
      const remaining = prev.slice(cutoff);

      return [
        ...processed.filter(
          (msg) => !msg.startsWith("OK") && !msg.startsWith("ERR"),
        ),
        ...remaining,
      ];
    });
  }, [serverMessages]); // eslint-disable-line react-hooks/exhaustive-deps

  const handleSubmit = () => {
    if (!wsHandle) return;
    const u = userInput.trim();
    const p = pinInput.trim();

    if (!u || !p) {
      setErrorMsg("Please fill in all fields");
      return;
    }

    setErrorMsg("");
    setIsLoading(true);

    if (tab === "login") {
      wsHandle.send(`LOGIN ${u} ${p}`);
    } else {
      wsHandle.send(`CREATE ${u} ${p}`);
    }
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Enter" && !isLoading) {
      handleSubmit();
    }
  };

  const isSuccess = errorMsg.includes("Account created");

  return (
    <div className="relative min-h-svh overflow-y-auto overflow-x-hidden flex items-center justify-center p-4">
      <Button
        variant="ghost"
        size="sm"
        className="absolute top-4 left-4"
        onClick={() => setScreen("ConnectScreen")}
      >
        <ArrowLeft className="w-4 h-4" />
      </Button>

      <div className="w-full max-w-md animate-slide-up">
        <div className="text-center mb-8">
          <h1 className="text-3xl font-bold text-zinc-100 mb-2">Poker</h1>
          <p className="text-zinc-500 text-xs tracking-widest uppercase">
            Team Fullhouse
          </p>
        </div>

        <Card className="border-transparent bg-transparent shadow-none backdrop-blur-0">
          <CardHeader className="pb-4">
            <Tabs
              value={tab}
              onValueChange={(v) => {
                setTab(v as "login" | "create");
                setErrorMsg("");
              }}
              className="w-full"
            >
              <TabsList className="grid w-full grid-cols-2">
                <TabsTrigger value="login">Sign In</TabsTrigger>
                <TabsTrigger value="create">Create Account</TabsTrigger>
              </TabsList>
            </Tabs>
          </CardHeader>
          <CardContent>
            <Tabs
              value={tab}
              onValueChange={(v) => {
                setTab(v as "login" | "create");
                setErrorMsg("");
              }}
            >
              <TabsContent value="login" className="mt-0 space-y-4">
                <div className="space-y-2">
                  <label className="text-sm font-medium text-zinc-300">
                    Username
                  </label>
                  <div className="relative">
                    <User className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-zinc-500" />
                    <Input
                      type="text"
                      value={userInput}
                      onChange={(e) => setUserInput(e.target.value)}
                      onKeyDown={handleKeyDown}
                      placeholder="Enter your username"
                      className="pl-10 focus:ring-0 focus:border-zinc-700"
                      autoCapitalize="none"
                      autoCorrect="off"
                      spellCheck={false}
                      disabled={isLoading}
                    />
                  </div>
                </div>

                <div className="space-y-2">
                  <label className="text-sm font-medium text-zinc-300">
                    PIN
                  </label>
                  <div className="relative">
                    <Lock className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-zinc-500" />
                    <Input
                      type="password"
                      value={pinInput}
                      onChange={(e) => setPinInput(e.target.value)}
                      onKeyDown={handleKeyDown}
                      placeholder="Enter your PIN"
                      className="pl-10 focus:ring-0 focus:border-zinc-700"
                      inputMode="numeric"
                      pattern="[0-9]*"
                      disabled={isLoading}
                    />
                  </div>
                </div>

                {errorMsg && (
                  <div
                    className={`flex items-center gap-2 p-3 rounded-lg text-sm ${
                      isSuccess
                        ? "bg-blue-500/10 border border-blue-500/20 text-blue-400"
                        : "bg-red-500/10 border border-red-500/20 text-red-400"
                    }`}
                  >
                    {isSuccess ? (
                      <CircleCheck className="w-4 h-4 flex-shrink-0" />
                    ) : (
                      <AlertCircle className="w-4 h-4 flex-shrink-0" />
                    )}
                    <span>{errorMsg}</span>
                  </div>
                )}

                <Button
                  onClick={handleSubmit}
                  disabled={isLoading}
                  className="w-full"
                  size="lg"
                >
                  {isLoading ? (
                    <>
                      <Loader2 className="w-4 h-4 animate-spin" />
                      Signing in...
                    </>
                  ) : (
                    "Sign In"
                  )}
                </Button>
              </TabsContent>

              <TabsContent value="create" className="mt-0 space-y-4">
                <div className="space-y-2">
                  <label className="text-sm font-medium text-zinc-300">
                    Username
                  </label>
                  <div className="relative">
                    <User className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-zinc-500" />
                    <Input
                      type="text"
                      value={userInput}
                      onChange={(e) => setUserInput(e.target.value)}
                      onKeyDown={handleKeyDown}
                      placeholder="Choose a username"
                      className="pl-10 focus:ring-0 focus:border-zinc-700"
                      autoCapitalize="none"
                      autoCorrect="off"
                      spellCheck={false}
                      disabled={isLoading}
                    />
                  </div>
                </div>

                <div className="space-y-2">
                  <label className="text-sm font-medium text-zinc-300">
                    PIN
                  </label>
                  <div className="relative">
                    <Lock className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-zinc-500" />
                    <Input
                      type="password"
                      value={pinInput}
                      onChange={(e) => setPinInput(e.target.value)}
                      onKeyDown={handleKeyDown}
                      placeholder="Create a PIN"
                      className="pl-10 focus:ring-0 focus:border-zinc-700"
                      inputMode="numeric"
                      pattern="[0-9]*"
                      disabled={isLoading}
                    />
                  </div>
                </div>

                {errorMsg && (
                  <div
                    className={`flex items-center gap-2 p-3 rounded-lg text-sm ${
                      isSuccess
                        ? "bg-blue-500/10 border border-blue-500/20 text-blue-400"
                        : "bg-red-500/10 border border-red-500/20 text-red-400"
                    }`}
                  >
                    {isSuccess ? (
                      <CircleCheck className="w-4 h-4 flex-shrink-0" />
                    ) : (
                      <AlertCircle className="w-4 h-4 flex-shrink-0" />
                    )}
                    <span>{errorMsg}</span>
                  </div>
                )}

                <Button
                  onClick={handleSubmit}
                  disabled={isLoading}
                  className="w-full"
                  size="lg"
                >
                  {isLoading ? (
                    <>
                      <Loader2 className="w-4 h-4 animate-spin" />
                      Creating account...
                    </>
                  ) : (
                    "Create Account"
                  )}
                </Button>
              </TabsContent>
            </Tabs>
          </CardContent>
        </Card>
      </div>
    </div>
  );
};
