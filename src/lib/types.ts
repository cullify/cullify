export type AppRoute = 'dashboard' | 'cull' | 'settings';

export type CullMode = 'quick' | 'expert' | 'arena';

export type ProjectStatus = 'done' | 'running' | 'paused';

export type Decision = 'keep' | 'cull' | 'auto' | null;

export type FilterMode = 'all' | 'keep' | 'cull' | 'pending';

export type ProviderId = string;

export interface Project {
  id: string;
  name: string;
  shortName: string;
  path: string;
  mode: CullMode;
  total: number;
  kept: number;
  culled: number;
  status: ProjectStatus;
  statusLabel: string;
  backend?: boolean;
}

export interface Photo {
  id: string;
  backendId?: string;
  sourceUrl?: string;
  originalUrl?: string;
  name: string;
  title: string;
  time: string;
  score: number;
  decision: Decision;
  palette: string;
  fileSize: string;
  camera: string;
  lens: string;
  focal: string;
  aperture: string;
  shutter: string;
  iso: string;
  size: string;
  group: string;
  reason: string;
  clarity: number;
  exposure: number;
  composition: number;
  faceScore: string;
}

export interface PhotoGroup {
  id: string;
  name: string;
  count: number;
}

export interface Provider {
  id: ProviderId;
  name: string;
  status: string;
  description: string;
  metaLeft: string;
  metaRight: string;
}

export interface VlmModel {
  id: string;
  name: string;
  file: string;
  size: string;
  speed: string;
  action: string;
  active?: boolean;
}

export interface Shortcut {
  id: string;
  action: string;
  scenario: string;
  keys: string[];
}
