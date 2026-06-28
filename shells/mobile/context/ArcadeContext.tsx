import React, { createContext, useContext, useState } from 'react';
import { SCENE_IDS, THEMES, type SceneId, type ThemeName } from '../constants/themes';

interface ArcadeContextValue {
  sceneId: SceneId;
  setSceneId: (id: SceneId) => void;
  themeName: ThemeName;
  setThemeName: (name: ThemeName) => void;
}

const ArcadeContext = createContext<ArcadeContextValue | null>(null);

export function ArcadeProvider({ children }: { children: React.ReactNode }) {
  const [sceneId, setSceneId] = useState<SceneId>(SCENE_IDS[2]); // matrix
  const [themeName, setThemeName] = useState<ThemeName>(THEMES[0].name); // Hacker

  return (
    <ArcadeContext.Provider value={{ sceneId, setSceneId, themeName, setThemeName }}>
      {children}
    </ArcadeContext.Provider>
  );
}

export function useArcadeContext(): ArcadeContextValue {
  const ctx = useContext(ArcadeContext);
  if (!ctx) throw new Error('useArcadeContext must be used inside ArcadeProvider');
  return ctx;
}
