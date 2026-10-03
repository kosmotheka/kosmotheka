import type { LucideIcon } from "lucide-react";
import {
  Archive,
  Bookmark,
  CircleHelp,
  Inbox,
  Info,
  LibraryBig,
  ListOrdered,
  ScanBarcode,
  Settings,
  Share2,
  SlidersHorizontal,
  Users,
} from "lucide-react";

import { type Overlay, type Section, navigate, openOverlay } from "@/lib/navigation";

export interface NavItem {
  id: string;
  icon: LucideIcon;
  /** i18n key. */
  label: string;
  section?: Section;
  overlay?: Overlay;
  /** Planned in the roadmap but not built yet: shown, but disabled. */
  soon?: boolean;
}

/** The three main destinations: bottom bar on mobile, top of the sidebar on desktop. */
export const mainNav: [NavItem, NavItem, NavItem] = [
  { id: "catalog", icon: LibraryBig, label: "nav.catalog", section: "catalog" },
  { id: "check", icon: ScanBarcode, label: "nav.check", section: "check" },
  { id: "missing", icon: Bookmark, label: "nav.missing", section: "missing" },
];

/** What belongs to the current library. */
export const libraryNav: NavItem[] = [
  { id: "requests", icon: Inbox, label: "nav.requests", section: "requests" },
  { id: "people", icon: Users, label: "nav.people", section: "people" },
  { id: "series", icon: ListOrdered, label: "nav.series", section: "series" },
  { id: "backups", icon: Archive, label: "nav.backups", overlay: { name: "backups" } },
  { id: "library", icon: SlidersHorizontal, label: "nav.librarySettings", section: "library" },
];

/** App-wide entries, at the end of the menu. Sharing is per device, for every library. */
export const appNav: NavItem[] = [
  { id: "devices", icon: Share2, label: "nav.share", section: "devices" },
  { id: "settings", icon: Settings, label: "nav.settings", overlay: { name: "settings" } },
  { id: "help", icon: CircleHelp, label: "nav.help", section: "help" },
  { id: "about", icon: Info, label: "nav.about", soon: true },
];

export function go(item: NavItem) {
  if (item.soon) return;
  if (item.overlay) openOverlay(item.overlay);
  else if (item.section) navigate(item.section);
}
