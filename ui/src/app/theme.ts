import { writable } from 'svelte/store';

function createThemeStore() {
  const getInitial = (): boolean => {
    if (typeof window !== 'undefined' && window.matchMedia) {
      return window.matchMedia('(prefers-color-scheme: dark)').matches;
    }
    return false;
  };

  const { subscribe, set } = writable<boolean>(getInitial());

  if (typeof window !== 'undefined' && window.matchMedia) {
    const mq = window.matchMedia('(prefers-color-scheme: dark)');
    const listener = (event: MediaQueryListEvent) => {
      set(event.matches);
    };

    if (mq.addEventListener) {
      mq.addEventListener('change', listener);
    } else if ((mq as any).addListener) {
      (mq as any).addListener(listener);
    }
  }

  return {
    subscribe,
    set
  };
}

export const isDarkMode = createThemeStore();
