import { convertFileSrc, invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import { revealItemInDir } from '@tauri-apps/plugin-opener';
import type { Photo, Project, ProjectStatus, ProviderId } from './types';

export interface BackendProject {
  id: string;
  name: string;
  mode: string;
  status: string;
  folderPath: string;
  totalPhotos: number;
  keptCount: number;
  culledCount: number;
  createdAt: string;
  updatedAt: string;
}

export interface BackendPhoto {
  id: string;
  projectId: string;
  originalName: string;
  filePath: string;
  thumbnailPath: string | null;
  fileSize: number;
  width: number | null;
  height: number | null;
  mimeType: string;
  analysisStatus: string;
  qualityScore: number | null;
  isBlurred: boolean;
  blurScore: number | null;
  isOverexposed: boolean;
  isUnderexposed: boolean;
  exposureScore: number | null;
  userDecision: 'keep' | 'cull' | null;
  isAutoCulled: boolean;
  perceptualHash: string | null;
  groupLabel: string;
  capturedAt: string | null;
  camera: string | null;
  lens: string | null;
  focal: string | null;
  aperture: string | null;
  shutter: string | null;
  iso: string | null;
  createdAt: string;
}

export interface ScanSummary {
  project: BackendProject;
  inserted: number;
  skipped: number;
}

export type CreateProjectOutcome =
  | { status: 'created'; summary: ScanSummary }
  | { status: 'cancelled' }
  | { status: 'failed'; error: string };

export type ExportProjectOutcome =
  | { status: 'exported'; summary: ExportSummary }
  | { status: 'cancelled' }
  | { status: 'failed'; error: string };

export interface CreateProjectRequest {
  folderPath: string;
  name?: string;
  mode?: 'quick' | 'expert' | 'arena';
}

export interface BatchDecisionRequest {
  projectId: string;
  photoIds: string[];
  decision: 'keep' | 'cull' | null;
}

export interface ShortcutConfig {
  id: string;
  keys: string[];
}

export interface AppConfig {
  providerId: ProviderId;
  activeModelId: string;
  blurThreshold: number;
  exposureTolerance: number;
  cullLine: number;
  vlmThreads: number;
  arenaTarget: string;
  autoGroup: boolean;
  gpuMetal: boolean;
  shortcuts: ShortcutConfig[];
}

export interface AppConfigEnvelope {
  config: AppConfig;
  configPath: string;
  appDataDir: string;
}

export interface ExportSummary {
  exportDir: string;
  jsonPath: string;
  csvPath: string;
  zipPath: string;
  total: number;
  kept: number;
  culled: number;
  pending: number;
}

export const defaultAppConfig: AppConfig = {
  providerId: 'builtin',
  activeModelId: 'gemma-3-4b',
  blurThreshold: 100,
  exposureTolerance: 0.018,
  cullLine: 40,
  vlmThreads: 3,
  arenaTarget: '20%',
  autoGroup: true,
  gpuMetal: true,
  shortcuts: [
    { id: 'keep', keys: ['K'] },
    { id: 'cull', keys: ['X'] },
    { id: 'nav', keys: ['Left', 'Right'] },
    { id: 'skip', keys: ['S'] },
    { id: 'arena', keys: ['A', 'D'] },
    { id: 'mark', keys: ['Space'] },
    { id: 'fullscreen', keys: ['F'] },
    { id: 'grid', keys: ['G'] },
    { id: 'undo', keys: ['Cmd', 'Z'] },
    { id: 'all', keys: ['Up', 'Down'] }
  ]
};

export function listProjects() {
  return invoke<BackendProject[]>('list_projects');
}

export function listPhotos(projectId: string) {
  return invoke<BackendPhoto[]>('list_photos', { projectId });
}

export function createProjectFromFolder(request: CreateProjectRequest) {
  return invoke<ScanSummary>('create_project_from_folder', { request });
}

export function renameProject(projectId: string, name: string) {
  return invoke<BackendProject>('rename_project', { projectId, name });
}

export function deleteProject(projectId: string) {
  return invoke<void>('delete_project', { projectId });
}

export function setPhotoDecision(photoId: string, decision: 'keep' | 'cull' | null) {
  return invoke<BackendPhoto>('set_photo_decision', { photoId, decision });
}

export function setPhotoDecisions(request: BatchDecisionRequest) {
  return invoke<BackendPhoto[]>('set_photo_decisions', { request });
}

export function loadAppConfig() {
  return invoke<AppConfigEnvelope>('load_app_config');
}

export function saveAppConfig(config: AppConfig) {
  return invoke<AppConfigEnvelope>('save_app_config', { config });
}

export function exportProject(projectId: string, exportRoot?: string) {
  const args = exportRoot ? { projectId, exportRoot } : { projectId };
  return invoke<ExportSummary>('export_project', args);
}

export async function pickImageFolder() {
  const selected = await open({
    directory: true,
    multiple: false,
    title: '选择照片文件夹'
  });

  return typeof selected === 'string' ? selected : null;
}

export async function pickExportFolder() {
  const selected = await open({
    directory: true,
    multiple: false,
    title: '选择导出文件夹'
  });

  return typeof selected === 'string' ? selected : null;
}

export async function tryListProjects() {
  try {
    return await listProjects();
  } catch (error) {
    console.info('Cullify backend unavailable, using mock projects.', error);
    return null;
  }
}

export async function tryListPhotos(projectId: string) {
  try {
    return await listPhotos(projectId);
  } catch (error) {
    console.info('Cullify backend unavailable, using mock photos.', error);
    return null;
  }
}

export async function tryCreateProjectFromPickedFolder(mode: 'quick' | 'expert' | 'arena' = 'quick') {
  const outcome = await createProjectFromPickedFolderWithOutcome(mode);
  return outcome.status === 'created' ? outcome.summary : null;
}

export async function createProjectFromPickedFolderWithOutcome(
  mode: 'quick' | 'expert' | 'arena' = 'quick'
): Promise<CreateProjectOutcome> {
  try {
    const folderPath = await pickImageFolder();
    if (!folderPath) return { status: 'cancelled' };

    const summary = await createProjectFromFolder({ folderPath, mode });
    return { status: 'created', summary };
  } catch (error) {
    console.info('Unable to create project from folder.', error);
    return { status: 'failed', error: readableError(error) };
  }
}

export async function tryLoadAppConfig() {
  try {
    return await loadAppConfig();
  } catch (error) {
    console.info('Cullify config backend unavailable, using default config.', error);
    return null;
  }
}

export async function trySaveAppConfig(config: AppConfig) {
  try {
    return await saveAppConfig(config);
  } catch (error) {
    console.info('Unable to save Cullify config.', error);
    return null;
  }
}

export async function tryExportProject(projectId: string) {
  const outcome = await exportProjectWithOutcome(projectId);
  return outcome.status === 'exported' ? outcome.summary : null;
}

export async function exportProjectWithOutcome(
  projectId: string,
  exportRoot?: string
): Promise<ExportProjectOutcome> {
  try {
    const summary = await exportProject(projectId, exportRoot);
    return { status: 'exported', summary };
  } catch (error) {
    console.info('Unable to export Cullify project.', error);
    return { status: 'failed', error: readableError(error) };
  }
}

export async function exportProjectWithPickedFolder(projectId: string): Promise<ExportProjectOutcome> {
  try {
    const exportRoot = await pickExportFolder();
    if (!exportRoot) return { status: 'cancelled' };
    return await exportProjectWithOutcome(projectId, exportRoot);
  } catch (error) {
    console.info('Unable to export Cullify project to picked folder.', error);
    return { status: 'failed', error: readableError(error) };
  }
}

export async function tryRevealInFileManager(path: string) {
  try {
    await revealItemInDir(path);
    return true;
  } catch (error) {
    console.info('Unable to reveal exported file.', error);
    return false;
  }
}

export function backendProjectToProject(project: BackendProject): Project {
  const mode = normalizeMode(project.mode);
  return {
    id: project.id,
    name: project.name,
    shortName: project.name,
    path: project.folderPath,
    mode,
    total: project.totalPhotos,
    kept: project.keptCount,
    culled: project.culledCount,
    status: normalizeProjectStatus(project.status),
    statusLabel: project.status === 'ready' ? '已就绪' : project.status,
    backend: true
  };
}

export function backendPhotoToPhoto(photo: BackendPhoto, index: number): Photo {
  const score = Math.round(photo.qualityScore ?? 50);
  const decision = photo.userDecision ?? (photo.isAutoCulled ? 'auto' : null);

  return {
    id: photo.id,
    backendId: photo.id,
    sourceUrl: convertFileSrc(photo.thumbnailPath || photo.filePath),
    originalUrl: convertFileSrc(photo.filePath),
    name: stripExtension(photo.originalName),
    title: photo.isAutoCulled ? '快速模式建议淘汰' : '快速模式候选',
    time: formatCaptureTime(photo.capturedAt, index),
    score,
    decision,
    palette: paletteForScore(score, index),
    fileSize: formatBytes(photo.fileSize),
    camera: photo.camera || '未知相机',
    lens: photo.lens || '未知镜头',
    focal: photo.focal || '—',
    aperture: photo.aperture || '—',
    shutter: photo.shutter || '—',
    iso: photo.iso || '—',
    size: photo.width && photo.height ? `${photo.width} x ${photo.height}` : '—',
    group: photo.groupLabel || '未分组',
    reason: reasonForPhoto(photo),
    clarity: photo.blurScore ?? 0,
    exposure: photo.exposureScore ?? 0.5,
    composition: Math.max(0.2, Math.min(0.95, score / 100)),
    faceScore: '—'
  };
}

function normalizeMode(mode: string): Project['mode'] {
  if (mode === 'expert' || mode === 'arena') return mode;
  return 'quick';
}

function normalizeProjectStatus(status: string): ProjectStatus {
  if (status === 'running' || status === 'analyzing') return 'running';
  if (status === 'paused') return 'paused';
  return 'done';
}

function stripExtension(name: string) {
  return name.replace(/\.[^.]+$/, '');
}

function formatBytes(value: number) {
  if (value < 1024 * 1024) return `${Math.max(1, Math.round(value / 1024))} KB`;
  return `${(value / 1024 / 1024).toFixed(1)} MB`;
}

function formatCaptureTime(value: string | null, index: number) {
  if (!value) return String(index + 1).padStart(2, '0');
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return String(index + 1).padStart(2, '0');
  return date.toLocaleString('zh-CN', {
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
    hour12: false
  });
}

function readableError(error: unknown): string {
  if (error instanceof Error) return error.message;
  if (typeof error === 'string') return error;
  return 'unknown error';
}

function reasonForPhoto(photo: BackendPhoto) {
  const reasons = [];
  if (photo.isBlurred) reasons.push('清晰度偏低');
  if (photo.isOverexposed) reasons.push('高光过曝');
  if (photo.isUnderexposed) reasons.push('整体欠曝');
  if (!reasons.length) return '快速模式建议保留 · 清晰度和曝光位于可交付区间，可继续人工复核。';
  return `快速模式建议淘汰 · ${reasons.join('、')}。`;
}

function paletteForScore(score: number, index: number) {
  const high = [
    'linear-gradient(135deg,#8a9aa3 0%,#56636f 35%,#2d3845 70%,#1a2230 100%)',
    'linear-gradient(135deg,#a89a7e 0%,#7c6f56 30%,#5a4f3d 60%,#3d3528 100%)'
  ];
  const mid = [
    'linear-gradient(135deg,#c8a87a 0%,#a08560 40%,#6f5a3e 80%,#3d3220 100%)',
    'linear-gradient(135deg,#9a9088 0%,#605650 50%,#3a3530 100%)'
  ];
  const low = [
    'linear-gradient(135deg,#c8c3b8 0%,#aaa69c 50%,#858279 100%)',
    'linear-gradient(135deg,#6a7080 0%,#3d4452 50%,#1f242f 100%)'
  ];
  const set = score >= 75 ? high : score >= 50 ? mid : low;
  return set[index % set.length];
}
