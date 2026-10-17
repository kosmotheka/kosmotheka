// Fails when a page of the built site links to a page or file of the site that does not exist.
// A relative link in Markdown is resolved from the page's own address (`/es/using-the-app/page/`),
// so `./other/` points one level too deep: this is what catches it. Run after `npm run build`.
import { existsSync, readFileSync, readdirSync, statSync } from "node:fs";
import { dirname, join, normalize } from "node:path";

const root = "dist";

function pages(dir) {
  return readdirSync(dir).flatMap((name) => {
    const path = join(dir, name);
    if (statSync(path).isDirectory()) return pages(path);
    return name.endsWith(".html") ? [path] : [];
  });
}

const broken = [];
for (const page of pages(root)) {
  const html = readFileSync(page, "utf8");
  for (const [, href] of html.matchAll(/href="([^"#?]+)/g)) {
    if (!href.startsWith(".") && !href.startsWith("/")) continue;
    if (href.startsWith("//")) continue;
    const target = decodeURIComponent(
      normalize(href.startsWith("/") ? join(root, href) : join(dirname(page), href)),
    );
    if (!existsSync(target) && !existsSync(join(target, "index.html"))) {
      broken.push(`${page.slice(root.length)} → ${href}`);
    }
  }
}

if (broken.length > 0) {
  console.error(`Broken links (${broken.length}):\n${[...new Set(broken)].join("\n")}`);
  process.exit(1);
}
console.log("All internal links resolve.");
