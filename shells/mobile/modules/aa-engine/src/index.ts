import { requireNativeModule } from 'expo-modules-core';

export interface AaEngineNative {
  /** Construct the engine for the given scene id. Returns true on success. */
  create(sceneId: string): boolean;
  /** Tear down the engine. */
  destroy(): void;
  /** Resize the character grid. Must be called after create() and before the first nextFrame(). */
  setGrid(width: number, height: number): void;
  /** Set the active colour theme by name ("Hacker" | "Amber" | "Ice" | "Ghost"). */
  setTheme(themeName: string): void;
  /** Forward a scene-specific setting. */
  applySetting(id: string, value: number): void;
  /**
   * Render the next frame at time t (seconds from app start).
   *
   * Returns an ArrayBuffer of `width * height * 8` bytes.
   * Each cell occupies 8 bytes:
   *   [0–3]  Unicode codepoint as uint32 LE
   *   [4]    R
   *   [5]    G
   *   [6]    B
   *   [7]    hasColor (1 = use RGB above, 0 = use theme text colour)
   *
   * Returns null if the engine is not initialised.
   */
  nextFrame(t: number): ArrayBuffer | null;
  /** List the built-in scene ids in display order. */
  sceneNames(): string[];
}

const NativeModule = requireNativeModule<AaEngineNative>('AaEngine');

// ── Public typed API ──────────────────────────────────────────────────────────

export function create(sceneId: string): boolean {
  return NativeModule.create(sceneId);
}

export function destroy(): void {
  NativeModule.destroy();
}

export function setGrid(width: number, height: number): void {
  NativeModule.setGrid(width, height);
}

export function setTheme(themeName: string): void {
  NativeModule.setTheme(themeName);
}

export function applySetting(id: string, value: number): void {
  NativeModule.applySetting(id, value);
}

export function nextFrame(t: number): ArrayBuffer | null {
  return NativeModule.nextFrame(t);
}

export function sceneNames(): string[] {
  return NativeModule.sceneNames();
}
