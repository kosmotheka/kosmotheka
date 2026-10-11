import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Loader2, MailOpen } from "lucide-react";

import { useLibraries } from "@/components/libraries-context";
import { SheetOrDialog } from "@/components/shell/sheet-or-dialog";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  type AcceptedInvitation,
  type OpenedInvitation,
  acceptInvitation,
  discardPendingInvitation,
  invitationErrorKey,
  invitationReasonKey,
  openInvitation,
  pendingInvitation,
} from "@/lib/api";
import { useAccount } from "@/lib/account";
import { cloudNotices, refreshCloudStatus } from "@/lib/cloud";
import { expiryText } from "@/lib/invitations";
import { closeOverlay, openOverlay } from "@/lib/navigation";
import { cn } from "@/lib/utils";

/**
 * «I have an invitation» (D43): paste the link, see only the name of the library and when it stops being
 * valid, and enter with the signed-in account. Whoever invites and who is inside are not shown: the app does
 * not know them. Without a session the link waits (in Rust) while the person signs in, and `App` brings this
 * dialog back. Every link, wherever it comes from, enters through `openInvitation`.
 */
export function InvitationDialog({
  open,
  errorKey,
  desktop,
  onDone,
}: {
  open: boolean;
  /** The link that brought the person here was wrong: what it says, from the locales. */
  errorKey?: string;
  desktop: boolean;
  onDone: (message: string) => void;
}) {
  const { t, i18n } = useTranslation();
  const account = useAccount();
  const libraries = useLibraries();
  const [opened, setOpened] = useState<OpenedInvitation | null>(null);
  // The library of a link that expired while the person signed in: said here, once.
  const [expiredFor, setExpiredFor] = useState<string | null>(null);
  const [link, setLink] = useState("");
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState<"open" | "accept" | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [joinedLater, setJoinedLater] = useState(false);
  // Why the library did not come down with the invitation, if the session after accepting failed.
  const [syncError, setSyncError] = useState<string | null>(null);

  // Each time it opens, start clean.
  const [seeded, setSeeded] = useState(false);
  if (open && !seeded) {
    setSeeded(true);
    setLink("");
    setError(errorKey ? t(errorKey) : null);
    setJoinedLater(false);
    setSyncError(null);
    setExpiredFor(null);
    setBusy(null);
    setLoading(true);
  } else if (!open && seeded) {
    setSeeded(false);
  }

  // What was waiting (the person came back from signing in), or else a field to paste in.
  useEffect(() => {
    if (!open) return;
    let cancelled = false;
    pendingInvitation()
      .then((waiting) => {
        if (cancelled) return;
        setOpened(waiting.state === "waiting" ? waiting : null);
        if (waiting.state === "expired") {
          setExpiredFor(waiting.libraryName);
          // It is told now; it is not told again the next time this opens.
          void discardPendingInvitation().catch(() => {});
        }
      })
      .catch(() => !cancelled && setOpened(null))
      .finally(() => !cancelled && setLoading(false));
    return () => {
      cancelled = true;
    };
  }, [open]);

  const peek = async () => {
    setBusy("open");
    setError(null);
    try {
      setOpened(await openInvitation(link));
      setExpiredFor(null);
      setLink("");
    } catch (e) {
      setError(t(invitationErrorKey(e)));
    } finally {
      setBusy(null);
    }
  };

  const decline = async () => {
    await discardPendingInvitation().catch(() => {});
    setOpened(null);
    closeOverlay();
  };

  const enter = async () => {
    setBusy("accept");
    setError(null);
    let joined: AcceptedInvitation;
    try {
      joined = await acceptInvitation();
    } catch (e) {
      setError(t(invitationErrorKey(e)));
      // Whatever the server said about it, the invitation is gone unless a retry can fix it.
      await pendingInvitation()
        .then((waiting) => setOpened(waiting.state === "waiting" ? waiting : null))
        .catch(() => setOpened(null));
      setBusy(null);
      return;
    }
    // From here the invitation is used: what fails is the screen's refresh, not the invitation.
    try {
      await refreshCloudStatus();
      libraries.reload();
      if (joined.library) {
        const notices = cloudNotices(joined.reports, libraries.all, t, false);
        const message =
          notices.length > 0
            ? notices.join(" ")
            : t("invitations.open.joined", { library: joined.libraryName ?? opened?.libraryName });
        await libraries.switchTo(joined.library);
        closeOverlay();
        onDone(message);
        return;
      }
    } catch {
      // Falls through to «you are in; it will arrive with the next sync».
    } finally {
      setBusy(null);
    }
    setOpened(null);
    setSyncError(joined.syncError);
    setJoinedLater(true);
  };

  const buttons = cn("flex gap-2", desktop ? "justify-end" : "flex-col-reverse");
  const size = cn(!desktop && "h-12");
  const errorBlock = error && (
    <Alert variant="destructive">
      <AlertDescription>{error}</AlertDescription>
    </Alert>
  );

  const content = () => {
    if (loading) {
      return <Loader2 className="size-5 animate-spin text-muted-foreground" aria-hidden />;
    }
    if (joinedLater) {
      return (
        <>
          <p className="text-sm leading-relaxed text-pretty">{t("invitations.open.joinedLater")}</p>
          {syncError && (
            <Alert variant="destructive">
              <AlertDescription>{t(invitationReasonKey(syncError))}</AlertDescription>
            </Alert>
          )}
          <div className={buttons}>
            <Button className={size} onClick={closeOverlay}>
              {t("common.close")}
            </Button>
          </div>
        </>
      );
    }
    if (busy === "accept") {
      return (
        <div role="status" className="flex flex-col gap-2 text-sm text-pretty">
          <p className="flex items-center gap-2 font-medium">
            <Loader2 className="size-4 animate-spin" /> {t("cloud.download.title")}
          </p>
          <p className="text-muted-foreground">{t("cloud.download.lead")}</p>
        </div>
      );
    }
    if (opened) {
      const when = expiryText(opened.expiresAt, t, i18n.resolvedLanguage);
      return (
        <>
          <div className="flex flex-col gap-1.5">
            <p className="text-lg leading-snug font-semibold">
              {t("invitations.open.invited", { library: opened.libraryName })}
            </p>
            <p className="text-[13px] text-muted-foreground">
              {t("invitations.open.validUntil", { when })}
            </p>
          </div>
          {account.signedIn ? (
            <p className="text-sm leading-relaxed text-pretty">
              {t("invitations.open.signedInBody", {
                library: opened.libraryName,
                account: account.user?.email ?? "",
              })}
            </p>
          ) : (
            <p className="text-sm leading-relaxed text-pretty">
              {t("invitations.open.signInFirst")}
            </p>
          )}
          {errorBlock}
          <div className={buttons}>
            <Button variant="outline" className={size} onClick={() => void decline()}>
              {t("invitations.open.decline")}
            </Button>
            {account.signedIn ? (
              <Button className={size} onClick={() => void enter()}>
                {t("invitations.open.enter", { library: opened.libraryName })}
              </Button>
            ) : (
              <>
                <Button
                  variant="outline"
                  className={size}
                  onClick={() => openOverlay({ name: "auth", step: "signup" })}
                >
                  {t("invitations.open.signUp")}
                </Button>
                <Button className={size} onClick={() => openOverlay({ name: "auth" })}>
                  {t("invitations.open.signIn")}
                </Button>
              </>
            )}
          </div>
        </>
      );
    }
    return (
      <form
        className="flex flex-col gap-4"
        onSubmit={(e) => {
          e.preventDefault();
          if (link.trim() !== "" && busy === null) void peek();
        }}
      >
        <label className="flex flex-col gap-1.5">
          <span className="text-sm font-medium">{t("invitations.open.field")}</span>
          <Input
            value={link}
            onChange={(e) => setLink(e.target.value)}
            disabled={busy !== null}
            placeholder={t("invitations.open.placeholder")}
            inputMode="url"
            autoFocus
            autoComplete="off"
            autoCapitalize="none"
            spellCheck={false}
          />
          <span className="text-[13px] text-muted-foreground">{t("invitations.open.hint")}</span>
        </label>
        {expiredFor !== null && (
          <Alert>
            <AlertDescription>
              {t("invitations.open.expired", { library: expiredFor })}
            </AlertDescription>
          </Alert>
        )}
        {errorBlock}
        <div className={buttons}>
          <Button type="button" variant="outline" className={size} onClick={closeOverlay}>
            {t("common.cancel")}
          </Button>
          <Button type="submit" className={size} disabled={link.trim() === "" || busy !== null}>
            {busy === "open" && <Loader2 className="animate-spin" />} {t("invitations.open.look")}
          </Button>
        </div>
      </form>
    );
  };

  return (
    <SheetOrDialog
      open={open}
      desktop={desktop}
      onClose={closeOverlay}
      title={t("invitations.open.title")}
      description={t("invitations.open.description")}
      icon={
        <span className="flex size-10 shrink-0 items-center justify-center rounded-full bg-muted text-muted-foreground">
          <MailOpen className="size-5" />
        </span>
      }
    >
      {content()}
    </SheetOrDialog>
  );
}
