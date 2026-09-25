// Cleans the compiled Tailwind stylesheet before the design sync, without
// changing how anything renders. Claude Design flags theme variables whose
// type it cannot infer (easings, animations, aspect ratios, Tailwind's own
// --tw-* defaults); each gets a `/* @kind other */` annotation, which Claude
// Design only reads on the same line as the declaration, after the `;`.
// It runs after the Tailwind build, which would otherwise move the comments.
// Usage: node .design-sync/postprocess-css.mjs <file.css>
import { readFileSync, writeFileSync } from 'node:fs';
import { createRequire } from 'node:module';

const require = createRequire(new URL('../.ds-sync/package.json', import.meta.url));
const postcss = require('postcss');

const file = process.argv[2];
const root = postcss.parse(readFileSync(file, 'utf8'));
const UNTYPED = /^--(tw-|ease|animate|aspect|default-transition|scroll-fade|shimmer)/;

const layerOf = (node) => {
  for (let p = node.parent; p; p = p.parent) {
    if (p.type === 'atrule' && p.name === 'layer') return p.params;
  }
  return null;
};

let marked = 0;
root.walkDecls((decl) => {
  if (!decl.prop.startsWith('--') || !UNTYPED.test(decl.prop)) return;
  const layer = layerOf(decl.parent);
  if (layer !== 'theme' && layer !== 'properties') return;
  const next = decl.next();
  if (next?.type === 'comment' && next.text === '@kind other') return;
  // raws.before = ' ' keeps the comment on the declaration's line.
  decl.after(postcss.comment({ text: '@kind other', raws: { before: ' ', left: ' ', right: ' ' } }));
  marked++;
});

writeFileSync(file, root.toString());
console.error(`postprocess-css: ${marked} variable(s) marked @kind other`);
