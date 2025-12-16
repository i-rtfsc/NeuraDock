/** @type {import('tailwindcss').Config} */
export default {
  darkMode: ['class'],
  content: [
    './index.html',
    './src/**/*.{js,ts,jsx,tsx}',
  ],
  theme: {
    extend: {
      colors: {
        border: 'hsl(var(--border))',
        input: 'hsl(var(--input))',
        ring: 'hsl(var(--ring))',
        background: 'hsl(var(--background))',
        foreground: 'hsl(var(--foreground))',
        primary: {
          DEFAULT: 'hsl(var(--primary))',
          foreground: 'hsl(var(--primary-foreground))',
        },
        secondary: {
          DEFAULT: 'hsl(var(--secondary))',
          foreground: 'hsl(var(--secondary-foreground))',
        },
        destructive: {
          DEFAULT: 'hsl(var(--destructive))',
          foreground: 'hsl(var(--destructive-foreground))',
        },
        muted: {
          DEFAULT: 'hsl(var(--muted))',
          foreground: 'hsl(var(--muted-foreground))',
        },
        accent: {
          DEFAULT: 'hsl(var(--accent))',
          foreground: 'hsl(var(--accent-foreground))',
        },
        popover: {
          DEFAULT: 'hsl(var(--popover))',
          foreground: 'hsl(var(--popover-foreground))',
        },
        card: {
          DEFAULT: 'hsl(var(--card))',
          foreground: 'hsl(var(--card-foreground))',
        },
        sidebar: {
          DEFAULT: 'hsl(var(--sidebar-background))',
          foreground: 'hsl(var(--sidebar-foreground))',
          border: 'hsl(var(--sidebar-border))',
        },
      },
      borderRadius: {
        lg: 'var(--radius-lg)',
        md: 'var(--radius-md)',
        sm: 'var(--radius-sm)',
        full: 'var(--radius-full)',
        DEFAULT: 'var(--radius-md)',
      },
      /* Layout Spacing - 引用design tokens */
      spacing: {
        'page-header-h': 'var(--layout-page-header-height)',
        'page-header-h-compact': 'var(--layout-page-header-height-compact)',
        'page-content-p': 'var(--layout-page-content-padding)',
        'sidebar': 'var(--layout-sidebar-width)',
        'sidebar-collapsed': 'var(--layout-sidebar-width-collapsed)',
        'sidebar-page': 'var(--layout-sidebar-page-width)',
        'section-gap': 'var(--spacing-section-gap)',
        'section-gap-sm': 'var(--spacing-section-gap-sm)',
        'card-gap': 'var(--spacing-card-gap)',
        'card-gap-sm': 'var(--spacing-card-gap-sm)',
        'element-gap': 'var(--spacing-element-gap)',
        'element-gap-sm': 'var(--spacing-element-gap-sm)',
      },
      /* Max Width - 引用content max width tokens */
      maxWidth: {
        'content-sm': 'var(--layout-page-content-max-width-sm)',
        'content-md': 'var(--layout-page-content-max-width-md)',
        'content-lg': 'var(--layout-page-content-max-width-lg)',
      },
      /* Padding - 引用component padding tokens */
      padding: {
        'card': 'var(--component-card-padding)',
        'card-sm': 'var(--component-card-padding-sm)',
        'card-lg': 'var(--component-card-padding-lg)',
        'stats-card': 'var(--component-stats-card-padding)',
      },
      /* Height - UI控件高度 */
      height: {
        'btn-sm': 'var(--control-button-height-sm)',
        'btn': 'var(--control-button-height-default)',
        'btn-lg': 'var(--control-button-height-lg)',
        'btn-icon-sm': 'var(--control-button-icon-sm)',
        'btn-icon': 'var(--control-button-icon-default)',
        'btn-icon-lg': 'var(--control-button-icon-lg)',
        'input-sm': 'var(--control-input-height-sm)',
        'input': 'var(--control-input-height-default)',
        'input-lg': 'var(--control-input-height-lg)',
      },
      /* Width - UI控件宽度 */
      width: {
        'btn-icon-sm': 'var(--control-button-icon-sm)',
        'btn-icon': 'var(--control-button-icon-default)',
        'btn-icon-lg': 'var(--control-button-icon-lg)',
      },
      boxShadow: {
        'xs': 'var(--shadow-xs)',
        'sm': 'var(--shadow-sm)',
        'md': 'var(--shadow-md)',
        'lg': 'var(--shadow-lg)',
        'xl': 'var(--shadow-xl)',
        '2xl': 'var(--shadow-2xl)',
        'hover-sm': 'var(--shadow-hover-sm)',
        'hover-md': 'var(--shadow-hover-md)',
        'hover-lg': 'var(--shadow-hover-lg)',
        // 保留旧的 macos 样式以向后兼容
        'macos': 'var(--shadow-sm)',
        'macos-dark': 'var(--shadow-sm)',
        'macos-hover': 'var(--shadow-hover-md)',
        'macos-active': 'var(--shadow-xs)',
      },
      transitionDuration: {
        'instant': 'var(--duration-instant)',
        'fast': 'var(--duration-fast)',
        'base': 'var(--duration-base)',
        'slow': 'var(--duration-slow)',
        'slower': 'var(--duration-slower)',
      },
      transitionTimingFunction: {
        'smooth': 'var(--ease-smooth)',
        'bounce': 'var(--ease-bounce)',
      },
      animation: {
        'fade-in': 'fadeIn var(--duration-base) var(--ease-smooth)',
        'fade-out': 'fadeOut var(--duration-base) var(--ease-smooth)',
        'slide-up': 'slideUp var(--duration-slow) var(--ease-smooth)',
        'slide-down': 'slideDown var(--duration-slow) var(--ease-smooth)',
        'slide-left': 'slideLeft var(--duration-slow) var(--ease-smooth)',
        'slide-right': 'slideRight var(--duration-slow) var(--ease-smooth)',
        'scale-in': 'scaleIn var(--duration-base) var(--ease-smooth)',
        'scale-out': 'scaleOut var(--duration-base) var(--ease-smooth)',
        'bounce-in': 'bounceIn var(--duration-slow) var(--ease-bounce)',
      },
      keyframes: {
        fadeIn: {
          '0%': { opacity: '0' },
          '100%': { opacity: '1' },
        },
        fadeOut: {
          '0%': { opacity: '1' },
          '100%': { opacity: '0' },
        },
        slideUp: {
          '0%': { transform: 'translateY(10px)', opacity: '0' },
          '100%': { transform: 'translateY(0)', opacity: '1' },
        },
        slideDown: {
          '0%': { transform: 'translateY(-10px)', opacity: '0' },
          '100%': { transform: 'translateY(0)', opacity: '1' },
        },
        slideLeft: {
          '0%': { transform: 'translateX(10px)', opacity: '0' },
          '100%': { transform: 'translateX(0)', opacity: '1' },
        },
        slideRight: {
          '0%': { transform: 'translateX(-10px)', opacity: '0' },
          '100%': { transform: 'translateX(0)', opacity: '1' },
        },
        scaleIn: {
          '0%': { transform: 'scale(0.95)', opacity: '0' },
          '100%': { transform: 'scale(1)', opacity: '1' },
        },
        scaleOut: {
          '0%': { transform: 'scale(1)', opacity: '1' },
          '100%': { transform: 'scale(0.95)', opacity: '0' },
        },
        bounceIn: {
          '0%': { transform: 'scale(0.9)', opacity: '0' },
          '50%': { transform: 'scale(1.05)' },
          '100%': { transform: 'scale(1)', opacity: '1' },
        },
      },
      backdropBlur: {
        xs: '2px',
      },
    },
  },
  plugins: [
    require('tailwindcss-animate'),
  ],
}
