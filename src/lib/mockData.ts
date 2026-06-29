import type { Photo, PhotoGroup, Project, Provider, Shortcut, VlmModel } from './types';

export const projects: Project[] = [
  {
    id: 'wedding-0427',
    name: '婚礼跟拍 · 0427',
    shortName: '婚礼 0427',
    path: '~/Pictures/2026-04-27-婚礼/原片',
    mode: 'quick',
    total: 1247,
    kept: 412,
    culled: 835,
    status: 'done',
    statusLabel: '已完成'
  },
  {
    id: 'western-sichuan-day2',
    name: '川西旅拍 · Day 2',
    shortName: '川西 Day2',
    path: '~/Pictures/2026-05-12-川西/Day2-稻城',
    mode: 'quick',
    total: 384,
    kept: 156,
    culled: 228,
    status: 'done',
    statusLabel: '已完成'
  },
  {
    id: 'product-06',
    name: '产品静物 · 06 系列',
    shortName: '产品 06',
    path: '~/Shoot/2026-06-产品06/RAW',
    mode: 'quick',
    total: 92,
    kept: 24,
    culled: 47,
    status: 'running',
    statusLabel: '第 4 轮'
  },
  {
    id: 'portrait-linxi',
    name: '人像约拍 · 林夕',
    shortName: '林夕人像',
    path: '~/Pictures/2026-06-18-林夕/连拍',
    mode: 'quick',
    total: 580,
    kept: 0,
    culled: 0,
    status: 'paused',
    statusLabel: '已暂停'
  }
];

export const photoGroups: PhotoGroup[] = [
  { id: 'all', name: '全部照片', count: 1247 },
  { id: 'arrival', name: '迎宾 · 入场', count: 186 },
  { id: 'rings', name: '仪式 · 交换戒指', count: 142 },
  { id: 'vows', name: '誓言 · 致辞', count: 98 },
  { id: 'family', name: '合影 · 双方家', count: 213 },
  { id: 'toast', name: '宴席 · 敬酒', count: 287 },
  { id: 'bouquet', name: '抛花 · 送客', count: 321 }
];

const palettes = [
  'linear-gradient(135deg,#a89a7e 0%,#7c6f56 30%,#5a4f3d 60%,#3d3528 100%)',
  'linear-gradient(135deg,#8a9aa3 0%,#56636f 35%,#2d3845 70%,#1a2230 100%)',
  'linear-gradient(135deg,#c8a87a 0%,#a08560 40%,#6f5a3e 80%,#3d3220 100%)',
  'linear-gradient(135deg,#d4b894 0%,#a8907a 50%,#75614f 100%)',
  'linear-gradient(135deg,#6a7080 0%,#3d4452 50%,#1f242f 100%)',
  'linear-gradient(135deg,#b8a085 0%,#806c54 50%,#4d3f30 100%)',
  'linear-gradient(135deg,#9a9088 0%,#605650 50%,#3a3530 100%)',
  'linear-gradient(135deg,#c0a590 0%,#8a7058 50%,#50402f 100%)'
];

const photoSeed: Array<[number, number, Photo['decision']]> = [
  [421, 87, 'keep'],
  [422, 91, 'keep'],
  [423, 38, 'cull'],
  [424, 54, null],
  [425, 82, 'keep'],
  [426, 29, 'auto'],
  [427, 76, 'keep'],
  [428, 88, null],
  [429, 42, 'cull'],
  [430, 93, 'keep'],
  [431, 61, null],
  [432, 79, 'keep'],
  [433, 35, 'auto'],
  [434, 85, 'keep'],
  [435, 58, null],
  [436, 31, 'cull'],
  [437, 89, 'keep'],
  [438, 72, null],
  [439, 66, null],
  [440, 95, 'keep'],
  [441, 47, 'cull'],
  [442, 83, null],
  [443, 52, null],
  [444, 78, 'keep']
];

export const photos: Photo[] = photoSeed.map(([seq, score, decision], index) => ({
  id: `img-${seq}`,
  name: `IMG_${seq}`,
  title: index === 1 ? '仪式交换戒指瞬间' : index % 3 === 0 ? '仪式现场候选' : '连拍候选',
  time: `16:42:${String(3 + index * 2).padStart(2, '0')}`,
  score,
  decision,
  palette: palettes[index % palettes.length],
  fileSize: `${(38 + index * 0.7).toFixed(1)} MB`,
  camera: 'Sony A7M4',
  lens: 'FE 70-200 GM II',
  focal: index % 2 === 0 ? '135 mm' : '105 mm',
  aperture: index % 3 === 0 ? 'f/2.8' : 'f/2.0',
  shutter: '1/400 s',
  iso: index % 4 === 0 ? '640' : '800',
  size: '7008 x 4672',
  group: '仪式 · 交换戒指',
  reason:
    score >= 80
      ? '建议保留 · 焦点精准、表情自然。主体关系明确，背景干净，情绪和光线都有可交付价值。'
      : score < 45
        ? '建议淘汰 · 清晰度或曝光存在明显问题。同组中已有更好的替代照片。'
        : '建议人工复核 · 技术质量可用，但情绪或构图优势不明显。',
  clarity: Math.min(0.99, Math.max(0.25, score / 100 + 0.05)),
  exposure: Math.min(0.98, Math.max(0.3, score / 100 - 0.03)),
  composition: Math.min(0.96, Math.max(0.28, score / 100 - 0.05)),
  faceScore: score > 70 ? '2/2' : score > 50 ? '1/2' : '0/2'
}));

export const providers: Provider[] = [
  {
    id: 'builtin',
    name: '内置 llama.cpp',
    status: '已加载',
    description: 'Rust 原生质量评分链路。快速模式无需模型，适合本地离线初筛和可交付导出。',
    metaLeft: 'native scanner',
    metaRight: '~0 ms'
  },
  {
    id: 'ollama',
    name: 'Ollama',
    status: '未连接',
    description: '通过 HTTP 与本地 Ollama 服务通信。适合作为 VLM 集成的第一条稳定路径。',
    metaLeft: 'localhost:11434',
    metaRight: '~5 ms'
  },
  {
    id: 'openai',
    name: 'OpenAI 兼容',
    status: '未配置',
    description: 'LM Studio / vLLM / TabbyAPI 等兼容端点。需要用户明确配置地址与密钥。',
    metaLeft: '/v1/chat/completions',
    metaRight: '取决于端点'
  }
];

export const models: VlmModel[] = [
  {
    id: 'gemma-3-4b',
    name: 'Gemma 3 4B · Q4_K_M',
    file: 'gemma-3-4b-it-Q4_K_M.gguf · 3.2 GB · 默认推荐',
    size: '3.2 GB',
    speed: '~2.4 s/张\n4 GB 显存',
    action: '已激活',
    active: true
  },
  {
    id: 'qwen-2-5-vl-7b',
    name: 'Qwen 2.5 VL 7B · Q4',
    file: 'qwen2.5-vl-7b-instruct-q4.gguf · 中文质量最高',
    size: '5.1 GB',
    speed: '~4.8 s/张\n6 GB 显存',
    action: '切换'
  },
  {
    id: 'smolvlm-256m',
    name: 'SmolVLM 256M · Q8',
    file: 'smolvlm-256m-instruct-q8.gguf · 低配电脑纯 CPU 可跑',
    size: '210 MB',
    speed: '~0.6 s/张\n512 MB',
    action: '切换'
  },
  {
    id: 'internvl3-2b',
    name: 'InternVL3 2B · Q4',
    file: 'internvl3-2b-q4.gguf · 超小体积',
    size: '1.4 GB',
    speed: '~2.8 s/张\n2 GB 显存',
    action: '下载'
  }
];

export const shortcuts: Shortcut[] = [
  { id: 'keep', action: '保留当前照片', scenario: '挑选界面', keys: ['K'] },
  { id: 'cull', action: '淘汰当前照片', scenario: '挑选界面', keys: ['X'] },
  { id: 'nav', action: '上一张 / 下一张', scenario: '挑选界面', keys: ['Left', 'Right'] },
  { id: 'skip', action: '跳过', scenario: '通用', keys: ['S'] },
  { id: 'arena', action: '选 A / 选 B', scenario: '竞技场', keys: ['A', 'D'] },
  { id: 'mark', action: '标记 / 取消标记', scenario: '挑选界面', keys: ['Space'] },
  { id: 'fullscreen', action: '全屏灯箱', scenario: '通用', keys: ['F'] },
  { id: 'grid', action: '切换网格 / 列表', scenario: '挑选界面', keys: ['G'] },
  { id: 'undo', action: '撤销', scenario: '通用', keys: ['Cmd', 'Z'] },
  { id: 'all', action: '全要 · 全不要', scenario: '竞技场', keys: ['Up', 'Down'] }
];
