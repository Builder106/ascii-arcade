export type ThemeName = 'Hacker' | 'Amber' | 'Ice' | 'Ghost';

export interface Theme {
  name: ThemeName;
  text: string;      // CSS hex colour for Skia paint
  background: string;
}

export const THEMES: Theme[] = [
  { name: 'Hacker', text: '#30d158', background: '#000000' },
  { name: 'Amber',  text: '#ffa600', background: '#1a0800' },
  { name: 'Ice',    text: '#00ffff', background: '#00000d' },  // #00000d ≈ #000d1a
  { name: 'Ghost',  text: '#1c1c1e', background: '#f5f5f5' },
];

export const DEFAULT_THEME = THEMES[0];

export const SCENE_IDS = ['donut', 'helix', 'matrix', 'fire', 'pipes', 'life', 'clock'] as const;
export type SceneId = typeof SCENE_IDS[number];
