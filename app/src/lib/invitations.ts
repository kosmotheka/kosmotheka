import type { TFunction } from "i18next";

import type { InvitationItem } from "@/lib/api";

/** «today at 18:20», «tomorrow at 18:20» or the date: when an invitation stops being worth anything. */
export function expiryText(
  iso: string,
  t: TFunction,
  locale: string | undefined,
  now = new Date(),
) {
  const at = new Date(iso);
  const time = at.toLocaleTimeString(locale, { hour: "2-digit", minute: "2-digit" });
  const day = (date: Date) =>
    new Date(date.getFullYear(), date.getMonth(), date.getDate()).getTime();
  const days = Math.round((day(at) - day(now)) / 86_400_000);
  if (days === 0) return t("invitations.expiry.today", { time });
  if (days === 1) return t("invitations.expiry.tomorrow", { time });
  return t("invitations.expiry.on", {
    date: at.toLocaleDateString(locale, { day: "numeric", month: "short" }),
    time,
  });
}

/**
 * What to say about a person's invitations: one still worth something, else the last accepted. The server
 * lists them all; a revoked or expired one says nothing.
 */
export function invitationOf(
  items: InvitationItem[],
  memberId: string,
  now = Date.now(),
): InvitationItem | null {
  const mine = items.filter((item) => item.memberId === memberId);
  const pending = mine.find((item) => item.state === "pending" && Date.parse(item.expiresAt) > now);
  if (pending) return pending;
  const accepted = mine
    .filter((item) => item.state === "accepted")
    .sort((a, b) => Date.parse(b.acceptedAt ?? "") - Date.parse(a.acceptedAt ?? ""));
  return accepted[0] ?? null;
}
