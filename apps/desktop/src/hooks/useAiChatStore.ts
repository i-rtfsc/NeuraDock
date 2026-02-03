import { create } from 'zustand';
import { persist } from 'zustand/middleware';

export interface AiChatTab {
  serviceId: string;
  name: string;
  url: string;
  icon?: string;
}

interface AiChatStore {
  // Open tabs state
  openTabs: AiChatTab[];
  activeTabId: string | null;
  
  // Settings
  maxCachedWebviews: number; // Max number of webviews to keep in memory (1-6)

  // Actions
  openTab: (tab: AiChatTab) => void;
  closeTab: (serviceId: string) => void;
  setActiveTab: (serviceId: string | null) => void;
  isTabOpen: (serviceId: string) => boolean;
  setMaxCachedWebviews: (count: number) => void;
}

export const useAiChatStore = create<AiChatStore>()(
  persist(
    (set, get) => ({
      openTabs: [],
      activeTabId: null,
      maxCachedWebviews: 3, // Default to 3 cached webviews

      openTab: (tab) => {
        const { openTabs, isTabOpen } = get();
        if (!isTabOpen(tab.serviceId)) {
          set({
            openTabs: [...openTabs, tab],
            activeTabId: tab.serviceId,
          });
        } else {
          // Tab already open, just set it as active
          set({ activeTabId: tab.serviceId });
        }
      },

      closeTab: (serviceId) => {
        const { openTabs, activeTabId } = get();
        const newTabs = openTabs.filter((tab) => tab.serviceId !== serviceId);
        let newActiveId = activeTabId;

        // If we're closing the active tab, switch to another
        if (activeTabId === serviceId) {
          newActiveId = newTabs.length > 0 ? newTabs[newTabs.length - 1].serviceId : null;
        }

        set({
          openTabs: newTabs,
          activeTabId: newActiveId,
        });
      },

      setActiveTab: (serviceId) => {
        set({ activeTabId: serviceId });
      },

      isTabOpen: (serviceId) => {
        return get().openTabs.some((tab) => tab.serviceId === serviceId);
      },
      
      setMaxCachedWebviews: (count) => {
        // Clamp between 1 and 6
        const clamped = Math.max(1, Math.min(6, count));
        set({ maxCachedWebviews: clamped });
      },
    }),
    {
      name: 'ai-chat-storage',
      // Persist tab state and settings
      partialize: (state) => ({
        openTabs: state.openTabs,
        activeTabId: state.activeTabId,
        maxCachedWebviews: state.maxCachedWebviews,
      }),
    }
  )
);
