import { useState, useEffect, useRef, useCallback } from 'react';
import type { Screen } from './types';
import { ConnectScreen } from './components/ConnectScreen';
import { LoginScreen } from './components/LoginScreen';
import { MenuScreen } from './components/MenuScreen';
import { GameScreen } from './components/GameScreen';
import './index.css';

function App() {
  const [screen, setScreen] = useState<Screen>('ConnectScreen');
  const [username, setUsername] = useState('');
  const [serverUrl, setServerUrl] = useState<string | null>(null);
  const [serverMessages, setServerMessages] = useState<string[]>([]);

  const wsRef = useRef<WebSocket | null>(null);
  const messageQueueRef = useRef<string[]>([]);

  useEffect(() => {
    if (!serverUrl) return;

    const ws = new WebSocket(serverUrl);
    wsRef.current = ws;

    ws.onopen = () => {
      console.log('WebSocket connected');
      while (messageQueueRef.current.length > 0) {
        const msg = messageQueueRef.current.shift();
        if (msg) ws.send(msg);
      }
    };

    ws.onmessage = (event) => {
      setServerMessages((prev) => [...prev, event.data]);
    };

    ws.onclose = () => {
      console.log('WebSocket disconnected');
    };

    ws.onerror = (error) => {
      console.error('WebSocket error:', error);
    };

    return () => {
      ws.close();
    };
  }, [serverUrl]);

  // Stable send function - never changes reference
  const send = useCallback((message: string) => {
    if (wsRef.current?.readyState === WebSocket.OPEN) {
      wsRef.current.send(message);
    } else {
      messageQueueRef.current.push(message);
    }
  }, []);

  const wsHandle = useRef({ send }).current;
  // Keep wsHandle.send up to date without recreating the object
  wsHandle.send = send;

  // Stable clearMessages - never changes reference
  const clearMessages = useCallback(() => {
    setServerMessages([]);
  }, []);

  const handleConnect = (url: string) => {
    setServerUrl(url);
    setScreen('LoginScreen');
  };

  return (
    <>
      {screen === 'ConnectScreen' && <ConnectScreen onConnect={handleConnect} />}
      {screen === 'LoginScreen' && (
        <LoginScreen
          wsHandle={wsHandle}
          serverMessages={serverMessages}
          setServerMessages={setServerMessages}
          setScreen={setScreen}
          setUsername={setUsername}
          clearMessages={clearMessages}
        />
      )}
      {screen === 'MenuScreen' && (
        <MenuScreen
          wsHandle={wsHandle}
          serverMessages={serverMessages}
          setServerMessages={setServerMessages}
          setScreen={setScreen}
          username={username}
          setUsername={setUsername}
        />
      )}
      {screen === 'GameScreen' && (
        <GameScreen
          wsHandle={wsHandle}
          serverMessages={serverMessages}
          setServerMessages={setServerMessages}
          username={username}
          setScreen={setScreen}
        />
      )}
    </>
  );
}

export default App;
