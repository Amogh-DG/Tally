import { invoke } from '@tauri-apps/api/core';

import mockDayReport from '../../../docs/fixtures/day_report.json';
import mockRangeHours from '../../../docs/fixtures/range_hours.json';
import mockSiteList from '../../../docs/fixtures/site_list.json';
import mockSearchHits from '../../../docs/fixtures/search_hits.json';
import mockSettings from '../../../docs/fixtures/settings.json';

export interface TimelineBlock {
  start: string;
  end: string;
  kind: 'app' | 'site';
  label: string;
  app: string;
}

export interface TopApp {
  app: string;
  secs: number;
}

export interface TopSite {
  domain: string;
  secs: number;
}

export interface DayReport {
  date: string;
  active_secs: number;
  timeline: TimelineBlock[];
  top_apps: TopApp[];
  top_sites: TopSite[];
}

export interface RangeHoursItem {
  date: string;
  active_secs: number;
}

export type RangeHours = RangeHoursItem[];

export interface PageItem {
  url: string;
  title: string;
  secs: number;
}

export interface SiteItem {
  domain: string;
  secs: number;
  pages: PageItem[];
}

export type SiteList = SiteItem[];

export interface Hit {
  kind: 'site' | 'page';
  domain: string;
  url?: string | null;
  title?: string | null;
  score: number;
  secs: number;
}

export interface Settings {
  afk_timeout_secs: number;
  day_start: string;
  port: number;
  extension_origins: string[];
  data_dir: string;
}

export interface ServerStatus {
  reachable: boolean;
}

const isMock = (): boolean => {
  return (
    import.meta.env.VITE_MOCK === '1' ||
    import.meta.env.VITE_MOCK === 'true' ||
    typeof window === 'undefined' ||
    !(window as any).__TAURI_INTERNALS__
  );
};

export async function get_day(date: string): Promise<DayReport> {
  if (isMock()) {
    return { ...mockDayReport, date } as DayReport;
  }
  return await invoke<DayReport>('get_day', { date });
}

export async function get_range_hours(from: string, to: string): Promise<RangeHours> {
  if (isMock()) {
    return mockRangeHours as RangeHours;
  }
  return await invoke<RangeHours>('get_range_hours', { from, to });
}

export async function get_sites(from: string, to: string): Promise<SiteList> {
  if (isMock()) {
    return mockSiteList as SiteList;
  }
  return await invoke<SiteList>('get_sites', { from, to });
}

export async function search(
  query: string,
  mode: string,
  site?: string | null,
  from?: string | null,
  to?: string | null
): Promise<Hit[]> {
  if (isMock()) {
    return mockSearchHits as Hit[];
  }
  return await invoke<Hit[]>('search', { query, mode, site, from, to });
}

export async function export_json(from: string, to: string, path: string): Promise<string> {
  if (isMock()) {
    return path;
  }
  return await invoke<string>('export_json', { from, to, path });
}

export async function get_settings(): Promise<Settings> {
  if (isMock()) {
    return mockSettings as Settings;
  }
  return await invoke<Settings>('get_settings');
}

export async function set_settings(settings: Settings): Promise<Settings> {
  if (isMock()) {
    return settings;
  }
  return await invoke<Settings>('set_settings', { settings });
}

export async function server_status(): Promise<ServerStatus> {
  if (isMock()) {
    return { reachable: true };
  }
  return await invoke<ServerStatus>('server_status');
}

// CamelCase convenience aliases
export const getDay = get_day;
export const getRangeHours = get_range_hours;
export const getSites = get_sites;
export const exportJson = export_json;
export const getSettings = get_settings;
export const setSettings = set_settings;
export const serverStatus = server_status;
