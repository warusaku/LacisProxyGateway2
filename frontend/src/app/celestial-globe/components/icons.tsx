// CelestialGlobe v2 — Network Device Icons
// mobes2.0 icons.tsx (162行) 準拠 + LPG2追加タイプ
// SVGベースのアイコン。lucide-react のアイコンを使用。

'use client';

import React from 'react';
import {
  Globe,
  Router,
  Network,
  Wifi,
  Monitor,
  Server,
  Settings,
  Smartphone,
  CircuitBoard,
  Cable,
  ExternalLink,
  Shield,
  Cpu,
  Camera,
  Printer,
} from 'lucide-react';

export type DeviceIconType =
  | 'internet' | 'router' | 'switch' | 'ap' | 'client'
  | 'server' | 'controller' | 'bridge' | 'mobile'
  | 'unmanaged_switch' | 'iot' | 'camera' | 'printer'
  | 'logic_device' | 'external' | 'lpg_server' | 'wg_peer'
  | 'gateway';

// eslint-disable-next-line @typescript-eslint/no-explicit-any
const ICON_MAP: Record<DeviceIconType, React.ComponentType<any>> = {
  internet: Globe,
  router: Router,
  gateway: Router,
  switch: Network,
  unmanaged_switch: Network,
  ap: Wifi,
  client: Monitor,
  server: Server,
  controller: Settings,
  bridge: Cable,
  mobile: Smartphone,
  iot: CircuitBoard,
  camera: Camera,
  printer: Printer,
  logic_device: Cpu,
  external: ExternalLink,
  lpg_server: Shield,
  wg_peer: Globe,
};

interface NetworkDeviceIconProps {
  type: string;
  className?: string;
  size?: number;
}

export function NetworkDeviceIcon({ type, className, size = 20 }: NetworkDeviceIconProps) {
  const IconComponent = ICON_MAP[type as DeviceIconType] ?? Monitor;
  return <IconComponent className={className} size={size} />;
}
