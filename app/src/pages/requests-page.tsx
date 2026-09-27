import {
  type ReactNode,
  createContext,
  useCallback,
  useContext,
  useEffect,
  useRef,
  useState,
} from "react";
import { type TFunction } from "i18next";
import { useTranslation } from "react-i18next";
import {
  ArrowLeft,
  Check,
  CircleCheck,
  CircleX,
  Clock,
  Inbox,
  Pencil,
  Plus,
  RotateCcw,
  Trash2,
  TriangleAlert,
} from "lucide-react";

import { AddBookDialog } from "@/components/add-book-dialog";
import { Cover } from "@/components/cover";
import { useLibraries, useLibraryName } from "@/components/libraries-context";
import { Avatar } from "@/components/people/role-parts";
import { Button } from "@/components/ui/button";
import {
  type ChangeRequest,
  type NewBook,
  type People,
  type RequestStatus,
  acceptRequest,
  acceptRequestEdited,
  listPeople,
  listRequests,
  permissionErrorKey,
  rejectRequest,
  requestedBook,
  withdrawRequest,
} from "@/lib/api";
import { leaveSection } from "@/lib/navigation";
import { cn } from "@/lib/utils";

type Tab = "pending" | "resolved";

/**
 * Requests in the active library (Design, phase 5 §3, D73). Administrators see what
 * others ask and decide; everyone else sees what they asked and may withdraw it.
 */
export function RequestsPage({
  desktop,
  version,
  onChanged,
  onNotice,
}: {
  desktop: boolean;
  version: number;
  onChanged: () => void;
  onNotice: (message: string) => void;
}) {
  const { t } = useTranslation();
  const libraries = useLibraries();
  const library = useLibraryName()(libraries.active);
  const [requests, setRequests] = useState<ChangeRequest[] | null>(null);
  const [people, setPeople] = useState<People | null>(null);
  const [tab, setTab] = useState<Tab>("pending");
  const [selected, setSelected] = useState<string | null>(null);
  const [editing, setEditing] = useState<{ id: string; book: NewBook } | null>(null);
  const [busy, setBusy] = useState(false);
  const [now, setNow] = useState(0);
  // Requests seen pending here and resolved elsewhere meanwhile get a banner.
  const seenPending = useRef(new Set<string>());
  const resolvedHere = useRef(new Set<string>());
  const [elsewhere, setElsewhere] = useState<ReadonlySet<string>>(new Set());

  useEffect(() => {
    let cancelled = false;
    Promise.all([listRequests(), listPeople()])
      .then(([loaded, who]) => {
        if (cancelled) return;
        const resolvedMeanwhile = loaded.filter(
          (r) =>
            r.status !== "pending" &&
            seenPending.current.has(r.id) &&
            !resolvedHere.current.has(r.id),
        );
        for (const r of loaded) {
          if (r.status === "pending") seenPending.current.add(r.id);
          else seenPending.current.delete(r.id);
        }
        if (resolvedMeanwhile.length > 0) {
          setElsewhere((previous) => new Set([...previous, ...resolvedMeanwhile.map((r) => r.id)]));
        }
        setRequests(loaded);
        setPeople(who);
        setNow(Date.now());
      })
      .catch((e: unknown) => !cancelled && onNotice(String(e)));
    return () => {
      cancelled = true;
    };
  }, [version, onNotice]);

  const admin = people?.myRole === "admin";
  const admins = people?.people.filter((p) => p.role === "admin").map((p) => p.name) ?? [];
  const pending = (requests ?? []).filter((r) => r.status === "pending");
  const resolved = (requests ?? []).filter((r) => r.status !== "pending");
  const roleOf = (r: ChangeRequest) =>
    people?.people.find((p) => p.devices.some((d) => d.id === r.deviceId))?.role ?? null;
  const resolvedElsewhere = (r: ChangeRequest) => elsewhere.has(r.id);
  const dismiss = (id: string) => {
    setElsewhere((previous) => new Set([...previous].filter((other) => other !== id)));
    setSelected(null);
  };

  const act = useCallback(
    async (id: string, action: () => Promise<unknown>, done: string) => {
      setBusy(true);
      // Kept selected so a request resolved elsewhere meanwhile shows who did it.
      setSelected(id);
      try {
        await action();
        resolvedHere.current.add(id);
        onNotice(done);
      } catch (e) {
        const key = permissionErrorKey(e);
        // Resolved by someone else meanwhile: the reload shows who did it.
        if (!key?.endsWith("request_resolved")) onNotice(key ? t(key) : String(e));
      } finally {
        setBusy(false);
        onChanged();
      }
    },
    [onChanged, onNotice, t],
  );

  const accept = (r: ChangeRequest) =>
    act(r.id, () => acceptRequest(r.id), t("requests.accepted", { what: describe(t, r) }));
  const reject = (r: ChangeRequest) =>
    act(r.id, () => rejectRequest(r.id), t("requests.rejected", { what: describe(t, r) }));
  const withdraw = (r: ChangeRequest) =>
    act(r.id, () => withdrawRequest(r.id), t("requests.withdrawn", { what: describe(t, r) }));
  const edit = async (r: ChangeRequest) => {
    try {
      setEditing({ id: r.id, book: await requestedBook(r.id) });
    } catch (e) {
      const key = permissionErrorKey(e);
      onNotice(key ? t(key) : String(e));
    }
  };

  const header = desktop ? (
    <div className="flex flex-col gap-1">
      <h1 className="text-[22px] font-semibold tracking-tight">
        {admin ? t("requests.title") : t("requests.mine")}
      </h1>
      <p className="text-[13px] text-muted-foreground">
        {admin
          ? t("requests.subtitle", { library })
          : t("requests.mineSubtitle", { library, admins: admins.join(", ") })}
      </p>
    </div>
  ) : (
    <header className="-mx-4 -mt-1 flex h-16 items-center gap-1 px-1">
      <Button
        variant="ghost"
        size="icon-lg"
        className="size-12 rounded-full"
        aria-label={t("book.back")}
        onClick={leaveSection}
      >
        <ArrowLeft className="size-[22px]" />
      </Button>
      <h1 className="truncate text-lg font-semibold">
        {admin ? t("requests.title") : t("requests.mine")}
      </h1>
    </header>
  );

  const wrap = (page: ReactNode) => <NowContext.Provider value={now}>{page}</NowContext.Provider>;

  if (requests === null || people === null) {
    return <div className="flex min-h-full flex-col gap-5">{header}</div>;
  }

  const editDialog = editing && (
    <AddBookDialog
      open
      desktop={desktop}
      library={library}
      title={t("requests.editTitle")}
      initial={editing.book}
      submit={(book) => acceptRequestEdited(editing.id, book)}
      onOpenChange={(open) => !open && setEditing(null)}
      onCreated={(_, title) => {
        resolvedHere.current.add(editing.id);
        onNotice(t("requests.acceptedEdited", { title }));
        onChanged();
      }}
    />
  );

  if (!admin) {
    const mine = requests.filter((r) => r.mine);
    return wrap(
      <div className="flex min-h-full flex-col gap-5">
        {header}
        {!desktop && (
          <p className="-mt-3 px-0.5 text-[13px] leading-snug text-pretty text-muted-foreground">
            {t("requests.mineSubtitleShort", {
              library,
              admins: admins.join(", "),
            })}
          </p>
        )}
        {mine.length === 0 ? (
          <EmptyList text={t("requests.noneMine")} />
        ) : (
          <MyRequests requests={mine} desktop={desktop} busy={busy} onWithdraw={withdraw} />
        )}
      </div>,
    );
  }

  const list = tab === "pending" ? pending : resolved;
  const current =
    list.find((r) => r.id === selected) ??
    requests.find((r) => r.id === selected && resolvedElsewhere(r)) ??
    list[0] ??
    null;

  const tabs = (
    <div
      role="tablist"
      className={cn(
        desktop
          ? "flex gap-[18px] border-b px-3.5 pt-3 text-[13px] font-medium"
          : "grid shrink-0 grid-cols-2 rounded-[14px] bg-muted p-1",
      )}
    >
      {(["pending", "resolved"] as const).map((option) => (
        <button
          key={option}
          type="button"
          role="tab"
          aria-selected={tab === option}
          onClick={() => {
            setTab(option);
            setSelected(null);
          }}
          className={cn(
            desktop
              ? "pb-2.5 text-muted-foreground aria-selected:text-foreground aria-selected:shadow-[inset_0_-2px_0_var(--foreground)]"
              : "h-10 rounded-[10px] text-sm text-muted-foreground aria-selected:bg-background aria-selected:font-semibold aria-selected:text-foreground aria-selected:shadow-sm",
          )}
        >
          {option === "pending"
            ? t("requests.pendingTab", { count: pending.length })
            : t("requests.resolvedTab")}
        </button>
      ))}
    </div>
  );

  const actions = (r: ChangeRequest) => ({
    onAccept: () => void accept(r),
    onReject: () => void reject(r),
    onEdit: r.editable ? () => void edit(r) : undefined,
    onDismiss: () => dismiss(r.id),
    onShowResolved: () => {
      dismiss(r.id);
      setTab("resolved");
      setSelected(r.id);
    },
  });

  if (!desktop) {
    const cards = [...requests.filter((r) => tab === "pending" && resolvedElsewhere(r)), ...list];
    return wrap(
      <div className="flex min-h-full flex-col gap-3.5">
        {header}
        {tabs}
        {cards.length === 0 ? (
          <EmptyList
            text={t(tab === "pending" ? "requests.nonePending" : "requests.noneResolved")}
          />
        ) : (
          cards.map((r) => (
            <RequestCard
              key={r.id}
              request={r}
              library={library}
              busy={busy}
              elsewhere={resolvedElsewhere(r)}
              {...actions(r)}
            />
          ))
        )}
        {editDialog}
      </div>,
    );
  }

  return wrap(
    <div className="flex h-full flex-col gap-[18px]">
      {header}
      <div className="grid min-h-0 flex-1 grid-cols-[340px_minmax(0,1fr)] overflow-hidden rounded-xl border">
        <div className="flex min-h-0 flex-col border-r">
          {tabs}
          <div className="min-h-0 flex-1 overflow-y-auto">
            {list.length === 0 && (
              <p className="p-4 text-sm text-muted-foreground">
                {t(tab === "pending" ? "requests.nonePending" : "requests.noneResolved")}
              </p>
            )}
            {list.map((r) => (
              <button
                key={r.id}
                type="button"
                onClick={() => setSelected(r.id)}
                className={cn(
                  "flex w-full items-start gap-3 border-b px-3.5 py-3 text-left hover:bg-muted/60",
                  current?.id === r.id && "bg-primary/9 hover:bg-primary/9",
                )}
              >
                <Avatar name={who(t, r)} />
                <span className="flex min-w-0 flex-1 flex-col gap-0.5">
                  <span
                    className={cn(
                      "text-sm leading-snug",
                      current?.id === r.id ? "font-semibold" : "font-medium",
                    )}
                  >
                    {sentence(t, r)}
                  </span>
                  <span className="text-xs text-muted-foreground">
                    <When at={r.at} />
                    {tab === "pending" && roleOf(r) && ` · ${t(`people.roles.${roleOf(r)}`)}`}
                    {tab === "resolved" && ` · ${t(`requests.status.${r.status}`)}`}
                  </span>
                </span>
              </button>
            ))}
          </div>
        </div>
        {current ? (
          <RequestDetail
            request={current}
            library={library}
            role={roleOf(current)}
            busy={busy}
            elsewhere={resolvedElsewhere(current)}
            {...actions(current)}
          />
        ) : (
          <div className="flex flex-col items-center justify-center gap-2 text-muted-foreground">
            <Inbox className="size-8" />
            <span className="text-sm">{t("requests.nothingSelected")}</span>
          </div>
        )}
      </div>
      {editDialog}
    </div>,
  );
}

interface Actions {
  busy: boolean;
  /** Resolved by someone else while it was open here. */
  elsewhere: boolean;
  onAccept: () => void;
  onReject: () => void;
  onEdit?: () => void;
  onDismiss: () => void;
  onShowResolved: () => void;
}

function RequestDetail({
  request: r,
  library,
  role,
  ...actions
}: Actions & { request: ChangeRequest; library: string; role: string | null }) {
  const { t } = useTranslation();
  return (
    <div className="flex min-h-0 flex-col gap-[18px] overflow-y-auto px-7 py-6">
      <div className="flex items-center gap-3">
        <Avatar name={who(t, r)} size={40} />
        <div className="flex flex-1 flex-col gap-0.5">
          <span className="text-lg font-semibold tracking-tight">{sentence(t, r)}</span>
          <span className="text-[13px] text-muted-foreground">
            <When at={r.at} long />
            {r.deviceName && ` · ${t("requests.from", { device: r.deviceName })}`}
            {role && ` · ${t("requests.roleIn", { role: t(`people.roles.${role}`), library })}`}
          </span>
        </div>
      </div>
      <Banners request={r} elsewhere={actions.elsewhere} />
      <div className="flex flex-col gap-2">
        <span className="text-xs font-medium text-muted-foreground">
          {r.status === "pending" ? t("requests.wouldChange") : t("requests.asked")}
        </span>
        <div className="flex gap-[18px] rounded-xl border p-4">
          {r.summary.title && isBook(r) && (
            <Cover alt="" className="h-[120px] w-20 shrink-0 rounded-md" />
          )}
          <div className="flex flex-1 flex-col gap-2.5">
            <ActionLabel request={r} library={library} />
            <dl className="grid grid-cols-[110px_1fr] gap-y-1.5 text-[13.5px]">
              {facts(t, r).map(([label, value]) => (
                <div key={label} className="contents">
                  <dt className="text-muted-foreground">{label}</dt>
                  <dd>{value}</dd>
                </div>
              ))}
            </dl>
          </div>
        </div>
      </div>
      <Note request={r} />
      {r.status !== "pending" && !actions.elsewhere && <Resolution request={r} />}
      <div className="mt-auto flex justify-end gap-2">
        <Buttons request={r} {...actions} />
      </div>
    </div>
  );
}

function RequestCard({
  request: r,
  library,
  ...actions
}: Actions & { request: ChangeRequest; library: string }) {
  const { t } = useTranslation();
  const line = [r.summary.format && t(`formats.${r.summary.format}`), r.summary.location]
    .filter(Boolean)
    .join(" · ");
  const byline = [t("requests.kinds.book"), r.summary.authors.join(", "), r.summary.year]
    .filter(Boolean)
    .join(" · ");
  return (
    <div className="flex flex-col gap-3 rounded-2xl border p-3.5">
      <div className="flex items-start gap-3">
        <Avatar name={who(t, r)} size={36} />
        <div className="flex flex-1 flex-col gap-0.5">
          <span className="text-[15px] leading-snug font-semibold text-pretty">
            {sentence(t, r)}
          </span>
          <span className="text-[13px] text-muted-foreground">
            <When at={r.at} />
            {r.deviceName && ` · ${t("requests.from", { device: r.deviceName })}`}
          </span>
        </div>
      </div>
      <div className="flex items-center gap-3 rounded-xl bg-muted p-3">
        {isBook(r) && <Cover alt="" className="h-[60px] w-10 shrink-0 rounded" />}
        <div className="flex min-w-0 flex-1 flex-col gap-0.5 text-[13px]">
          <ActionLabel request={r} library={library} />
          {r.summary.title && <span className="text-sm font-semibold">{r.summary.title}</span>}
          {isBook(r) && byline && <span className="text-muted-foreground">{byline}</span>}
          {line && <span className="text-muted-foreground">{line}</span>}
        </div>
      </div>
      <Banners request={r} elsewhere={actions.elsewhere} />
      <Note request={r} boxed />
      {r.status !== "pending" && !actions.elsewhere && <Resolution request={r} />}
      {(r.status === "pending" || actions.elsewhere) && (
        <div className="flex flex-col gap-2">
          <Buttons request={r} mobile {...actions} />
        </div>
      )}
    </div>
  );
}

function Buttons({
  request: r,
  mobile = false,
  busy,
  elsewhere,
  onAccept,
  onReject,
  onEdit,
  onDismiss,
  onShowResolved,
}: Actions & { request: ChangeRequest; mobile?: boolean }) {
  const { t } = useTranslation();
  const size = mobile ? "h-11 rounded-xl" : undefined;
  if (elsewhere) {
    return (
      <>
        {!mobile && (
          <Button variant="outline" onClick={onShowResolved}>
            {t("requests.showResolved")}
          </Button>
        )}
        <Button variant={mobile ? "outline" : "default"} className={size} onClick={onDismiss}>
          {t("requests.understood")}
        </Button>
      </>
    );
  }
  if (r.status !== "pending") return null;
  const accept = (
    <Button className={size} disabled={busy || !r.applies} onClick={onAccept}>
      <Check /> {t("requests.accept")}
    </Button>
  );
  const reject = (
    <Button variant="outline" className={size} disabled={busy} onClick={onReject}>
      {t("requests.reject")}
    </Button>
  );
  const edit = onEdit && (
    <Button variant="outline" className={size} disabled={busy} onClick={onEdit}>
      <Pencil /> {t("requests.editAccept")}
    </Button>
  );
  if (!mobile) {
    return (
      <>
        {reject}
        {edit}
        {accept}
      </>
    );
  }
  return (
    <>
      {accept}
      <div className={cn("grid gap-2", edit ? "grid-cols-2" : "grid-cols-1")}>
        {reject}
        {edit}
      </div>
    </>
  );
}

function Banners({ request: r, elsewhere }: { request: ChangeRequest; elsewhere: boolean }) {
  const { t } = useTranslation();
  if (elsewhere) {
    return (
      <div
        role="status"
        className="flex items-start gap-2 rounded-[10px] bg-success-soft px-3 py-2.5 text-sm leading-snug text-pretty"
      >
        <CircleCheck className="mt-0.5 size-4 shrink-0 text-success" />
        <span>
          <strong className="font-semibold">
            {t(`requests.elsewhere.${r.status}`, {
              person: r.resolvedBy ?? t("requests.someone"),
            })}
          </strong>{" "}
          {r.status === "accepted" && t("requests.elsewhere.nothingToDo")}
        </span>
      </div>
    );
  }
  if (r.status === "pending" && !r.applies) {
    return (
      <div
        role="status"
        className="flex items-start gap-2 rounded-[10px] bg-warning-soft px-3 py-2.5 text-sm leading-snug text-pretty"
      >
        <TriangleAlert className="mt-0.5 size-4 shrink-0 text-warning" />
        <span>
          <strong className="font-semibold">{t("requests.stale")}</strong>{" "}
          {r.editable ? t("requests.staleEditable") : t("requests.staleReject")}
        </span>
      </div>
    );
  }
  return null;
}

function Note({ request: r, boxed = false }: { request: ChangeRequest; boxed?: boolean }) {
  const { t } = useTranslation();
  if (!r.note) return null;
  return (
    <div className={cn("flex flex-col gap-1", boxed && "rounded-xl bg-muted px-3 py-2.5")}>
      <span className="text-xs font-medium text-muted-foreground">
        {t("requests.noteFrom", { person: who(t, r) })}
      </span>
      <span className="text-sm leading-normal text-pretty">{r.note}</span>
    </div>
  );
}

function Resolution({ request: r }: { request: ChangeRequest }) {
  return (
    <p className="flex items-center gap-2 text-[13px] text-muted-foreground">
      <StatusBadge status={r.status} />
      {r.resolvedBy}
      {r.resolvedAt !== null && (
        <>
          {" · "}
          <When at={r.resolvedAt} />
        </>
      )}
    </p>
  );
}

function ActionLabel({ request: r, library }: { request: ChangeRequest; library: string }) {
  const { t } = useTranslation();
  const removes = r.summary.action === "delete_copy" || r.summary.action === "delete_book";
  const Icon = removes ? Trash2 : r.summary.action === "edit" ? Pencil : Plus;
  return (
    <span
      className={cn(
        "flex items-center gap-1 text-xs font-semibold",
        removes ? "text-destructive" : "text-success",
      )}
    >
      <Icon className="size-[13px]" />
      {t(`requests.actions.${r.summary.action}`, { library })}
    </span>
  );
}

function MyRequests({
  requests,
  desktop,
  busy,
  onWithdraw,
}: {
  requests: ChangeRequest[];
  desktop: boolean;
  busy: boolean;
  onWithdraw: (r: ChangeRequest) => void;
}) {
  const { t } = useTranslation();
  const when = (r: ChangeRequest) =>
    r.status === "pending" || r.resolvedAt === null ? (
      <When at={r.at} long={desktop} />
    ) : (
      <>
        {r.resolvedBy ?? t("requests.someone")} · <When at={r.resolvedAt} />
      </>
    );
  const withdrawButton = (r: ChangeRequest) =>
    r.status === "pending" && (
      <Button
        variant="link"
        size="sm"
        className="h-auto px-0 underline-offset-3"
        disabled={busy}
        onClick={() => onWithdraw(r)}
      >
        {t("requests.withdraw")}
      </Button>
    );

  if (desktop) {
    const cols = "grid grid-cols-[minmax(0,1fr)_130px_170px_90px] gap-3";
    return (
      <div role="table" className="max-w-[900px] overflow-hidden rounded-xl border">
        <div
          role="row"
          className={cn(
            cols,
            "border-b bg-muted px-4 py-2.5 text-xs font-medium text-muted-foreground",
          )}
        >
          <span role="columnheader">{t("requests.whatYouAsked")}</span>
          <span role="columnheader">{t("requests.statusLabel")}</span>
          <span role="columnheader">{t("requests.when")}</span>
          <span role="columnheader" />
        </div>
        {requests.map((r) => (
          <div
            key={r.id}
            role="row"
            className={cn(cols, "min-h-14 items-center border-b px-4 last:border-b-0")}
          >
            <span role="cell" className="text-sm font-medium">
              {capitalize(describe(t, r))}
            </span>
            <span role="cell">
              <StatusBadge status={r.status} />
            </span>
            <span role="cell" className="text-[13px] text-muted-foreground">
              {when(r)}
            </span>
            <span role="cell" className="text-right">
              {withdrawButton(r)}
            </span>
          </div>
        ))}
      </div>
    );
  }

  const groups = [
    { key: "pending", items: requests.filter((r) => r.status === "pending") },
    { key: "resolved", items: requests.filter((r) => r.status !== "pending") },
  ].filter((g) => g.items.length > 0);
  return (
    <>
      {groups.map((group) => (
        <section key={group.key} className="flex flex-col gap-2">
          <h2 className="px-0.5 text-[13px] font-medium text-muted-foreground">
            {group.key === "pending"
              ? t("requests.pendingTab", { count: group.items.length })
              : t("requests.resolvedTab")}
          </h2>
          <div className="flex flex-col divide-y overflow-hidden rounded-2xl border">
            {group.items.map((r) => (
              <div key={r.id} className="flex min-h-16 items-center gap-3 px-3.5 py-2.5">
                <span className="flex min-w-0 flex-1 flex-col gap-1">
                  <span className="text-[15px] font-medium">{capitalize(describe(t, r))}</span>
                  <span className="flex items-center gap-2 text-[13px] text-muted-foreground">
                    <StatusBadge status={r.status} />
                    <span>{when(r)}</span>
                  </span>
                </span>
                {withdrawButton(r)}
              </div>
            ))}
          </div>
        </section>
      ))}
    </>
  );
}

function StatusBadge({ status }: { status: RequestStatus }) {
  const { t } = useTranslation();
  const Icon = {
    pending: Clock,
    accepted: CircleCheck,
    rejected: CircleX,
    withdrawn: RotateCcw,
  }[status];
  return (
    <span
      className={cn(
        "inline-flex h-6 shrink-0 items-center gap-1.5 rounded-full px-2.5 text-xs font-semibold whitespace-nowrap",
        status === "pending" && "bg-warning-soft text-warning",
        status === "accepted" && "bg-success-soft text-success",
        (status === "rejected" || status === "withdrawn") && "bg-muted text-muted-foreground",
      )}
    >
      <Icon className="size-[13px]" />
      {t(`requests.status.${status}`)}
    </span>
  );
}

function EmptyList({ text }: { text: string }) {
  return (
    <div className="flex flex-col items-center gap-2 py-12 text-center text-muted-foreground">
      <Inbox className="size-8" />
      <span className="text-sm">{text}</span>
    </div>
  );
}

/** When the list was loaded; "20 minutes ago" counts from then. */
const NowContext = createContext(0);

/** "Hace 20 min", "hace 3 días"; with `long`, the date and time after a day. */
function When({ at, long = false }: { at: number; long?: boolean }) {
  const { i18n } = useTranslation();
  const now = useContext(NowContext);
  const minutes = Math.max(0, Math.round((now - at) / 60000));
  const rtf = new Intl.RelativeTimeFormat(i18n.resolvedLanguage, {
    numeric: "auto",
  });
  const text =
    minutes < 60
      ? rtf.format(-minutes, "minute")
      : minutes < 60 * 24
        ? rtf.format(-Math.round(minutes / 60), "hour")
        : long
          ? new Date(at).toLocaleString(i18n.resolvedLanguage, {
              day: "numeric",
              month: "short",
              hour: "2-digit",
              minute: "2-digit",
            })
          : rtf.format(-Math.round(minutes / 1440), "day");
  return <>{capitalize(text)}</>;
}

const who = (t: TFunction, r: ChangeRequest) => r.person ?? r.deviceName ?? t("requests.someone");

const isBook = (r: ChangeRequest) =>
  r.summary.action === "add_book" || r.summary.action === "delete_book";

/** What the request does, as "add Tehanu"; the sentence and the list both use it. */
export function describe(t: TFunction, r: ChangeRequest) {
  const title = r.summary.title ?? t("requests.aBook");
  return t(`requests.describe.${r.summary.action}`, { title });
}

/** "Hugo wants to add Tehanu". */
function sentence(t: TFunction, r: ChangeRequest) {
  return t("requests.wants", { person: who(t, r), what: describe(t, r) });
}

function facts(t: TFunction, r: ChangeRequest): [string, string][] {
  const s = r.summary;
  const rows: [string, string | null][] = [
    [t("requests.fields.title"), s.title],
    [t("requests.fields.authors"), s.authors.join(", ") || null],
    [t("requests.fields.year"), [s.year, s.publisher].filter(Boolean).join(" · ") || null],
    [t("requests.fields.copy"), s.format && t(`formats.${s.format}`)],
    [t("requests.fields.where"), s.location],
  ];
  return rows.filter((row): row is [string, string] => Boolean(row[1]));
}

export const capitalize = (text: string) => text.charAt(0).toUpperCase() + text.slice(1);
