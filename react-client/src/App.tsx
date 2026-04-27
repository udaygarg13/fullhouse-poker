import { useState, useEffect, useRef, useCallback } from 'react';
import type { Screen } from './types';
import { LoginScreen } from './components/LoginScreen';
import { MenuScreen } from './components/MenuScreen';
import { GameScreen } from './components/GameScreen';
import './index.css';

function App() {
  const [screen, setScreen] = useState<Screen>('LoginScreen');
  const [username, setUsername] = useState('');
  const serverUrl = import.meta.env.VITE_SERVER_URL || 'ws://127.0.0.1:7878';
  const [serverMessages, setServerMessages] = useState<string[]>([]);

  const wsRef = useRef<WebSocket | null>(null);
  const messageQueueRef = useRef<string[]>([]);

  useEffect(() => {
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
  }, []);

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

  return (
    <>
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
