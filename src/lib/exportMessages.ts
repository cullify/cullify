export function describeExportError(error: string) {
  if (error.includes('source photo is missing')) {
    return '导出失败：原始照片已移动或删除。请恢复项目文件夹后重试。';
  }

  if (error.includes('project has no photos to export')) {
    return '导出失败：当前项目没有可导出的照片。';
  }

  if (error.includes('project not found')) {
    return '导出失败：找不到当前项目，请回到主控台重新选择项目。';
  }

  if (error.includes('__TAURI__') || error.includes('invoke') || error.includes('not available')) {
    return '导出真实项目需要在 Tauri 桌面环境中运行。';
  }

  return `导出失败：${error}`;
}
