import { useEffect, useState } from 'react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Loader2, AlertCircle } from 'lucide-react';

interface ConnectScreenProps {
  onConnect: (address: string) => void;
  isConnecting?: boolean;
}

export const ConnectScreen = ({ onConnect, isConnecting = false }: ConnectScreenProps) => {
  const [serverInput, setServerInput] = useState('127.0.0.1:7878');
  const [errorMsg, setErrorMsg] = useState('');
  const [keyboardOffset, setKeyboardOffset] = useState(0);

  useEffect(() => {
    if (typeof window === 'undefined' || !window.visualViewport) return;

    const viewport = window.visualViewport;

    const updateKeyboardOffset = () => {
      const viewportHeight = viewport.height + viewport.offsetTop;
      const keyboardHeight = Math.max(0, window.innerHeight - viewportHeight);
      setKeyboardOffset(keyboardHeight > 0 ? Math.min(keyboardHeight * 0.45, 180) : 0);
    };

    updateKeyboardOffset();
    viewport.addEventListener('resize', updateKeyboardOffset);
    viewport.addEventListener('scroll', updateKeyboardOffset);

    return () => {
      viewport.removeEventListener('resize', updateKeyboardOffset);
      viewport.removeEventListener('scroll', updateKeyboardOffset);
    };
  }, []);

  const normalizeWsUrl = (addr: string): string => {
    if (addr.startsWith('ws://') || addr.startsWith('wss://')) {
      return addr;
    }
    return `ws://${addr}`;
  };

  const handleSubmit = () => {
    const raw = serverInput.trim();
    if (!raw) {
      setErrorMsg('Please enter a server address.');
      return;
    }
    setErrorMsg('');
    onConnect(normalizeWsUrl(raw));
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter' && !isConnecting) {
      handleSubmit();
    }
  };

  return (
    <div className="relative min-h-svh overflow-y-auto overflow-x-hidden flex items-center justify-center p-4">
      <div
        className="w-full max-w-md animate-slide-up transition-transform duration-200 ease-out"
        style={{ transform: `translateY(-${keyboardOffset}px)` }}
      >
        <div className="text-center mb-8">
          <h1 className="text-3xl font-bold text-zinc-100 mb-2">Poker</h1>
          <p className="text-zinc-500 text-xs tracking-widest uppercase">Team Fullhouse</p>
        </div>

        <Card className="border-transparent bg-transparent shadow-none backdrop-blur-0">
          <CardHeader>
            <CardTitle>Connect to Server</CardTitle>
            <CardDescription>Enter the server address</CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            <div className="space-y-2">
              <label className="text-sm font-medium text-zinc-300">Server Address</label>
              <Input
                type="text"
                value={serverInput}
                onChange={(e) => setServerInput(e.target.value)}
                onKeyDown={handleKeyDown}
                placeholder="127.0.0.1:7878"
                className="focus:ring-0 focus:border-zinc-700"
                autoCapitalize="none"
                autoCorrect="off"
                spellCheck={false}
                disabled={isConnecting}
              />
            </div>

            {errorMsg && (
              <div className="flex items-center gap-2 p-3 rounded-lg bg-red-500/10 border border-red-500/20 text-red-400 text-sm">
                <AlertCircle className="w-4 h-4 flex-shrink-0" />
                <span>{errorMsg}</span>
              </div>
            )}

            <Button
              onClick={handleSubmit}
              disabled={isConnecting || !serverInput.trim()}
              className="w-full"
              size="lg"
            >
              {isConnecting ? (
                <>
                  <Loader2 className="w-4 h-4 animate-spin" />
                  Connecting...
                </>
              ) : (
                <>
                  Connect
                </>
              )}
            </Button>
          </CardContent>
        </Card>
      </div>
    </div>
  );
};
