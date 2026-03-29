import { createContext, useContext, useEffect, useState } from 'react';

export type Theme = 'light' | 'dark' | 'system';
export type ThemeStyle =
  | 'default'
  | 'graphite'
  | 'emerald'
  | 'sunset'
  | 'midnight'
  | 'ocean'
  | 'violet';
export type ThemeSkin =
  | 'soft'
  | 'pill'
  | 'sharp'
  | 'glass'
  | 'prism'
  | 'hud'
  | 'cyber'
  | 'cyber-darkline';

interface ThemeContextType {
  theme: Theme;
  setTheme: (theme: Theme) => void;
  style: ThemeStyle;
  setThemeStyle: (style: ThemeStyle) => void;
  skin: ThemeSkin;
  setThemeSkin: (skin: ThemeSkin) => void;
}

const ThemeContext = createContext<ThemeContextType | undefined>(undefined);

const THEME_STORAGE_KEY = 'neuradock-theme';
const THEME_STYLE_STORAGE_KEY = 'neuradock-theme-style';
const THEME_SKIN_STORAGE_KEY = 'neuradock-theme-skin';
const THEME_STYLES: ThemeStyle[] = [
  'default',
  'graphite',
  'emerald',
  'sunset',
  'midnight',
  'ocean',
  'violet',
];
const THEME_SKINS: ThemeSkin[] = [
  'soft',
  'pill',
  'sharp',
  'glass',
  'prism',
  'hud',
  'cyber',
  'cyber-darkline',
];
const THEME_STYLE_CLASSES = [
  'theme-default',
  'theme-graphite',
  'theme-emerald',
  'theme-sunset',
  'theme-midnight',
  'theme-ocean',
  'theme-violet',
];
const THEME_SKIN_CLASSES = [
  'skin-default',
  'skin-soft',
  'skin-pill',
  'skin-sharp',
  'skin-glass',
  'skin-prism',
  'skin-hud',
  'skin-cyber',
  'skin-cyber-darkline',
  // Legacy classes kept for safe cleanup of persisted old values.
  'skin-compact',
  'skin-brutalist',
  'skin-minimal',
  'skin-vivid',
];
const LEGACY_THEME_SKIN_MAP: Record<string, ThemeSkin> = {
  default: 'soft',
  compact: 'sharp',
  brutalist: 'sharp',
  minimal: 'sharp',
  vivid: 'hud',
  cyberDarkline: 'cyber-darkline',
  cyber_darkline: 'cyber-darkline',
  cyberdarkline: 'cyber-darkline',
};
const LEGACY_THEME_STYLE_MAP: Record<string, ThemeStyle> = {
  rose: 'violet',
  cotton: 'graphite',
  lavender: 'violet',
  peach: 'sunset',
};

function normalizeThemeStyle(value: string | null): ThemeStyle {
  if (!value) return 'default';
  if (THEME_STYLES.includes(value as ThemeStyle)) return value as ThemeStyle;
  return LEGACY_THEME_STYLE_MAP[value] ?? 'default';
}

function normalizeThemeSkin(value: string | null): ThemeSkin {
  if (!value) return 'soft';
  if (THEME_SKINS.includes(value as ThemeSkin)) return value as ThemeSkin;
  return LEGACY_THEME_SKIN_MAP[value] ?? 'soft';
}

export function ThemeProvider({ children }: { children: React.ReactNode }) {
  const [theme, setThemeState] = useState<Theme>(() => {
    // Initialize from localStorage
    const stored = localStorage.getItem(THEME_STORAGE_KEY);
    return (stored === 'light' || stored === 'dark' || stored === 'system') ? stored : 'system';
  });
  const [style, setStyleState] = useState<ThemeStyle>(() => {
    const stored = localStorage.getItem(THEME_STYLE_STORAGE_KEY);
    return normalizeThemeStyle(stored);
  });
  const [skin, setSkinState] = useState<ThemeSkin>(() => {
    const stored = localStorage.getItem(THEME_SKIN_STORAGE_KEY);
    return normalizeThemeSkin(stored);
  });

  const setTheme = (newTheme: Theme) => {
    localStorage.setItem(THEME_STORAGE_KEY, newTheme);
    setThemeState(newTheme);
  };

  const setThemeStyle = (newStyle: ThemeStyle) => {
    localStorage.setItem(THEME_STYLE_STORAGE_KEY, newStyle);
    setStyleState(newStyle);
  };

  const setThemeSkin = (newSkin: ThemeSkin) => {
    localStorage.setItem(THEME_SKIN_STORAGE_KEY, newSkin);
    setSkinState(newSkin);
  };

  useEffect(() => {
    const root = window.document.documentElement;

    const syncTauriTheme = async (theme: Theme) => {
      try {
        const [{ getAllWindows }, appModule] = await Promise.all([
          import('@tauri-apps/api/window'),
          import('@tauri-apps/api/app'),
        ]);
        const tauriTheme = theme === 'system' ? null : theme;
        await appModule.setTheme(tauriTheme);
        const windows = await getAllWindows();
        await Promise.all(windows.map((win) => win.setTheme(tauriTheme)));
      } catch (error) {
        // Ignore in non-Tauri environments (tests, web previews).
        console.warn('[theme] Failed to sync window theme', error);
      }
    };

    const applyTheme = (theme: Theme) => {
      root.classList.remove('light', 'dark');

      if (theme === 'system') {
        const systemTheme = window.matchMedia('(prefers-color-scheme: dark)')
          .matches
          ? 'dark'
          : 'light';
        root.classList.add(systemTheme);
      } else {
        root.classList.add(theme);
      }
    };

    const applyStyle = (style: ThemeStyle) => {
      root.classList.remove(...THEME_STYLE_CLASSES);
      root.classList.add(`theme-${style}`);
    };

    const applySkin = (nextSkin: ThemeSkin) => {
      root.classList.remove(...THEME_SKIN_CLASSES);
      root.classList.add(`skin-${nextSkin}`);
    };

    applyTheme(theme);
    applyStyle(style);
    applySkin(skin);
    void syncTauriTheme(theme);

    // Listen for system theme changes when theme is 'system'
    if (theme === 'system') {
      const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)');

      const handleChange = (e: MediaQueryListEvent | MediaQueryList) => {
        root.classList.remove('light', 'dark');
        root.classList.add(e.matches ? 'dark' : 'light');
        void syncTauriTheme('system');
      };

      // Modern browsers
      if (mediaQuery.addEventListener) {
        mediaQuery.addEventListener('change', handleChange);
        return () => mediaQuery.removeEventListener('change', handleChange);
      }
      // Fallback for older browsers
      else if (mediaQuery.addListener) {
        mediaQuery.addListener(handleChange);
        return () => mediaQuery.removeListener(handleChange);
      }
    }
  }, [theme, style, skin]);

  return (
    <ThemeContext.Provider value={{ theme, setTheme, style, setThemeStyle, skin, setThemeSkin }}>
      {children}
    </ThemeContext.Provider>
  );
}

export function useTheme() {
  const context = useContext(ThemeContext);
  if (context === undefined) {
    throw new Error('useTheme must be used within a ThemeProvider');
  }
  return context;
}
