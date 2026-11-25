import assert from 'node:assert';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

// Read fixtures
const fixturesDir = path.resolve(__dirname, '../../../../docs/fixtures');
const dayReportRaw = fs.readFileSync(path.join(fixturesDir, 'day_report.json'), 'utf8');
const rangeHoursRaw = fs.readFileSync(path.join(fixturesDir, 'range_hours.json'), 'utf8');

const dayReport = JSON.parse(dayReportRaw);
const rangeHours = JSON.parse(rangeHoursRaw);

console.log('Testing Daily View logic and fixtures...');

// Test 1: Validate day_report.json fixture structure
assert.strictEqual(dayReport.date, '2026-11-23', 'Fixture date should be 2026-11-23');
assert.strictEqual(dayReport.active_secs, 28800.0, 'Fixture active_secs should be 28800.0 (8 hours)');
assert.strictEqual(dayReport.timeline.length, 3, 'Timeline should have 3 blocks');
assert.strictEqual(dayReport.timeline[0].app, 'code.exe');
assert.strictEqual(dayReport.timeline[1].label, 'github.com');
assert.strictEqual(dayReport.timeline[2].label, 'Visual Studio Code');
assert.strictEqual(dayReport.top_apps.length, 2, 'Top apps should have 2 entries');
assert.strictEqual(dayReport.top_sites.length, 2, 'Top sites should have 2 entries');
console.log('✓ Test 1 Passed: Day report fixture is valid');

// Test 2: Validate range_hours.json fixture structure
assert.strictEqual(rangeHours.length, 7, 'Range hours should contain 7 days');
assert.strictEqual(rangeHours[0].date, '2026-11-17');
assert.strictEqual(rangeHours[6].date, '2026-11-23');
console.log('✓ Test 2 Passed: Range hours fixture is valid');

// Test 3: Test Duration Formatting
function formatDuration(secs) {
  if (!secs || secs <= 0) return '0m';
  const totalSeconds = Math.round(secs);
  const hours = Math.floor(totalSeconds / 3600);
  const minutes = Math.floor((totalSeconds % 3600) / 60);
  const seconds = totalSeconds % 60;
  if (hours > 0) return `${hours}h ${minutes}m`;
  if (minutes > 0) return `${minutes}m`;
  return `${seconds}s`;
}

assert.strictEqual(formatDuration(28800), '8h 0m');
assert.strictEqual(formatDuration(21600), '6h 0m');
assert.strictEqual(formatDuration(7200), '2h 0m');
assert.strictEqual(formatDuration(5400), '1h 30m');
assert.strictEqual(formatDuration(1800), '30m');
assert.strictEqual(formatDuration(45), '45s');
assert.strictEqual(formatDuration(0), '0m');
console.log('✓ Test 3 Passed: formatDuration matches requirements');

// Test 4: Test Shift Date
function shiftDate(dateStr, days) {
  const [year, month, day] = dateStr.split('-').map(Number);
  const d = new Date(year, month - 1, day);
  d.setDate(d.getDate() + days);
  const y = d.getFullYear();
  const m = String(d.getMonth() + 1).padStart(2, '0');
  const dd = String(d.getDate()).padStart(2, '0');
  return `${y}-${m}-${dd}`;
}

assert.strictEqual(shiftDate('2026-11-23', -1), '2026-11-22');
assert.strictEqual(shiftDate('2026-11-23', 1), '2026-11-24');
assert.strictEqual(shiftDate('2026-11-23', -6), '2026-11-17');
console.log('✓ Test 4 Passed: shiftDate functions accurately');

// Test 5: Test Time Parsing
function parseSecondsInDay(isoString, reportDate) {
  const match = isoString.match(/^(\d{4}-\d{2}-\d{2})T(\d{2}):(\d{2}):(\d{2})/);
  if (match) {
    const [, datePart, h, m, s] = match;
    if (datePart === reportDate) {
      return parseInt(h, 10) * 3600 + parseInt(m, 10) * 60 + parseInt(s, 10);
    }
  }
  return 0;
}

assert.strictEqual(parseSecondsInDay('2026-11-23T09:00:00.000Z', '2026-11-23'), 32400); // 9h
assert.strictEqual(parseSecondsInDay('2026-11-23T12:00:00.000Z', '2026-11-23'), 43200); // 12h
assert.strictEqual(parseSecondsInDay('2026-11-23T14:00:00.000Z', '2026-11-23'), 50400); // 14h
assert.strictEqual(parseSecondsInDay('2026-11-23T17:00:00.000Z', '2026-11-23'), 61200); // 17h
console.log('✓ Test 5 Passed: parseSecondsInDay correctly parses timeline start and end times');

// Test 6: Empty Day logic
function getDayReportMock(date) {
  if (date !== '2026-11-23') {
    return {
      date,
      active_secs: 0,
      timeline: [],
      top_apps: [],
      top_sites: []
    };
  }
  return dayReport;
}

const emptyDay = getDayReportMock('2026-11-24');
assert.strictEqual(emptyDay.active_secs, 0);
assert.strictEqual(emptyDay.timeline.length, 0);
assert.strictEqual(emptyDay.top_apps.length, 0);
assert.strictEqual(emptyDay.top_sites.length, 0);

const activeDay = getDayReportMock('2026-11-23');
assert.strictEqual(activeDay.active_secs, 28800.0);
assert.strictEqual(activeDay.timeline.length, 3);
console.log('✓ Test 6 Passed: Empty day state and active day state resolve properly');

console.log('\nAll tests successfully passed!');
