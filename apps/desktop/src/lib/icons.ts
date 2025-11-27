/**
 * Icon Library Configuration
 * 
 * 主力: Iconify (@iconify/react)
 * - 提供超过 200,000 个图标
 * - 按需加载，性能优异
 * - 统一的 API 接口
 * 
 * 备选: Tabler Icons (@tabler/icons-react)
 * - 提供一致的设计风格
 * - React 组件方式使用
 * - 适合需要特定风格的场景
 */

// Iconify 使用示例
// import { Icon } from '@iconify/react';
// <Icon icon="mdi:home" />
// <Icon icon="lucide:settings" width="24" height="24" />

// Tabler Icons 使用示例
// import { IconHome } from '@tabler/icons-react';
// <IconHome size={24} stroke={1.5} />

export const ICON_DEFAULTS = {
  size: 24,
  stroke: 1.5,
};

/**
 * 推荐的图标集合（Iconify）：
 * - mdi: Material Design Icons
 * - lucide: Lucide Icons (现代、简洁)
 * - tabler: Tabler Icons
 * - carbon: IBM Carbon Icons
 * - heroicons: Heroicons
 * - ph: Phosphor Icons
 */

export const ICON_COLLECTIONS = {
  primary: 'lucide',  // 主要使用 Lucide 图标集
  fallback: 'mdi',    // 如果找不到则使用 Material Design Icons
} as const;
