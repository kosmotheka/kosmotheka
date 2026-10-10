import { useState } from "react";
import { useTranslation } from "react-i18next";
import { LibraryBig, LogOut, Smartphone } from "lucide-react";

import { useAccount } from "@/lib/account";
import { type LibrarySummary, dismissAccountNotice } from "@/lib/api";
import { openOverlay } from "@/lib/navigation";

const WELCOME_SEEN = "kosmotheka.welcomeSeen";

function welcomeSeen(): boolean {
  try {
    return localStorage.getItem(WELCOME_SEEN) !== null;
  } catch {
    // Storage unavailable (private mode, blocked site data…): act as already seen,
    // so the welcome screen never gets stuck showing on every launch.
    return true;
  }
}

function markWelcomeSeen() {
  try {
    localStorage.setItem(WELCOME_SEEN, "1");
  } catch {
    // Nothing to do: the welcome screen just may show again next time.
  }
}

/** A device with nothing in it yet, so the welcome screen never shows over real data. */
function isEmpty(libraries: LibrarySummary[]): boolean {
  return libraries.length === 0 || libraries.every((library) => library.works === 0);
}

const openAuth = () => openOverlay({ name: "auth" });

/**
 * Full-screen welcome (first run) and `device_removed`, plus the `sessionClosed`
 * dialog (Notice.dc.html). Mounted once in App.tsx.
 */
export function AccountNotice({ libraries }: { libraries: LibrarySummary[] }) {
  const { t } = useTranslation();
  const account = useAccount();
  const [dismissed, setDismissed] = useState(false);
  const showWelcome = !dismissed && !welcomeSeen() && isEmpty(libraries);

  const finishWelcome = () => {
    markWelcomeSeen();
    setDismissed(true);
  };

  const dismiss = () => {
    void dismissAccountNotice();
  };

  if (account.notice === "device_removed") {
    return (
      <div className="fixed inset-0 z-50 flex flex-col items-center justify-center gap-6 bg-background p-7 text-center">
        <span className="flex size-14 items-center justify-center rounded-full bg-secondary text-secondary-foreground">
          <Smartphone className="size-[26px]" />
        </span>
        <div className="flex max-w-sm flex-col gap-2.5">
          <h1 className="text-[26px] font-semibold tracking-tight">
            {t("account.notice.removed.title")}
          </h1>
          <p className="text-[15px] text-pretty text-muted-foreground">
            {t("account.notice.removed.body")}
          </p>
        </div>
        <div className="flex w-full max-w-sm flex-col gap-3">
          <button
            type="button"
            className="h-12 rounded-xl bg-primary text-[15px] font-semibold text-primary-foreground"
            onClick={dismiss}
          >
            {t("account.notice.removed.start")}
          </button>
          <button
            type="button"
            className="h-12 rounded-xl border bg-transparent text-[15px] font-medium"
            onClick={() => {
              dismiss();
              openAuth();
            }}
          >
            {t("account.notice.removed.signIn")}
          </button>
          <p className="text-[13px] text-pretty text-muted-foreground">
            {t("account.notice.removed.note")}
          </p>
        </div>
      </div>
    );
  }

  if (account.notice === "signed_out_password_changed") {
    return (
      <div className="fixed inset-0 z-50 flex items-center justify-center bg-scrim p-4">
        <div className="flex w-full max-w-sm flex-col gap-6 rounded-2xl bg-background p-7 text-center shadow-lg">
          <span className="mx-auto flex size-14 items-center justify-center rounded-full bg-secondary text-secondary-foreground">
            <LogOut className="size-[26px]" />
          </span>
          <div className="flex flex-col gap-2.5">
            <h1 className="text-[22px] font-semibold tracking-tight">
              {t("account.notice.sessionClosed.title")}
            </h1>
            <p className="text-[15px] text-pretty text-muted-foreground">
              {t("account.notice.sessionClosed.body")}
            </p>
          </div>
          <div className="flex flex-col gap-3">
            <button
              type="button"
              className="h-12 rounded-xl bg-primary text-[15px] font-semibold text-primary-foreground"
              onClick={() => {
                dismiss();
                openAuth();
              }}
            >
              {t("account.notice.sessionClosed.signIn")}
            </button>
            <button
              type="button"
              className="h-12 rounded-xl border bg-transparent text-[15px] font-medium"
              onClick={dismiss}
            >
              {t("account.notice.sessionClosed.notNow")}
            </button>
          </div>
        </div>
      </div>
    );
  }

  if (showWelcome) {
    return (
      <div className="fixed inset-0 z-50 flex flex-col items-center justify-center gap-6 bg-background p-7 text-center">
        <div className="flex items-center gap-2.5">
          <span className="flex size-10 items-center justify-center rounded-[11px] bg-foreground text-background">
            <LibraryBig className="size-[21px]" />
          </span>
          <span className="text-[19px] font-semibold tracking-tight">Kosmotheka</span>
        </div>
        <div className="flex max-w-sm flex-col gap-2.5">
          <h1 className="text-[28px] font-semibold tracking-tight text-pretty">
            {t("account.notice.welcome.title")}
          </h1>
          <p className="text-base text-pretty text-muted-foreground">
            {t("account.notice.welcome.body")}
          </p>
        </div>
        <div className="flex w-full max-w-sm flex-col gap-3">
          <button
            type="button"
            className="h-12 rounded-xl bg-primary text-[15px] font-semibold text-primary-foreground"
            onClick={finishWelcome}
          >
            {t("account.notice.welcome.start")}
          </button>
          <button
            type="button"
            className="h-12 rounded-xl border bg-transparent text-[15px] font-medium"
            onClick={() => {
              finishWelcome();
              openAuth();
            }}
          >
            {t("account.notice.welcome.signIn")}
          </button>
          <button
            type="button"
            className="h-10 rounded-xl text-[14px] font-medium text-primary"
            onClick={() => {
              finishWelcome();
              openOverlay({ name: "invitation" });
            }}
          >
            {t("invitations.open.entry")}
          </button>
          {/* order matches Notice.dc.html: filled primary action first, outline second */}
          <p className="text-[13px] text-pretty text-muted-foreground">
            {t("account.notice.welcome.note")}
          </p>
        </div>
      </div>
    );
  }

  return null;
}
