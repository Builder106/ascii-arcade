import React, { useCallback, useMemo, useRef } from 'react';
import { StyleSheet } from 'react-native';
import {
  Canvas,
  useCanvasRef,
  Skia,
  matchFont,
  type SkCanvas,
  type SkFont,
  type SkPaint,
} from '@shopify/react-native-skia';

import type { FrameState } from '../hooks/useArcadeEngine';
import type { Theme } from '../constants/themes';

interface Props {
  frame: FrameState | null;
  theme: Theme;
  cellWidth: number;
  cellHeight: number;
}

const BYTES_PER_CELL = 8;

function hexToSkColor(hex: string): number {
  const r = parseInt(hex.slice(1, 3), 16);
  const g = parseInt(hex.slice(3, 5), 16);
  const b = parseInt(hex.slice(5, 7), 16);
  // Skia colour: ARGB as a 32-bit int (alpha = 255)
  return (0xff << 24) | (r << 16) | (g << 8) | b;
}

/**
 * Renders a Frame (flat Uint8Array of cell data) onto a Skia canvas.
 *
 * Drawing is imperative and bypasses React's reconciler — the canvas ref
 * forces a re-draw whenever `frame` changes.  All draw calls run on the
 * main thread via react-native-skia's SkiaView.
 */
export function ArcadeCanvas({ frame, theme, cellWidth, cellHeight }: Props) {
  const canvasRef = useCanvasRef();

  // matchFont queries the system font manager for Courier New (bundled on iOS
  // and Android). Falls back to the default monospace when not found.
  const font: SkFont | null = useMemo(
    () =>
      matchFont(
        { fontFamily: 'Courier New', fontSize: cellHeight * 0.75 },
      ) ?? Skia.Font(null, cellHeight * 0.75),
    [cellHeight],
  );

  const textPaintRef = useRef<SkPaint | null>(null);
  const bgPaintRef = useRef<SkPaint | null>(null);

  const drawFrame = useCallback(
    (skCanvas: SkCanvas) => {
      if (!frame?.buffer || !font) return;

      const { buffer, cols, rows } = frame;
      const totalW = cols * cellWidth;
      const totalH = rows * cellHeight;

      // Background fill
      if (!bgPaintRef.current) {
        bgPaintRef.current = Skia.Paint();
      }
      bgPaintRef.current.setColor(hexToSkColor(theme.background));
      skCanvas.drawRect(Skia.XYWHRect(0, 0, totalW, totalH), bgPaintRef.current);

      // Text paint — colour is mutated per cell
      if (!textPaintRef.current) {
        textPaintRef.current = Skia.Paint();
        textPaintRef.current.setAntiAlias(true);
      }

      const themeColor = hexToSkColor(theme.text);
      const baseline = cellHeight * 0.82;

      for (let row = 0; row < rows; row++) {
        for (let col = 0; col < cols; col++) {
          const i = (row * cols + col) * BYTES_PER_CELL;

          // Decode Unicode scalar (u32 LE)
          const cp =
            buffer[i] |
            (buffer[i + 1] << 8) |
            (buffer[i + 2] << 16) |
            (buffer[i + 3] << 24);

          // Skip spaces and null chars — no draw needed
          if (cp === 0x20 || cp === 0) continue;

          const hasColor = buffer[i + 7] === 1;
          if (hasColor) {
            const color =
              (0xff << 24) |
              (buffer[i + 4] << 16) |
              (buffer[i + 5] << 8) |
              buffer[i + 6];
            textPaintRef.current!.setColor(color);
          } else {
            textPaintRef.current!.setColor(themeColor);
          }

          skCanvas.drawText(
            String.fromCodePoint(cp),
            col * cellWidth,
            row * cellHeight + baseline,
            textPaintRef.current!,
            font,
          );
        }
      }
    },
    [frame, font, theme, cellWidth, cellHeight],
  );

  // Trigger a redraw whenever frame data changes.
  React.useEffect(() => {
    canvasRef.current?.redraw();
  }, [frame, canvasRef]);

  return (
    <Canvas
      ref={canvasRef}
      style={StyleSheet.absoluteFill}
      onDraw={drawFrame}
    />
  );
}
