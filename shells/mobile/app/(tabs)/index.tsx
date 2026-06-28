import React, { useState, useCallback, useMemo } from 'react';
import {
  View,
  Text,
  StyleSheet,
  useWindowDimensions,
  Pressable,
} from 'react-native';
import { useSafeAreaInsets } from 'react-native-safe-area-context';
import { router } from 'expo-router';

import { ArcadeCanvas } from '../../components/ArcadeCanvas';
import { useArcadeEngine, type FrameState } from '../../hooks/useArcadeEngine';
import { THEMES, SCENE_IDS, type SceneId } from '../../constants/themes';
import { useArcadeContext } from '../../context/ArcadeContext';

// ── Grid sizing ───────────────────────────────────────────────────────────────
const FONT_SIZE = 13;
const CELL_W = FONT_SIZE * 0.6;
const CELL_H = FONT_SIZE * 1.25;

const SCENE_LABELS: Record<SceneId, string> = {
  donut:  'DONUT',
  helix:  'HELIX',
  matrix: 'MATRIX',
  fire:   'FIRE',
  pipes:  'PIPES',
  life:   'LIFE',
  clock:  'CLOCK',
};

export default function SceneScreen() {
  const { width, height } = useWindowDimensions();
  const insets = useSafeAreaInsets();
  const { sceneId, setSceneId, themeName, setThemeName } = useArcadeContext();

  const drawH = height - insets.bottom;
  const cols = Math.floor(width / CELL_W);
  const rows = Math.floor(drawH / CELL_H);

  const [frame, setFrame] = useState<FrameState | null>(null);

  const theme = useMemo(
    () => THEMES.find((t) => t.name === themeName) ?? THEMES[0],
    [themeName],
  );

  const onFrame = useCallback((f: FrameState) => setFrame(f), []);

  useArcadeEngine({ scene: sceneId, theme: themeName, cols, rows, onFrame });

  const cycleScene = useCallback(() => {
    const idx = SCENE_IDS.indexOf(sceneId);
    setSceneId(SCENE_IDS[(idx + 1) % SCENE_IDS.length]);
  }, [sceneId, setSceneId]);

  const cycleTheme = useCallback(() => {
    const idx = THEMES.findIndex((t) => t.name === themeName);
    setThemeName(THEMES[(idx + 1) % THEMES.length].name);
  }, [themeName, setThemeName]);

  const canvasStyle = useMemo(
    () => ({ width: cols * CELL_W, height: rows * CELL_H }),
    [cols, rows],
  );

  return (
    <View style={[styles.root, { backgroundColor: theme.background }]}>
      <View style={canvasStyle}>
        <ArcadeCanvas
          frame={frame}
          theme={theme}
          cellWidth={CELL_W}
          cellHeight={CELL_H}
        />
      </View>

      <View style={[styles.hud, { bottom: insets.bottom + 4 }]}>
        <Pressable onPress={cycleScene} hitSlop={12}>
          <Text style={[styles.hudLabel, { color: theme.text }]}>
            {SCENE_LABELS[sceneId]}
          </Text>
        </Pressable>

        <View style={styles.hudRight}>
          <Pressable onPress={cycleTheme} hitSlop={12}>
            <Text style={[styles.hudBadge, { color: theme.text, borderColor: theme.text }]}>
              {themeName.toUpperCase()}
            </Text>
          </Pressable>

          <Pressable
            onPress={() => router.push('/(tabs)/settings')}
            hitSlop={12}
            style={styles.settingsBtn}
          >
            <Text style={[styles.settingsIcon, { color: theme.text }]}>⚙</Text>
          </Pressable>
        </View>
      </View>
    </View>
  );
}

const styles = StyleSheet.create({
  root: {
    flex: 1,
    alignItems: 'center',
    justifyContent: 'flex-start',
  },
  hud: {
    position: 'absolute',
    left: 12,
    right: 12,
    flexDirection: 'row',
    alignItems: 'center',
    justifyContent: 'space-between',
  },
  hudLabel: {
    fontFamily: 'monospace',
    fontSize: 11,
    letterSpacing: 2,
    opacity: 0.7,
  },
  hudRight: {
    flexDirection: 'row',
    alignItems: 'center',
    gap: 10,
  },
  hudBadge: {
    fontFamily: 'monospace',
    fontSize: 9,
    letterSpacing: 1.5,
    borderWidth: 1,
    paddingHorizontal: 5,
    paddingVertical: 2,
    opacity: 0.6,
  },
  settingsBtn: {
    opacity: 0.6,
  },
  settingsIcon: {
    fontSize: 16,
  },
});
