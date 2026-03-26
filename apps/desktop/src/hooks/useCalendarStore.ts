import { create } from 'zustand';
import { persist } from 'zustand/middleware';

interface CalendarSettings {
  /** tianapi.com API key for holiday data */
  tianapiKey: string;
  setTianapiKey: (key: string) => void;
  /** Weather city name for wttr.in */
  weatherCity: string;
  setWeatherCity: (city: string) => void;
}

export const useCalendarStore = create<CalendarSettings>()(
  persist(
    (set) => ({
      tianapiKey: '',
      setTianapiKey: (tianapiKey) => set({ tianapiKey }),
      weatherCity: '',
      setWeatherCity: (weatherCity) => set({ weatherCity }),
    }),
    {
      name: 'calendar-settings',
    }
  )
);
