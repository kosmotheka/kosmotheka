import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { Check, Copy, Loader2, Mail, Link2, QrCode, Share2, UserRoundPlus } from "lucide-react";

import { SheetOrDialog } from "@/components/shell/sheet-or-dialog";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  type InvitationKind,
  type InviteResult,
  type InviteStatus,
  type Person,
  inviteStatus,
  inviteToLibrary,
  invitationErrorKey,
  invitationReasonKey,
} from "@/lib/api";
import { expiryText } from "@/lib/invitations";
import { navigate, openOverlay } from "@/lib/navigation";
import { cn } from "@/lib/utils";

type Done = { result: InviteResult; name: string; email: string | null; kind: InvitationKind };

/**
 * Inviting from People (D43, enmienda del 10/10/2026): two ways, no role to pick, because a new person
 * always enters as a reader. Rust says whether the library can invite (`inviteStatus`) and what each person
 * may be offered (`Person.invitable`); this only shows it. The link of a link invitation lives in this
 * component's state and nowhere else: closing the dialog loses it.
 */
export function InviteDialog({
  libraryId,
  library,
  person,
  desktop,
  onClose,
  onInvited,
}: {
  libraryId: string;
  library: string;
  /** Someone already in the library (by email, keeping their role), or nobody: a new person. */
  person: Person | null;
  desktop: boolean;
  onClose: () => void;
  /** The invitation list is out of date. */
  onInvited: () => void;
  /** «Turn on people and roles» was chosen. */
}) {
  const { t, i18n } = useTranslation();
  const [status, setStatus] = useState<InviteStatus | { failed: string } | null>(null);
  // Bumped by «Try again»: the status is asked once more.
  const [attempt, setAttempt] = useState(0);
  const modeButtons = useRef<Record<string, HTMLButtonElement | null>>({});
  const [kind, setKind] = useState<InvitationKind>("email");
  const [name, setName] = useState("");
  const [email, setEmail] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [done, setDone] = useState<Done | null>(null);
  const [copied, setCopied] = useState(false);
  const [confirmClose, setConfirmClose] = useState(false);
  const existing = person !== null;
  const link = done?.result.invitation?.link ?? null;

  useEffect(() => {
    let cancelled = false;
    inviteStatus(libraryId)
      .then((loaded) => !cancelled && setStatus(loaded))
      .catch((e: unknown) => !cancelled && setStatus({ failed: invitationErrorKey(e) }));
    return () => {
      cancelled = true;
    };
  }, [libraryId, attempt]);

  /** A link that was not copied is lost on closing: ask once. */
  const requestClose = () => {
    if (link && !confirmClose) setConfirmClose(true);
    else onClose();
  };

  const submit = async () => {
    setBusy(true);
    setError(null);
    const asked: InvitationKind = existing ? "email" : kind;
    try {
      const result = await inviteToLibrary(libraryId, {
        person: existing ? { type: "existing", member: person.id } : { type: "new", name },
        kind: asked,
        email: asked === "email" ? email : null,
      });
      onInvited();
      setDone({
        result,
        name: existing ? person.name : name.trim(),
        email: asked === "email" ? email.trim() : null,
        kind: asked,
      });
    } catch (e) {
      setError(t(invitationErrorKey(e)));
    } finally {
      setBusy(false);
    }
  };

  const copy = async () => {
    if (!link) return;
    try {
      await navigator.clipboard.writeText(link);
      setCopied(true);
      window.setTimeout(() => setCopied(false), 2500);
    } catch {
      setError(t("invitations.invite.copyFailed"));
    }
  };

  const share = () => {
    if (!link) return;
    void navigator.share({ title: library, url: link }).catch(() => {});
  };

  const buttons = cn("flex gap-2", desktop ? "justify-end" : "flex-col-reverse");
  const size = cn(!desktop && "h-12");
  const title = existing
    ? t("invitations.invite.titleExisting", { person: person.name })
    : t("invitations.invite.title", { library });
  const canShare = typeof navigator.share === "function";

  const blocked = status && !("failed" in status) && status.blocker;
  const body = () => {
    if (status === null) {
      return (
        <p className="flex items-center gap-2 text-sm text-muted-foreground">
          <Loader2 className="size-4 animate-spin" /> {t("invitations.invite.checking")}
        </p>
      );
    }
    if ("failed" in status) {
      return (
        <>
          <Alert variant="destructive">
            <AlertDescription>{t(status.failed)}</AlertDescription>
          </Alert>
          <div className={buttons}>
            <Button variant="outline" className={size} onClick={onClose}>
              {t("common.close")}
            </Button>
            <Button
              className={size}
              onClick={() => {
                setStatus(null);
                setAttempt((n) => n + 1);
              }}
            >
              {t("invitations.invite.retry")}
            </Button>
          </div>
        </>
      );
    }
    if (blocked) {
      const action: Record<string, (() => void) | undefined> = {
        not_signed_in: () => openOverlay({ name: "auth" }),
        not_in_cloud: () => openOverlay({ name: "cloudPick" }),
        no_cloud_power: () => navigate("library"),
      };
      const go = action[blocked];
      return (
        <>
          <p className="text-sm leading-relaxed text-pretty">
            {t(`invitations.blocked.${blocked}.body`, { library })}
          </p>
          <div className={buttons}>
            <Button variant="outline" className={size} onClick={onClose}>
              {t("common.close")}
            </Button>
            {go && (
              <Button
                className={size}
                onClick={() => {
                  onClose();
                  go();
                }}
              >
                {t(`invitations.blocked.${blocked}.action`)}
              </Button>
            )}
          </div>
        </>
      );
    }
    if (done) return finished(done);
    return form();
  };

  const finished = ({ result, name: who, email: to, kind: asked }: Done) => {
    const invitation = result.invitation;
    if (result.failed || !invitation) {
      return (
        <>
          <p className="text-sm leading-relaxed text-pretty">
            {result.personCreated
              ? t("invitations.result.createdButFailed", { person: who })
              : t("invitations.result.failed")}
          </p>
          {result.failed && (
            <Alert variant="destructive">
              <AlertDescription>{t(invitationReasonKey(result.failed))}</AlertDescription>
            </Alert>
          )}
          {result.personCreated && (
            <p className="text-[13px] text-muted-foreground">
              {t("invitations.result.retryByEmail", { person: who })}
            </p>
          )}
          <div className={buttons}>
            <Button className={size} onClick={onClose}>
              {t("common.close")}
            </Button>
          </div>
        </>
      );
    }
    const when = expiryText(invitation.expiresAt, t, i18n.resolvedLanguage);
    if (asked === "email") {
      return (
        <>
          <p className="flex items-start gap-2.5 text-sm leading-relaxed text-pretty">
            <Mail className="mt-0.5 size-[18px] shrink-0 text-muted-foreground" />
            <span>{t("invitations.result.sentTo", { email: to })}</span>
          </p>
          <p className="text-[13px] text-muted-foreground">
            {t("invitations.result.once", { when })}
          </p>
          <div className={buttons}>
            <Button className={size} onClick={onClose}>
              {t("invitations.result.done")}
            </Button>
          </div>
        </>
      );
    }
    return (
      <>
        <p className="text-sm font-medium">{t("invitations.result.linkFor", { person: who })}</p>
        <Input
          readOnly
          value={link ?? ""}
          aria-label={t("invitations.result.linkLabel")}
          className="h-10 font-mono text-[13px]"
          onFocus={(e) => e.currentTarget.select()}
        />
        <div className={cn("flex gap-2", !desktop && "flex-col")}>
          <Button className={cn("flex-1", size)} onClick={() => void copy()}>
            {copied ? <Check /> : <Copy />}{" "}
            {copied ? t("invitations.result.copied") : t("invitations.result.copy")}
          </Button>
          {canShare && (
            <Button variant="outline" className={cn("flex-1", size)} onClick={share}>
              <Share2 /> {t("invitations.result.share")}
            </Button>
          )}
        </div>
        <p className="text-[13px] text-muted-foreground">
          {t("invitations.result.once", { when })}
        </p>
        <p className="text-[13px] text-muted-foreground">
          {t("invitations.result.readerOnly", { person: who })}
        </p>
        {error && (
          <Alert variant="destructive">
            <AlertDescription>{error}</AlertDescription>
          </Alert>
        )}
        {confirmClose ? (
          <div role="alert" className="flex flex-col gap-3 rounded-xl bg-warning-soft p-3.5">
            <p className="text-sm text-pretty">{t("invitations.result.closeWarning")}</p>
            <div className={buttons}>
              <Button variant="outline" className={size} onClick={() => setConfirmClose(false)}>
                {t("invitations.result.back")}
              </Button>
              <Button className={size} onClick={onClose}>
                {t("invitations.result.closeAnyway")}
              </Button>
            </div>
          </div>
        ) : (
          <div className={buttons}>
            <Button variant="outline" className={size} onClick={requestClose}>
              {t("invitations.result.done")}
            </Button>
          </div>
        )}
      </>
    );
  };

  const form = () => {
    const modes: { key: InvitationKind; icon: typeof Mail }[] = [
      { key: "email", icon: Mail },
      { key: "link", icon: Link2 },
    ];
    const asked: InvitationKind = existing ? "email" : kind;
    const ready = (existing || name.trim() !== "") && (asked === "link" || email.trim() !== "");
    return (
      <form
        className="flex flex-col gap-4"
        onSubmit={(e) => {
          e.preventDefault();
          if (ready && !busy) void submit();
        }}
      >
        {!existing && (
          <div role="radiogroup" aria-label={title} className="flex flex-col gap-2 sm:flex-row">
            {modes.map(({ key, icon: Icon }) => (
              <button
                key={key}
                type="button"
                role="radio"
                aria-checked={kind === key}
                ref={(node) => {
                  modeButtons.current[key] = node;
                }}
                // One stop in the group, and the arrows move inside it, as in any group of options.
                tabIndex={kind === key ? 0 : -1}
                onClick={() => {
                  setKind(key);
                  setError(null);
                }}
                onKeyDown={(e) => {
                  const step = { ArrowRight: 1, ArrowDown: 1, ArrowLeft: -1, ArrowUp: -1 }[e.key];
                  if (step === undefined) return;
                  e.preventDefault();
                  const at = modes.findIndex((mode) => mode.key === key) + step;
                  const next = modes[(at + modes.length) % modes.length]?.key ?? key;
                  setKind(next);
                  setError(null);
                  modeButtons.current[next]?.focus();
                }}
                className={cn(
                  "flex flex-1 flex-col gap-1 rounded-2xl border p-3.5 text-left",
                  kind === key && "border-primary bg-primary/8",
                )}
              >
                <span className="flex items-center gap-2 text-[15px] font-medium">
                  <Icon className="size-4" /> {t(`invitations.invite.modes.${key}.label`)}
                </span>
                <span className="text-[13px] leading-snug text-muted-foreground">
                  {t(`invitations.invite.modes.${key}.hint`)}
                </span>
              </button>
            ))}
          </div>
        )}
        {existing && (
          <p className="text-sm leading-relaxed text-pretty">
            {t("invitations.invite.existingBody", { person: person.name })}
          </p>
        )}
        {!existing && (
          <label className="flex flex-col gap-1.5">
            <span className="text-sm font-medium">{t("invitations.invite.name")}</span>
            <Input
              value={name}
              onChange={(e) => setName(e.target.value)}
              disabled={busy}
              autoFocus
              autoComplete="off"
            />
            <span className="text-[13px] text-muted-foreground">
              {t("invitations.invite.nameHint")}
            </span>
          </label>
        )}
        {asked === "email" && (
          <label className="flex flex-col gap-1.5">
            <span className="text-sm font-medium">{t("invitations.invite.emailField")}</span>
            <Input
              type="email"
              inputMode="email"
              value={email}
              onChange={(e) => setEmail(e.target.value)}
              disabled={busy}
              autoFocus={existing}
              autoComplete="off"
            />
            <span className="text-[13px] text-muted-foreground">
              {t("invitations.invite.emailHint")}
            </span>
          </label>
        )}
        <p className="text-[13px] leading-snug text-pretty text-muted-foreground">
          {existing
            ? t("invitations.invite.keepsRole", { person: person.name })
            : t("invitations.invite.readerNote")}
        </p>
        {error && (
          <Alert variant="destructive">
            <AlertDescription>{error}</AlertDescription>
          </Alert>
        )}
        <div className={buttons}>
          <Button type="button" variant="outline" className={size} onClick={onClose}>
            {t("common.cancel")}
          </Button>
          <Button type="submit" className={size} disabled={!ready || busy}>
            {busy ? <Loader2 className="animate-spin" /> : <UserRoundPlus />}{" "}
            {t(asked === "email" ? "invitations.invite.sendEmail" : "invitations.invite.makeLink")}
          </Button>
        </div>
        {!existing && (
          <button
            type="button"
            className="flex items-center gap-2 self-start text-[13px] font-medium text-primary"
            onClick={() => {
              onClose();
              openOverlay({ name: "pair" });
            }}
          >
            <QrCode className="size-4" /> {t("invitations.invite.useQr")}
          </button>
        )}
      </form>
    );
  };

  return (
    <SheetOrDialog
      open
      desktop={desktop}
      onClose={requestClose}
      title={title}
      description={t("invitations.invite.description", { library })}
      icon={
        <span className="flex size-10 shrink-0 items-center justify-center rounded-full bg-muted text-muted-foreground">
          <UserRoundPlus className="size-5" />
        </span>
      }
    >
      {body()}
    </SheetOrDialog>
  );
}
