import { useState } from 'react';
import { Character } from '../types';

interface Props {
  characters: Character[];
  activeChar: Character | null;
  onSelectChar: (char: Character) => void;
  onAddChar: () => void;
  onDeleteChar: (charId: number) => void;
  authPending: boolean;
}

export default function CharacterHeader({
  characters,
  activeChar,
  onSelectChar,
  onAddChar,
  onDeleteChar,
  authPending,
}: Props) {
  const [dropdownOpen, setDropdownOpen] = useState(false);

  const portraitUrl = (charId: number) =>
    `https://images.evetech.net/characters/${charId}/portrait?size=64`;

  return (
    <header className="app-header">
      <div className="header-brand">
        <span className="brand-icon">⊛</span>
        <span className="brand-name">HangarSweep</span>
      </div>

      <div className="header-character">
        {activeChar ? (
          <div className="char-selector">
            <button
              className="char-chip"
              onClick={() => setDropdownOpen((o) => !o)}
            >
              <img
                src={portraitUrl(activeChar.id)}
                alt={activeChar.name}
                className="char-portrait"
                onError={(e) => { (e.target as HTMLImageElement).style.display = 'none'; }}
              />
              <span className="char-name">{activeChar.name}</span>
              <span className="char-caret">{dropdownOpen ? '▲' : '▼'}</span>
            </button>

            {dropdownOpen && (
              <div className="char-dropdown">
                {characters.map((c) => (
                  <div
                    key={c.id}
                    className={`char-option ${c.id === activeChar.id ? 'active' : ''}`}
                  >
                    <button
                      className="char-option-select"
                      onClick={() => {
                        onSelectChar(c);
                        setDropdownOpen(false);
                      }}
                    >
                      <img
                        src={portraitUrl(c.id)}
                        alt={c.name}
                        className="char-portrait-sm"
                        onError={(e) => { (e.target as HTMLImageElement).style.display = 'none'; }}
                      />
                      {c.name}
                    </button>
                    <button
                      className="char-delete"
                      title="Remove character"
                      onClick={() => {
                        onDeleteChar(c.id);
                        setDropdownOpen(false);
                      }}
                    >
                      ✕
                    </button>
                  </div>
                ))}
                <div className="char-dropdown-footer">
                  <button
                    className="btn btn-add-char"
                    onClick={() => {
                      onAddChar();
                      setDropdownOpen(false);
                    }}
                    disabled={authPending}
                  >
                    {authPending ? 'Waiting…' : '+ Add Character'}
                  </button>
                </div>
              </div>
            )}
          </div>
        ) : (
          <button
            className="btn btn-primary"
            onClick={onAddChar}
            disabled={authPending}
          >
            {authPending ? 'Waiting for login…' : '+ Add Character'}
          </button>
        )}
      </div>
    </header>
  );
}
