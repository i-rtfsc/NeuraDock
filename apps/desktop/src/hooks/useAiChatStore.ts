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

  // Actions
  openTab: (tab: AiChatTab) => void;
  closeTab: (serviceId: string) => void;
  setActiveTab: (serviceId: string | null) => void;
  isTabOpen: (serviceId: string) => boolean;
}

export const useAiChatStore = create<AiChatStore>()(
  persist(
    (set, get) => ({
      openTabs: [],
      activeTabId: null,

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
    }),
    {
      name: 'ai-chat-storage',
      // Only persist tab state, not functions or transient state
      partialize: (state) => ({
        openTabs: state.openTabs,
        activeTabId: state.activeTabId,
      }),
    }
  )
);
