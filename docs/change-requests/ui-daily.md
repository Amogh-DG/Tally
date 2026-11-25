# Change Request: ui-daily

## CR-01: Mock Mode Behavior for `get_day(date)` in `ui/src/app/api.ts`

### Status
Submitted / Workaround Applied in `ui/src/views/daily/`

### Problem
In `ui/src/app/api.ts`, lines 86-90:
```typescript
export async function get_day(date: string): Promise<DayReport> {
  if (isMock()) {
    return { ...mockDayReport, date } as DayReport;
  }
  return await invoke<DayReport>('get_day', { date });
}
```
In mock mode (`VITE_MOCK=1`), `get_day` returns `mockDayReport` (which has 28800s active time and 3 timeline blocks from 2026-11-23) for *every* requested date. This makes it impossible for the UI to test or demonstrate empty day states when navigating to dates outside the fixture dataset (e.g., navigating to previous or next days with no recorded activity).

### Proposed Change
In `ui/src/app/api.ts`:
```typescript
export async function get_day(date: string): Promise<DayReport> {
  if (isMock()) {
    if (date === mockDayReport.date) {
      return { ...mockDayReport } as DayReport;
    }
    // Return empty day report for dates without fixture activity
    return {
      date,
      active_secs: 0,
      timeline: [],
      top_apps: [],
      top_sites: []
    } as DayReport;
  }
  return await invoke<DayReport>('get_day', { date });
}
```

### Workaround in `ui/src/views/daily/`
Since `api.ts` is frozen for Phase 1, `ui/src/views/daily/` wraps the `get_day` call: when in mock mode (`isMock()`), if the selected date is not `'2026-11-23'`, it synthesizes an empty `DayReport` (`active_secs: 0`, `timeline: []`, `top_apps: []`, `top_sites: []`). This enables clean demonstration of empty day states while preserving 100% fidelity with `mockDayReport` on `2026-11-23`.
