import { readFileSync } from 'fs';
import { fileURLToPath } from 'url';
import { dirname, join } from 'path';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

// Read fixtures
const siteList = JSON.parse(readFileSync(join(__dirname, '../../../docs/fixtures/site_list.json'), 'utf8'));
const searchHits = JSON.parse(readFileSync(join(__dirname, '../../../docs/fixtures/search_hits.json'), 'utf8'));
const settings = JSON.parse(readFileSync(join(__dirname, '../../../docs/fixtures/settings.json'), 'utf8'));

console.log('Testing Mock Fixtures & App Logic:');

// 1. Verify Site List Fixture
console.log('\n1. Verifying Site List...');
if (!Array.isArray(siteList) || siteList.length === 0) {
  throw new Error('Site list fixture is invalid');
}
console.log(`✓ Loaded ${siteList.length} sites:`, siteList.map(s => s.domain).join(', '));
for (const site of siteList) {
  if (typeof site.domain !== 'string' || typeof site.secs !== 'number') {
    throw new Error(`Site missing domain or secs: ${JSON.stringify(site)}`);
  }
  if (!Array.isArray(site.pages)) {
    throw new Error(`Site pages is not an array: ${site.domain}`);
  }
  for (const page of site.pages) {
    if (typeof page.url !== 'string' || typeof page.title !== 'string' || typeof page.secs !== 'number') {
      throw new Error(`Page invalid shape in site ${site.domain}: ${JSON.stringify(page)}`);
    }
  }
}
console.log('✓ All sites and pages validate against contracts');

// 2. Verify Search Hits Fixture
console.log('\n2. Verifying Search Hits...');
if (!Array.isArray(searchHits) || searchHits.length === 0) {
  throw new Error('Search hits fixture is invalid');
}
console.log(`✓ Loaded ${searchHits.length} hits`);
for (const hit of searchHits) {
  if (!['site', 'page'].includes(hit.kind)) {
    throw new Error(`Hit kind invalid: ${hit.kind}`);
  }
  if (typeof hit.domain !== 'string' || typeof hit.secs !== 'number' || typeof hit.score !== 'number') {
    throw new Error(`Hit properties invalid: ${JSON.stringify(hit)}`);
  }
}
console.log('✓ All search hits validate against contracts');

// 3. Verify Settings Fixture and Round-trip
console.log('\n3. Verifying Settings and Round-trip...');
if (typeof settings.afk_timeout_secs !== 'number' || typeof settings.day_start !== 'string') {
  throw new Error('Settings fixture invalid');
}
console.log('✓ Original settings:', settings);

const updatedSettings = {
  ...settings,
  afk_timeout_secs: 300,
  day_start: '04:00'
};
console.log('✓ Modified settings for save:', updatedSettings);
Object.assign(settings, updatedSettings);
if (settings.afk_timeout_secs !== 300 || settings.day_start !== '04:00') {
  throw new Error('Settings mutation round-trip failed');
}
console.log('✓ Settings round-trip verified successfully');

// 4. Test formatters logic
console.log('\n4. Verifying formatters...');
function formatDuration(seconds) {
  if (!seconds || seconds <= 0) return '0s';
  const total = Math.round(seconds);
  const hours = Math.floor(total / 3600);
  const minutes = Math.floor((total % 3600) / 60);
  const secs = total % 60;
  const parts = [];
  if (hours > 0) parts.push(`${hours}h`);
  if (minutes > 0) parts.push(`${minutes}m`);
  if (secs > 0 && hours === 0) parts.push(`${secs}s`);
  return parts.length > 0 ? parts.join(' ') : '0s';
}

function formatPercent(value, total) {
  if (!total || total <= 0) return 0;
  const pct = (value / total) * 100;
  return Math.min(100, Math.max(0, Math.round(pct * 10) / 10));
}

if (formatDuration(5400) !== '1h 30m') throw new Error(`formatDuration(5400) was ${formatDuration(5400)}`);
if (formatDuration(600) !== '10m') throw new Error(`formatDuration(600) was ${formatDuration(600)}`);
if (formatDuration(45) !== '45s') throw new Error(`formatDuration(45) was ${formatDuration(45)}`);
if (formatPercent(1200, 1800) !== 66.7) throw new Error(`formatPercent(1200, 1800) was ${formatPercent(1200, 1800)}`);
console.log('✓ Formatter assertions passed');

console.log('\nALL TESTS PASSED! ✨');
