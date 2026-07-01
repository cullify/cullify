import { createContext, type Snippet } from 'svelte';
import type { AppRoute, Project } from '$lib/types';

export type ResourceMode = {
  cpu: string;
  memory: string;
  gpu: string;
};

export type ShellTask = {
  id: string;
  label: string;
  detail: string;
  progress: number | null;
};

export type FooterStats = {
  photo: string;
  analysis: string;
  model: string;
  resources: ResourceMode;
};

const defaultResources: ResourceMode = {
  cpu: 'CPU 待机',
  memory: '内存 待机',
  gpu: '显存 待机'
};

export class ShellController {
  active = $state<AppRoute>('dashboard');
  title = $state('Cullify');
  subtitle = $state('本地离线 AI 照片选片');
  cullCount = $state('0');
  recentProjects = $state<Project[]>([]);
  sidebarExtra = $state<Snippet | null>(null);
  showDefaultSidebarDetails = $state(true);
  task = $state<ShellTask | null>(null);
  footer = $state<FooterStats>({
    photo: '照片 0 / 0',
    analysis: '分析待机',
    model: '模型未占用',
    resources: { ...defaultResources }
  });

  configure(next: {
    active?: AppRoute;
    title?: string;
    subtitle?: string;
    cullCount?: string;
    recentProjects?: Project[];
    sidebarExtra?: Snippet | null;
    showDefaultSidebarDetails?: boolean;
    footer?: Partial<FooterStats> & { resources?: Partial<ResourceMode> };
  }) {
    if (next.active) this.active = next.active;
    if (next.title !== undefined) this.title = next.title;
    if (next.subtitle !== undefined) this.subtitle = next.subtitle;
    if (next.cullCount !== undefined) this.cullCount = next.cullCount;
    if (next.recentProjects !== undefined) this.recentProjects = next.recentProjects;
    if (next.sidebarExtra !== undefined) this.sidebarExtra = next.sidebarExtra;
    if (next.showDefaultSidebarDetails !== undefined) this.showDefaultSidebarDetails = next.showDefaultSidebarDetails;
    if (next.footer) this.setFooter(next.footer);
  }

  setFooter(next: Partial<FooterStats> & { resources?: Partial<ResourceMode> }) {
    this.footer = {
      photo: next.photo ?? this.footer.photo,
      analysis: next.analysis ?? this.footer.analysis,
      model: next.model ?? this.footer.model,
      resources: {
        cpu: next.resources?.cpu ?? this.footer.resources.cpu,
        memory: next.resources?.memory ?? this.footer.resources.memory,
        gpu: next.resources?.gpu ?? this.footer.resources.gpu
      }
    };
  }

  startTask(task: ShellTask) {
    this.task = task;
  }

  finishTask(id: string, detail = '完成') {
    if (this.task?.id !== id) return;
    this.task = { ...this.task, detail, progress: 100 };
    window.setTimeout(() => {
      if (this.task?.id === id) this.task = null;
    }, 1800);
  }

  failTask(id: string, detail: string) {
    if (this.task?.id !== id) return;
    this.task = { ...this.task, detail, progress: null };
    window.setTimeout(() => {
      if (this.task?.id === id) this.task = null;
    }, 3200);
  }

  resetPage() {
    this.active = 'dashboard';
    this.title = 'Cullify';
    this.subtitle = '本地离线 AI 照片选片';
    this.cullCount = '0';
    this.recentProjects = [];
    this.sidebarExtra = null;
    this.showDefaultSidebarDetails = true;
    this.footer = {
      photo: '照片 0 / 0',
      analysis: '分析待机',
      model: this.task ? this.footer.model : '模型未占用',
      resources: { ...defaultResources }
    };
  }
}

export const [getShellContext, setShellContext] = createContext<ShellController>();
