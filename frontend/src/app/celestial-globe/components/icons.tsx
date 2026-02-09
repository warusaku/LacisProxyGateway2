/**
 * Network Device Icons — mobes2.0 icons.tsx SVGベース移植
 *
 * lucide-react不使用。SVG pathで直接描画。
 * LPG2追加: external, lpg_server, wg_peer
 */

import type { NodeType } from '../types';

interface IconProps {
  className?: string;
}

// mobes2.0 renderLegacyIcon 相当
const ICON_PATHS: Record<string, string> = {
  // Cloud (internet)
  internet: 'M3 15a4 4 0 004 4h9a5 5 0 10-.1-9.999 5.002 5.002 0 10-9.78 2.096A4.001 4.001 0 003 15z',
  // Globe (controller/gateway/router)
  controller: 'M21 12a9 9 0 01-9 9m9-9a9 9 0 00-9-9m9 9H3m9 9a9 9 0 01-9-9m9 9c1.657 0 3-4.03 3-9s-1.343-9-3-9m0 18c-1.657 0-3-4.03-3-9s1.343-9 3-9m-9 9a9 9 0 019-9',
  gateway: 'M21 12a9 9 0 01-9 9m9-9a9 9 0 00-9-9m9 9H3m9 9a9 9 0 01-9-9m9 9c1.657 0 3-4.03 3-9s-1.343-9-3-9m0 18c-1.657 0-3-4.03-3-9s1.343-9 3-9m-9 9a9 9 0 019-9',
  router: 'M21 12a9 9 0 01-9 9m9-9a9 9 0 00-9-9m9 9H3m9 9a9 9 0 01-9-9m9 9c1.657 0 3-4.03 3-9s-1.343-9-3-9m0 18c-1.657 0-3-4.03-3-9s1.343-9 3-9m-9 9a9 9 0 019-9',
  // GitBranch (switch)
  switch: 'M6 3v12M18 9a3 3 0 100-6 3 3 0 000 6zm0 0v3a2 2 0 01-2 2H8a2 2 0 01-2-2V9M6 21a3 3 0 100-6 3 3 0 000 6z',
  // Wifi (ap)
  ap: 'M8.111 16.404a5.5 5.5 0 017.778 0M12 20h.01m-7.08-7.071c3.904-3.905 10.236-3.905 14.141 0M1.394 9.393c5.857-5.858 15.355-5.858 21.213 0',
  // Monitor (client)
  client: 'M9.75 17L9 20l-1 1h8l-1-1-.75-3M3 13h18M5 17h14a2 2 0 002-2V5a2 2 0 00-2-2H5a2 2 0 00-2 2v10a2 2 0 002 2z',
  // Shield (wg_peer)
  wg_peer: 'M9 12l2 2 4-4m5.618-4.016A11.955 11.955 0 0112 2.944a11.955 11.955 0 01-8.618 3.04A12.02 12.02 0 003 9c0 5.591 3.824 10.29 9 11.622 5.176-1.332 9-6.03 9-11.622 0-1.042-.133-2.052-.382-3.016z',
  // Box (logic_device)
  logic_device: 'M20 7l-8-4-8 4m16 0l-8 4m8-4v10l-8 4m0-10L4 7m8 4v10M4 7v10l8 4',
  // HardDrive (external)
  external: 'M22 12H2M5.45 5.11L2 12v6a2 2 0 002 2h16a2 2 0 002-2v-6l-3.45-6.89A2 2 0 0016.76 4H7.24a2 2 0 00-1.79 1.11zM6 16h.01M10 16h.01',
  // Server (lpg_server)
  lpg_server: 'M5 12h14M5 12a2 2 0 01-2-2V6a2 2 0 012-2h14a2 2 0 012 2v4a2 2 0 01-2 2M5 12a2 2 0 00-2 2v4a2 2 0 002 2h14a2 2 0 002-2v-4a2 2 0 00-2-2m-2-4h.01M17 16h.01',
};

export function NetworkDeviceIcon({ nodeType, className = 'w-5 h-5' }: { nodeType: NodeType; className?: string }) {
  const path = ICON_PATHS[nodeType] || ICON_PATHS.client;

  return (
    <svg
      className={className}
      fill="none"
      stroke="currentColor"
      viewBox="0 0 24 24"
      strokeLinecap="round"
      strokeLinejoin="round"
      strokeWidth={1.5}
    >
      <path d={path} />
    </svg>
  );
}

export function renderNetworkIcon(nodeType: NodeType, className?: string) {
  return <NetworkDeviceIcon nodeType={nodeType} className={className} />;
}
