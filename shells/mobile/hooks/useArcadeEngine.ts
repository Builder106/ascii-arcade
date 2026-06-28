import { useEffect, useRef, useCallback } from 'react';
import * as AaEngine from 'aa-engine';
import type { SceneId, ThemeName } from '../constants/themes';

export interface FrameState {
  buffer: Uint8Array | null;
  cols: number;
  rows: number;
}

interface Options {
  scene: SceneId;
  theme: ThemeName;
  cols: number;
  rows: number;
  onFrame: (state: FrameState) => void;
}

/**
 * Drives the aa-core engine at ~30 fps and calls onFrame with each new frame.
 *
 * Grid dimensions (cols × rows) should be stable between renders; resize
 * events should trigger a new invocation of this hook via a changed key.
 */
export function useArcadeEngine({ scene, theme, cols, rows, onFrame }: Options) {
  // Stable ref so the animation loop closure doesn't capture a stale callback.
  const onFrameRef = useRef(onFrame);
  onFrameRef.current = onFrame;

  // Track whether the engine is ready to serve frames.
  const readyRef = useRef(false);

  const startTime = useRef(0);
  const rafHandle = useRef<number>(0);
  const lastFrameTime = useRef(0);
  const FRAME_INTERVAL_MS = 1000 / 30;

  const loop = useCallback((timestamp: number) => {
    if (!readyRef.current) {
      rafHandle.current = requestAnimationFrame(loop);
      return;
    }

    if (timestamp - lastFrameTime.current >= FRAME_INTERVAL_MS) {
      lastFrameTime.current = timestamp;
      const t = (timestamp - startTime.current) / 1000;
      const buf = AaEngine.nextFrame(t);
      if (buf) {
        onFrameRef.current({
          buffer: new Uint8Array(buf),
          cols,
          rows,
        });
      }
    }

    rafHandle.current = requestAnimationFrame(loop);
  }, [cols, rows]); // cols/rows captured at hook initialisation

  useEffect(() => {
    readyRef.current = false;

    const ok = AaEngine.create(scene);
    if (!ok) return;

    AaEngine.setGrid(cols, rows);
    AaEngine.setTheme(theme);
    readyRef.current = true;
    startTime.current = performance.now();
    lastFrameTime.current = 0;

    rafHandle.current = requestAnimationFrame(loop);

    return () => {
      cancelAnimationFrame(rafHandle.current);
      AaEngine.destroy();
      readyRef.current = false;
    };
  }, [scene, theme, cols, rows, loop]);
}
