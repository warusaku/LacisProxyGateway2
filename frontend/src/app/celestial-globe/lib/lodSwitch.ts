// CelestialGlobe v2 — LOD Switch
// mobes2.0 lib/lodSwitch.ts (117行) 準拠
// ズームレベルに応じて data-lod 属性を切替。CSS制御のため React 再レンダリング不要。

export type LodLevel = 'low' | 'mid' | 'high' | 'full';

export interface LodSwitchOptions {
  /** low→mid 切替閾値 (enter) */
  lowToMid?: number;
  /** mid→low 切替閾値 (exit, ヒステリシス) */
  midToLow?: number;
  /** mid→high 切替閾値 (enter) */
  midToHigh?: number;
  /** high→mid 切替閾値 (exit, ヒステリシス) */
  highToMid?: number;
  /** high→full 切替閾値 (enter) */
  highToFull?: number;
  /** full→high 切替閾値 (exit, ヒステリシス) */
  fullToHigh?: number;
}

export interface ViewportProvider {
  getZoom: () => number;
}

const DEFAULT_OPTIONS: Required<LodSwitchOptions> = {
  lowToMid: 0.40,
  midToLow: 0.35,
  midToHigh: 0.90,
  highToMid: 0.85,
  highToFull: 1.40,
  fullToHigh: 1.30,
};

/**
 * canvas要素にLOD属性をバインドし、ズームレベルの変化を監視する。
 * @returns cleanup関数
 */
export function bindLodSwitch(
  canvasEl: HTMLElement | null,
  viewportProvider: ViewportProvider,
  options?: LodSwitchOptions,
): () => void {
  if (!canvasEl) return () => {};

  const opts = { ...DEFAULT_OPTIONS, ...options };
  let currentLod: LodLevel = 'mid';
  let lastZoom = -1;
  let lastVelocity = 0;
  let lastTime = performance.now();
  let rafId: number | null = null;

  function determineLod(zoom: number): LodLevel {
    // ヒステリシス: 現在のLODに応じて異なる閾値を使用
    switch (currentLod) {
      case 'low':
        if (zoom >= opts.lowToMid) return 'mid';
        return 'low';
      case 'mid':
        if (zoom < opts.midToLow) return 'low';
        if (zoom >= opts.midToHigh) return 'high';
        return 'mid';
      case 'high':
        if (zoom < opts.highToMid) return 'mid';
        if (zoom >= opts.highToFull) return 'full';
        return 'high';
      case 'full':
        if (zoom < opts.fullToHigh) return 'high';
        return 'full';
      default:
        return 'mid';
    }
  }

  function tick() {
    const zoom = viewportProvider.getZoom();
    const now = performance.now();
    const dt = now - lastTime;

    if (dt > 0 && lastZoom >= 0) {
      lastVelocity = Math.abs(zoom - lastZoom) / dt * 1000;
    }

    lastTime = now;
    lastZoom = zoom;

    // velocity gate: 高速ズーム中は切替抑制
    if (lastVelocity < 2.0) {
      const newLod = determineLod(zoom);
      if (newLod !== currentLod) {
        currentLod = newLod;
        canvasEl!.setAttribute('data-lod', currentLod);
      }
    }

    rafId = requestAnimationFrame(tick);
  }

  // 初期LOD設定
  const initialZoom = viewportProvider.getZoom();
  currentLod = determineLod(initialZoom);
  canvasEl.setAttribute('data-lod', currentLod);

  rafId = requestAnimationFrame(tick);

  return () => {
    if (rafId !== null) {
      cancelAnimationFrame(rafId);
    }
  };
}
