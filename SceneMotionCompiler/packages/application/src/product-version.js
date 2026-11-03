import { readFileSync } from 'node:fs';

const packageJson = JSON.parse(readFileSync(new URL('../../../package.json', import.meta.url), 'utf8'));
if (typeof packageJson.version !== 'string' || !packageJson.version) throw new Error('package.json version is missing');

export const PRODUCT_VERSION = packageJson.version;
