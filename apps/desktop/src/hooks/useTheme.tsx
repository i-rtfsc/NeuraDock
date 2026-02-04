import { createContext, useContext, useEffect, useState } from 'react';

type Theme = 'light' | 'dark' | 'system';
type ThemeStyle = 'default' | 'graphite' | 'emerald' | 'sunset' | 'midnight' | 'rose' | 'ocean' | 'cotton' | 'lavender' | 'peach';

interface ThemeContextType {
  theme: Theme;
  setTheme: (theme: Theme) => void;
  style: ThemeStyle;
  setThemeStyle: (style: ThemeStyle) => void;
}

const ThemeContext = createContext<ThemeContextType | undefined>(undefined);

const THEME_STORAGE_KEY = 'neuradock-theme';
const THEME_STYLE_STORAGE_KEY = 'neuradock-theme-style';
const THEME_STYLE_CLASSES = [
  'theme-default',
  'theme-graphite',
  'theme-emerald',
  'theme-sunset',
  'theme-midnight',
  'theme-rose',
  'theme-ocean',
  'theme-cotton',
  'theme-lavender',
  'theme-peach',
];

export function ThemeProvider({ children }: { children: React.ReactNode }) {
  const [theme, setThemeState] = useState<Theme>(() => {
    // Initialize from localStorage
    const stored = localStorage.getItem(THEME_STORAGE_KEY);
    return (stored === 'light' || stored === 'dark' || stored === 'system') ? stored : 'system';
  });
  const [style, setStyleState] = useState<ThemeStyle>(() => {
    const stored = localStorage.getItem(THEME_STYLE_STORAGE_KEY);
    return (stored === 'default' || stored === 'graphite' || stored === 'emerald' || stored === 'sunset' || stored === 'midnight' || stored === 'rose' || stored === 'ocean' || stored === 'cotton' || stored === 'lavender' || stored === 'peach')
      ? stored
      : 'default';
  });

  const setTheme = (newTheme: Theme) => {
    localStorage.setItem(THEME_STORAGE_KEY, newTheme);
    setThemeState(newTheme);
  };

  const setThemeStyle = (newStyle: ThemeStyle) => {
    localStorage.setItem(THEME_STYLE_STORAGE_KEY, newStyle);
    setStyleState(newStyle);
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

    applyTheme(theme);
    applyStyle(style);
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
  }, [theme, style]);

  return (
    <ThemeContext.Provider value={{ theme, setTheme, style, setThemeStyle }}>
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
