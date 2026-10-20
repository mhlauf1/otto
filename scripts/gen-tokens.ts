import { writeFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { renderTokensCss } from '../src/design/renderTokensCss';

const out = fileURLToPath(new URL('../src/design/tokens.css', import.meta.url));
writeFileSync(out, renderTokensCss());
console.log(`wrote ${out}`);
