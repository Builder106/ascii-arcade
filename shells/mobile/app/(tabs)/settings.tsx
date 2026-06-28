import React from 'react';
import {
  View,
  Text,
  Pressable,
  ScrollView,
  StyleSheet,
} from 'react-native';
import { useSafeAreaInsets } from 'react-native-safe-area-context';
import { router } from 'expo-router';

import { THEMES, SCENE_IDS, type SceneId, type ThemeName } from '../../constants/themes';
import { useArcadeContext } from '../../context/ArcadeContext';

const SCENE_LABELS: Record<SceneId, string> = {
  donut:  'Donut',
  helix:  'Helix',
  matrix: 'Matrix Rain',
  fire:   'Fire',
  pipes:  'Pipes',
  life:   'Game of Life',
  clock:  'Clock',
};

const SCENE_DESCRIPTIONS: Record<SceneId, string> = {
  donut:  '3D torus spinning in ASCII space',
  helix:  'Double helix twisting through the grid',
  matrix: 'Falling glyph columns keyed to the active theme',
  fire:   'Upward-spreading flame simulation',
  pipes:  'Box-drawing pipes that crawl and branch',
  life:   "Conway's Game of Life seeded at random",
  clock:  'Live digital clock in block ASCII',
};

export default function SettingsScreen() {
  const insets = useSafeAreaInsets();
  const { sceneId, setSceneId, themeName, setThemeName } = useArcadeContext();

  return (
    <View style={[styles.root, { paddingTop: insets.top + 12, paddingBottom: insets.bottom }]}>
      <View style={styles.header}>
        <Text style={styles.title}>Settings</Text>
        <Pressable onPress={() => router.back()} hitSlop={12}>
          <Text style={styles.done}>Done</Text>
        </Pressable>
      </View>

      <ScrollView contentContainerStyle={styles.scroll} showsVerticalScrollIndicator={false}>

        {/* ── Theme ── */}
        <Text style={styles.sectionHeader}>THEME</Text>
        <View style={styles.card}>
          {THEMES.map((theme, idx) => (
            <Pressable
              key={theme.name}
              style={[styles.row, idx > 0 && styles.rowBorder]}
              onPress={() => setThemeName(theme.name as ThemeName)}
            >
              <View style={[styles.swatch, { backgroundColor: theme.text }]} />
              <Text style={styles.rowLabel}>{theme.name}</Text>
              {themeName === theme.name && (
                <Text style={styles.check}>✓</Text>
              )}
            </Pressable>
          ))}
        </View>

        {/* ── Scene ── */}
        <Text style={styles.sectionHeader}>SCENE</Text>
        <View style={styles.card}>
          {SCENE_IDS.map((id, idx) => (
            <Pressable
              key={id}
              style={[styles.row, idx > 0 && styles.rowBorder]}
              onPress={() => setSceneId(id as SceneId)}
            >
              <View style={styles.rowText}>
                <Text style={styles.rowLabel}>{SCENE_LABELS[id]}</Text>
                <Text style={styles.rowSub}>{SCENE_DESCRIPTIONS[id]}</Text>
              </View>
              {sceneId === id && (
                <Text style={styles.check}>✓</Text>
              )}
            </Pressable>
          ))}
        </View>

        {/* ── About ── */}
        <Text style={styles.sectionHeader}>ABOUT</Text>
        <View style={styles.card}>
          <View style={styles.row}>
            <Text style={styles.rowLabel}>Version</Text>
            <Text style={styles.rowValue}>0.1.0</Text>
          </View>
          <View style={[styles.row, styles.rowBorder]}>
            <Text style={styles.rowLabel}>Engine</Text>
            <Text style={styles.rowValue}>aa-core (Rust)</Text>
          </View>
          <View style={[styles.row, styles.rowBorder]}>
            <Text style={styles.rowLabel}>Renderer</Text>
            <Text style={styles.rowValue}>react-native-skia</Text>
          </View>
        </View>

      </ScrollView>
    </View>
  );
}

const BG = '#0a0a0a';
const CARD = '#141414';
const BORDER = '#222';
const TEXT = '#e0e0e0';
const MUTED = '#555';
const ACCENT = '#30d158';

const styles = StyleSheet.create({
  root: {
    flex: 1,
    backgroundColor: BG,
  },
  header: {
    flexDirection: 'row',
    alignItems: 'center',
    justifyContent: 'space-between',
    paddingHorizontal: 20,
    paddingBottom: 16,
    borderBottomWidth: StyleSheet.hairlineWidth,
    borderBottomColor: BORDER,
  },
  title: {
    color: TEXT,
    fontSize: 20,
    fontWeight: '600',
    letterSpacing: 0.3,
  },
  done: {
    color: ACCENT,
    fontSize: 16,
  },
  scroll: {
    padding: 20,
    gap: 8,
  },
  sectionHeader: {
    color: MUTED,
    fontSize: 11,
    letterSpacing: 1.5,
    marginTop: 16,
    marginBottom: 6,
    marginLeft: 4,
  },
  card: {
    backgroundColor: CARD,
    borderRadius: 10,
    borderWidth: StyleSheet.hairlineWidth,
    borderColor: BORDER,
    overflow: 'hidden',
  },
  row: {
    flexDirection: 'row',
    alignItems: 'center',
    paddingHorizontal: 16,
    paddingVertical: 13,
    gap: 12,
  },
  rowBorder: {
    borderTopWidth: StyleSheet.hairlineWidth,
    borderTopColor: BORDER,
  },
  rowText: {
    flex: 1,
    gap: 2,
  },
  rowLabel: {
    color: TEXT,
    fontSize: 15,
    flex: 1,
  },
  rowSub: {
    color: MUTED,
    fontSize: 12,
  },
  rowValue: {
    color: MUTED,
    fontSize: 14,
  },
  swatch: {
    width: 14,
    height: 14,
    borderRadius: 7,
  },
  check: {
    color: ACCENT,
    fontSize: 16,
  },
});
