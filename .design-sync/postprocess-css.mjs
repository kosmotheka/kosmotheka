// Cleans the compiled Tailwind stylesheet before the design sync, without
// changing how anything renders. Claude Design flags theme variables whose
// type it cannot infer (easings, animations, aspect ratios); each gets a
// `/* @kind other */` annotation, which Claude Design only reads on the same
// line as the declaration, after the `;`, and only in token scopes (:root in
// @layer theme). Tailwind's --tw-* internals are left alone: in @layer
// properties the annotation has no effect, and filtering them is up to the
// sync tool. Variables that Tailwind only declares inside @layer properties
// (behind @supports) but that are real tokens, like --shimmer-angle, are
// copied to a plain :root in @layer theme, with their annotation; the value is
// the same, so rendering does not change.
// It runs after the Tailwind build, which would otherwise move the comments.
// Usage: node .design-sync/postprocess-css.mjs <file.css>
import { readFileSync, writeFileSync } from 'node:fs';
import { createRequire } from 'node:module';

const require = createRequire(new URL('../.ds-sync/package.json', import.meta.url));
const postcss = require('postcss');

const file = process.argv[2];
const root = postcss.parse(readFileSync(file, 'utf8'));
const UNTYPED = /^--(ease|animate|aspect|default-transition|scroll-fade|shimmer)/;
const HOISTED = /^--shimmer-/;

const layerOf = (node) => {
  for (let p = node.parent; p; p = p.parent) {
    if (p.type === 'atrule' && p.name === 'layer') return p.params;
  }
  return null;
};

const annotate = (decl) => {
  const next = decl.next();
  if (next?.type === 'comment' && next.text === '@kind other') return false;
  // raws.before = ' ' keeps the comment on the declaration's line.
  decl.after(postcss.comment({ text: '@kind other', raws: { before: ' ', left: ' ', right: ' ' } }));
  return true;
};

// Copy real tokens that only live in @layer properties to @layer theme :root.
const themeLayer = root.nodes.find((n) => n.type === 'atrule' && n.name === 'layer' && n.params === 'theme');
const hoisted = new Map();
root.walkDecls((decl) => {
  const inRootScope = decl.parent.type === 'rule' && decl.parent.selector.includes(':root');
  if (HOISTED.test(decl.prop) && inRootScope && layerOf(decl.parent) === 'properties') hoisted.set(decl.prop, decl.value);
});
if (themeLayer && hoisted.size > 0) {
  const rule = postcss.rule({ selector: ':root, :host', raws: { between: ' ' } });
  for (const [prop, value] of hoisted) rule.append(postcss.decl({ prop, value }));
  themeLayer.append(rule);
}

let marked = 0;
root.walkDecls((decl) => {
  if (!UNTYPED.test(decl.prop) || layerOf(decl.parent) !== 'theme') return;
  if (annotate(decl)) marked++;
});

writeFileSync(file, root.toString());
console.error(`postprocess-css: ${hoisted.size} variable(s) copied to @layer theme, ${marked} marked @kind other`);
