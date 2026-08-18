import { useEffect, useState, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { Character, AuthCompletePayload, AuthErrorPayload, LiquidityRow } from './types';
import CharacterHeader from './components/CharacterHeader';
import LocationList from './components/LocationList';
import AssetDetail from './components/AssetDetail';

export default function App() {
  const [characters, setCharacters] = useState<Character[]>([]);
  const [activeChar, setActiveChar] = useState<Character | null>(null);
  const [rows, setRows] = useState<LiquidityRow[]>([]);
  const [selectedLocation, setSelectedLocation] = useState<LiquidityRow | null>(null);
  const [syncing, setSyncing] = useState(false);
  const [syncMessage, setSyncMessage] = useState<string>('');
  const [error, setError] = useState<string | null>(null);
  const [authPending, setAuthPending] = useState(false);

  const loadCharacters = useCallback(async () => {
    try {
      const chars = await invoke<Character[]>('get_characters');
      setCharacters(chars);
      if (chars.length > 0 && !activeChar) {
        setActiveChar(chars[0]);
      }
    } catch (e) {
      setError(String(e));
    }
  }, [activeChar]);

  const loadSummary = useCallback(async (charId: number) => {
    try {
      const data = await invoke<LiquidityRow[]>('get_liquidity_summary', { characterId: charId });
      setRows(data);
      setSelectedLocation(null);
    } catch (e) {
      setError(String(e));
    }
  }, []);

  // Initial load
  useEffect(() => {
    loadCharacters();
  }, []);

  // Re-load summary when active character changes
  useEffect(() => {
    if (activeChar) {
      loadSummary(activeChar.id);
    }
  }, [activeChar]);

  // SSO event listeners
  useEffect(() => {
    const unlistenComplete = listen<AuthCompletePayload>('auth-complete', (_event) => {
      setAuthPending(false);
      setError(null);
      loadCharacters();
    });
    const unlistenError = listen<AuthErrorPayload>('auth-error', (event) => {
      setAuthPending(false);
      setError(`Authentication failed: ${event.payload.message}`);
    });
    const unlistenProgress = listen<{ step: string; status: string; message?: string }>('sync-progress', (event) => {
      const { step, status, message } = event.payload;
      if (status === 'running') {
        setSyncMessage(message ?? `Syncing ${step}...`);
      } else if (status === 'complete') {
        setSyncMessage(`${step} ✓`);
      }
    });
    return () => {
      unlistenComplete.then((f) => f());
      unlistenError.then((f) => f());
      unlistenProgress.then((f) => f());
    };
  }, [loadCharacters]);

  const handleLogin = async () => {
    setAuthPending(true);
    setError(null);
    try {
      await invoke('login');
    } catch (e) {
      setAuthPending(false);
      setError(String(e));
    }
  };

  const handleSync = async () => {
    if (!activeChar) return;
    setSyncing(true);
    setSyncMessage('Starting sync...');
    setError(null);
    try {
      await invoke('sync_all', { characterId: activeChar.id });
      await loadSummary(activeChar.id);
      setSyncMessage('Sync complete');
    } catch (e) {
      setError(String(e));
      setSyncMessage('');
    } finally {
      setSyncing(false);
    }
  };

  const handleDeleteChar = async (charId: number) => {
    try {
      await invoke('delete_character', { characterId: charId });
      const updated = characters.filter((c) => c.id !== charId);
      setCharacters(updated);
      if (activeChar?.id === charId) {
        setActiveChar(updated[0] ?? null);
        setRows([]);
        setSelectedLocation(null);
      }
    } catch (e) {
      setError(String(e));
    }
  };

  return (
    <div className="app-shell">
      <CharacterHeader
        characters={characters}
        activeChar={activeChar}
        onSelectChar={setActiveChar}
        onAddChar={handleLogin}
        onDeleteChar={handleDeleteChar}
        authPending={authPending}
      />

      <main className="main-content">
        {error && (
          <div className="error-banner">
            <span>⚠ {error}</span>
            <button onClick={() => setError(null)}>✕</button>
          </div>
        )}

        {!activeChar ? (
          <div className="empty-state">
            <div className="empty-icon">🛸</div>
            <h2>No Characters Added</h2>
            <p>Add an EVE character to start tracking your assets.</p>
            <button className="btn btn-primary" onClick={handleLogin} disabled={authPending}>
              {authPending ? 'Waiting for browser login…' : '+ Add EVE Character'}
            </button>
          </div>
        ) : (
          <div className="content-grid">
            <div className="panel left-panel">
              <div className="panel-header">
                <h3>Dead Capital Locations</h3>
                <div className="panel-actions">
                  {syncMessage && <span className="sync-msg">{syncMessage}</span>}
                  <button
                    className="btn btn-sync"
                    onClick={handleSync}
                    disabled={syncing}
                  >
                    {syncing ? '⟳ Syncing…' : '⟳ Sync Now'}
                  </button>
                </div>
              </div>
              <LocationList
                rows={rows}
                selectedLocationId={selectedLocation?.location_id ?? null}
                onSelect={setSelectedLocation}
              />
            </div>

            <div className="panel right-panel">
              {selectedLocation ? (
                <AssetDetail
                  location={selectedLocation}
                  characterId={activeChar.id}
                />
              ) : (
                <div className="empty-detail">
                  <p>Select a location to view assets</p>
                </div>
              )}
            </div>
          </div>
        )}
      </main>
    </div>
  );
}
