import { invoke as tauriInvoke } from "@tauri-apps/api/core";
import {
  Activity,
  AlertTriangle,
  Bot,
  CheckCircle2,
  ChevronRight,
  CircleDollarSign,
  Clock3,
  KeyRound,
  Loader2,
  Plus,
  RefreshCcw,
  Settings2,
  ShieldAlert,
  Trash2,
  Zap,
} from "lucide-react";
import { FormEvent, useEffect, useMemo, useState } from "react";

type ProviderStatus = "ok" | "warning" | "critical" | "unknown" | "error";
type UsageUnit = "credits" | "usd" | "tokens" | "requests" | "percentage" | "unknown";

interface ProviderView {
  id: string;
  providerType: string;
  displayName: string;
  status: ProviderStatus;
  used?: number | null;
  limit?: number | null;
  remaining?: number | null;
  percentage?: number | null;
  unit: UsageUnit;
  resetAt?: string | null;
  lastRefreshAt?: string | null;
  message?: string | null;
}

interface DashboardPayload {
  providers: ProviderView[];
  globalStatus: ProviderStatus;
}

const statusCopy: Record<ProviderStatus, { label: string; blurb: string }> = {
  ok: { label: "Healthy", blurb: "Within limit" },
  warning: { label: "Near limit", blurb: "Watch usage" },
  critical: { label: "Critical", blurb: "Almost out" },
  unknown: { label: "Partial", blurb: "Limited data" },
  error: { label: "Action needed", blurb: "Refresh failed" },
};

const fmt = new Intl.NumberFormat(undefined, { maximumFractionDigits: 2 });
const isTauriRuntime = () => typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

async function invoke<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  if (isTauriRuntime()) return tauriInvoke<T>(command, args);
  return mockInvoke<T>(command, args);
}

async function mockInvoke<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  const raw = window.localStorage.getItem("quotatray:web-providers");
  const providers: ProviderView[] = raw ? JSON.parse(raw) : [];
  const save = (next: ProviderView[]) => window.localStorage.setItem("quotatray:web-providers", JSON.stringify(next));
  const mockProvider = (): ProviderView => {
    const percentage = [32, 76, 94, 58][Math.floor(Date.now() / 60_000) % 4];
    return {
      id: "mock-default",
      providerType: "mock",
      displayName: "Mock",
      status: percentage >= 90 ? "critical" : percentage >= 70 ? "warning" : "ok",
      used: percentage * 10,
      limit: 1000,
      remaining: 1000 - percentage * 10,
      percentage,
      unit: "requests",
      resetAt: new Date(Date.now() + 3 * 60 * 60 * 1000).toISOString(),
      lastRefreshAt: new Date().toISOString(),
      message: "Browser preview mode. Run npm run tauri:dev for real provider/keychain support.",
    };
  };

  if (command === "get_dashboard") {
    const mockOnly = providers.filter((provider) => provider.providerType === "mock");
    if (mockOnly.length !== providers.length) save(mockOnly);
    const globalStatus = computeGlobalStatus(mockOnly);
    return { providers: mockOnly, globalStatus } as T;
  }
  if (command === "add_mock_provider") {
    save([mockProvider(), ...providers.filter((provider) => provider.id !== "mock-default")]);
    return undefined as T;
  }
  if (command === "add_codex_provider") {
    throw new Error("Codex requires the desktop app. Run `npm run tauri:dev:msys` so QuotaTray can read ~/.codex/auth.json.");
  }
  if (command === "add_opencode_go_provider") {
    throw new Error("OpenCode Go requires the desktop app. Run `npm run tauri:dev:msys` so QuotaTray can read ~/.local/share/opencode/opencode.db.");
  }
  if (command === "add_openai_provider") {
    void args;
    throw new Error("OpenAI requires the desktop app. Run `npm run tauri:dev:msys` for real API key validation.");
  }
  if (command === "add_openrouter_provider") {
    void args;
    throw new Error("OpenRouter requires the desktop app. Run `npm run tauri:dev:msys` for real API key validation.");
  }
  if (command === "refresh_all") {
    save(providers.map((provider) => provider.providerType === "mock" ? mockProvider() : { ...provider, lastRefreshAt: new Date().toISOString() }));
    return undefined as T;
  }
  if (command === "refresh_provider") {
    const providerId = args?.providerId as string;
    save(providers.map((provider) => provider.id === providerId && provider.providerType === "mock" ? mockProvider() : provider.id === providerId ? { ...provider, lastRefreshAt: new Date().toISOString() } : provider));
    return undefined as T;
  }
  if (command === "remove_provider") {
    const providerId = args?.providerId as string;
    save(providers.filter((provider) => provider.id !== providerId));
    return undefined as T;
  }
  throw new Error(`Unsupported preview command: ${command}`);
}

function computeGlobalStatus(providers: ProviderView[]): ProviderStatus {
  const statuses = providers.map((provider) => provider.status);
  if (statuses.length === 0) return "unknown";
  if (statuses.includes("error")) return "error";
  if (statuses.includes("critical")) return "critical";
  if (statuses.includes("warning")) return "warning";
  if (statuses.every((status) => status === "unknown")) return "unknown";
  return "ok";
}

export function App() {
  const [dashboard, setDashboard] = useState<DashboardPayload>({ providers: [], globalStatus: "unknown" });
  const [selectedProviderId, setSelectedProviderId] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [refreshing, setRefreshing] = useState<string | null>(null);
  const [panel, setPanel] = useState<"dashboard" | "add">("dashboard");
  const [apiKey, setApiKey] = useState("");
  const [authMode, setAuthMode] = useState<"api_key" | "login">("api_key");
  const [error, setError] = useState<string | null>(null);

  const selectedProvider = useMemo(() => {
    return dashboard.providers.find((provider) => provider.id === selectedProviderId) ?? dashboard.providers[0] ?? null;
  }, [dashboard.providers, selectedProviderId]);

  async function load() {
    setError(null);
    try {
      const payload = await invoke<DashboardPayload>("get_dashboard");
      setDashboard(payload);
      if (!selectedProviderId && payload.providers[0]) setSelectedProviderId(payload.providers[0].id);
    } catch (err) {
      setError(toMessage(err));
    } finally {
      setLoading(false);
    }
  }

  async function refreshAll() {
    setRefreshing("all");
    setError(null);
    try {
      await invoke("refresh_all");
      await load();
    } catch (err) {
      setError(toMessage(err));
    } finally {
      setRefreshing(null);
    }
  }

  async function refreshProvider(providerId: string) {
    setRefreshing(providerId);
    setError(null);
    try {
      await invoke("refresh_provider", { providerId });
      await load();
    } catch (err) {
      setError(toMessage(err));
    } finally {
      setRefreshing(null);
    }
  }

  async function addMock() {
    setRefreshing("add-mock");
    setError(null);
    try {
      await invoke("add_mock_provider");
      setPanel("dashboard");
      await load();
    } catch (err) {
      setError(toMessage(err));
    } finally {
      setRefreshing(null);
    }
  }

  async function addCodex() {
    setRefreshing("add-codex");
    setError(null);
    try {
      await invoke("add_codex_provider");
      setPanel("dashboard");
      await load();
    } catch (err) {
      setError(toMessage(err));
    } finally {
      setRefreshing(null);
    }
  }

  async function addOpenCodeGo() {
    setRefreshing("add-opencode-go");
    setError(null);
    try {
      await invoke("add_opencode_go_provider");
      setPanel("dashboard");
      await load();
    } catch (err) {
      setError(toMessage(err));
    } finally {
      setRefreshing(null);
    }
  }

  async function addOpenRouter(event: FormEvent) {
    event.preventDefault();
    if (!apiKey.trim()) return;
    setRefreshing("add-openrouter");
    setError(null);
    try {
      await invoke("add_openrouter_provider", { apiKey: apiKey.trim() });
      setApiKey("");
      setPanel("dashboard");
      await load();
    } catch (err) {
      setError(toMessage(err));
    } finally {
      setRefreshing(null);
    }
  }

  async function addOpenAI(event: FormEvent) {
    event.preventDefault();
    if (!apiKey.trim()) return;
    setRefreshing("add-openai");
    setError(null);
    try {
      await invoke("add_openai_provider", { apiKey: apiKey.trim() });
      setApiKey("");
      setPanel("dashboard");
      await load();
    } catch (err) {
      setError(toMessage(err));
    } finally {
      setRefreshing(null);
    }
  }

  async function removeProvider(providerId: string) {
    setRefreshing(providerId);
    setError(null);
    try {
      await invoke("remove_provider", { providerId });
      if (selectedProviderId === providerId) setSelectedProviderId(null);
      await load();
    } catch (err) {
      setError(toMessage(err));
    } finally {
      setRefreshing(null);
    }
  }

  useEffect(() => {
    load();
    const id = window.setInterval(load, 60_000);
    return () => window.clearInterval(id);
  }, []);

  return (
    <main className="shell">
      <section className="popover">
        <ProviderRail
          providers={dashboard.providers}
          selectedProviderId={selectedProvider?.id ?? null}
          onSelect={(id) => {
            setSelectedProviderId(id);
            setPanel("dashboard");
          }}
          onAdd={() => setPanel("add")}
        />

        {error && <div className="errorBanner"><ShieldAlert size={16} /> {error}</div>}

        {panel === "add" ? (
          <AddProvider
            apiKey={apiKey}
            setApiKey={setApiKey}
            authMode={authMode}
            setAuthMode={setAuthMode}
            addOpenRouter={addOpenRouter}
            addOpenAI={addOpenAI}
            addCodex={addCodex}
            addOpenCodeGo={addOpenCodeGo}
            addMock={addMock}
            busy={refreshing}
          />
        ) : loading ? (
          <div className="empty"><Loader2 className="spin" /> Loading usage…</div>
        ) : selectedProvider ? (
          <ProviderDetail
            provider={selectedProvider}
            globalStatus={dashboard.globalStatus}
            busy={refreshing === selectedProvider.id || refreshing === "all"}
            onRefresh={() => refreshProvider(selectedProvider.id)}
            onRefreshAll={refreshAll}
            onRemove={() => removeProvider(selectedProvider.id)}
            onAdd={() => setPanel("add")}
          />
        ) : (
          <EmptyState onAdd={() => setPanel("add")} onMock={addMock} busy={refreshing === "add-mock"} />
        )}
      </section>
    </main>
  );
}

function ProviderRail({
  providers,
  selectedProviderId,
  onSelect,
  onAdd,
}: {
  providers: ProviderView[];
  selectedProviderId: string | null;
  onSelect: (id: string) => void;
  onAdd: () => void;
}) {
  return (
    <header className="providerRail" aria-label="Providers">
      {providers.map((provider) => (
        <button
          key={provider.id}
          className={`providerTab ${selectedProviderId === provider.id ? "active" : ""}`}
          onClick={() => onSelect(provider.id)}
          title={provider.displayName}
        >
          <ProviderIcon providerType={provider.providerType} />
          <span>{provider.displayName}</span>
          <i className={`statusLine status-${provider.status}`} />
        </button>
      ))}
      <button className="providerTab addTab" onClick={onAdd} title="Add provider">
        <Plus size={18} />
        <span>Add</span>
        <i className="statusLine" />
      </button>
    </header>
  );
}

function ProviderDetail({
  provider,
  globalStatus,
  busy,
  onRefresh,
  onRefreshAll,
  onRemove,
  onAdd,
}: {
  provider: ProviderView;
  globalStatus: ProviderStatus;
  busy: boolean;
  onRefresh: () => void;
  onRefreshAll: () => void;
  onRemove: () => void;
  onAdd: () => void;
}) {
  const percentage = clamp(provider.percentage ?? inferPercentage(provider));
  const meterStyle = { "--meter": `${percentage}%` } as React.CSSProperties;
  const copy = statusCopy[provider.status];
  const global = statusCopy[globalStatus];

  return (
    <section className="detail">
      <div className="titleRow">
        <div>
          <h1>{provider.displayName}</h1>
          <p>{provider.lastRefreshAt ? `Updated ${relativeTime(provider.lastRefreshAt)}` : "Not refreshed yet"}</p>
        </div>
        <span className={`statusText statusText-${provider.status}`}>{copy.label}</span>
      </div>

      <div className="rule" />

      <UsageSection
        title="Usage"
        percentage={percentage}
        meterStyle={meterStyle}
        footerLeft={`${Math.round(percentage)}% used`}
        footerRight={provider.resetAt ? `Resets ${relativeTimeFuture(provider.resetAt)}` : "Reset unknown"}
        status={provider.status}
      />

      <div className="detailGrid">
        <Metric label="Used" value={valueWithUnit(provider.used, provider.unit)} />
        <Metric label="Limit" value={valueWithUnit(provider.limit, provider.unit)} />
        <Metric label="Remaining" value={valueWithUnit(provider.remaining, provider.unit)} />
      </div>

      {provider.message && <p className="messageText">{provider.message}</p>}

      <div className="rule" />

      <section className="costBlock">
        <div>
          <h2>Summary</h2>
          <p>Global status: {global.label} · {global.blurb}</p>
        </div>
        <ChevronRight size={21} />
      </section>

      <div className="rule" />

      <nav className="actionList" aria-label="Actions">
        <button onClick={onAdd}><KeyRound size={19} /> Add provider</button>
        <button onClick={onRefresh} disabled={busy}><RefreshCcw className={busy ? "spin" : ""} size={19} /> Refresh current</button>
        <button onClick={onRefreshAll} disabled={busy}><Activity size={19} /> Refresh all</button>
        <button><Settings2 size={19} /> Settings</button>
      </nav>

      <div className="rule" />

      <button className="removeButton" onClick={onRemove} disabled={busy}>
        <Trash2 size={18} /> Remove {provider.displayName}
      </button>
    </section>
  );
}

function UsageSection({
  title,
  percentage,
  meterStyle,
  footerLeft,
  footerRight,
  status,
}: {
  title: string;
  percentage: number;
  meterStyle: React.CSSProperties;
  footerLeft: string;
  footerRight: string;
  status: ProviderStatus;
}) {
  return (
    <section className="usageSection">
      <h2>{title}</h2>
      <div className={`meter meter-${status}`} style={meterStyle} aria-label={`Usage ${Math.round(percentage)} percent`}>
        <span />
      </div>
      <div className="usageFooter">
        <strong>{footerLeft}</strong>
        <span>{footerRight}</span>
      </div>
    </section>
  );
}

function AddProvider({
  apiKey,
  setApiKey,
  authMode,
  setAuthMode,
  addOpenRouter,
  addOpenAI,
  addCodex,
  addOpenCodeGo,
  addMock,
  busy,
}: {
  apiKey: string;
  setApiKey: (value: string) => void;
  authMode: "api_key" | "login";
  setAuthMode: (value: "api_key" | "login") => void;
  addOpenRouter: (event: FormEvent) => void;
  addOpenAI: (event: FormEvent) => void;
  addCodex: () => void;
  addOpenCodeGo: () => void;
  addMock: () => void;
  busy: string | null;
}) {
  return (
    <section className="addPanel">
      <div className="addHeader">
        <h1>Add provider</h1>
        <p>Connect with login auth when a provider supports OAuth, or paste an API key directly.</p>
      </div>

      <div className="authSwitch" role="tablist" aria-label="Auth method">
        <button className={authMode === "login" ? "active" : ""} onClick={() => setAuthMode("login")} type="button">Login auth</button>
        <button className={authMode === "api_key" ? "active" : ""} onClick={() => setAuthMode("api_key")} type="button">API key</button>
      </div>

      {authMode === "login" ? (
        <>
          <section className="loginPanel">
            <h2>Codex local login</h2>
            <p>
              Uses your existing Codex session from ~/.codex/auth.json. If the token is stale, QuotaTray will try to refresh it with the stored refresh token.
            </p>
            <button className="primaryButton" onClick={addCodex} disabled={!!busy} type="button">
              {busy === "add-codex" ? <Loader2 className="spin" size={16} /> : <Plus size={16} />} Connect Codex
            </button>
            <p className="smallNote">For regular OpenAI API usage, switch to API key. ChatGPT web quota is not exposed as a general public API.</p>
          </section>

          <section className="loginPanel">
            <h2>OpenCode Go local usage</h2>
            <p>
              Reads your local OpenCode history from ~/.local/share/opencode/opencode.db and estimates 5-hour, weekly, and monthly quota usage.
            </p>
            <button className="primaryButton" onClick={addOpenCodeGo} disabled={!!busy} type="button">
              {busy === "add-opencode-go" ? <Loader2 className="spin" size={16} /> : <Plus size={16} />} Connect OpenCode Go
            </button>
            <p className="smallNote">This local monitor does not need your OpenCode Go password. Web subscription quota via cookies can be added later if needed.</p>
          </section>
        </>
      ) : null}

      {authMode === "api_key" ? <>
      <div className="providerChoice">
        <div>
          <h2>Mock Provider</h2>
          <p>Preview usage states without an API key.</p>
        </div>
        <button className="secondaryButton" onClick={addMock} disabled={!!busy}>
          {busy === "add-mock" ? <Loader2 className="spin" size={16} /> : <Zap size={16} />} Add mock
        </button>
      </div>

      <form className="providerChoice" onSubmit={addOpenAI}>
        <div>
          <h2>OpenAI / GPT</h2>
          <p>Connect an OpenAI API key. ChatGPT Plus/Pro subscriptions do not expose quota usage here.</p>
        </div>
        <label className="keyField">
          <KeyRound size={16} />
          <input
            value={apiKey}
            onChange={(event) => setApiKey(event.target.value)}
            placeholder="sk-proj-… or sk-…"
            type="password"
            autoComplete="off"
          />
        </label>
        <button className="primaryButton" disabled={!apiKey.trim() || !!busy}>
          {busy === "add-openai" ? <Loader2 className="spin" size={16} /> : <Plus size={16} />} Connect OpenAI
        </button>
      </form>

      <form className="providerChoice secondaryProvider" onSubmit={addOpenRouter}>
        <div>
          <h2>OpenRouter</h2>
          <p>Optional: track OpenRouter credit usage via API key.</p>
        </div>
        <label className="keyField">
          <KeyRound size={16} />
          <input
            value={apiKey}
            onChange={(event) => setApiKey(event.target.value)}
            placeholder="sk-or-v1-…"
            type="password"
            autoComplete="off"
          />
        </label>
        <button className="secondaryButton" disabled={!apiKey.trim() || !!busy}>
          {busy === "add-openrouter" ? <Loader2 className="spin" size={16} /> : <Plus size={16} />} Connect OpenRouter
        </button>
      </form>
      </> : null}
    </section>
  );
}

function EmptyState({ onAdd, onMock, busy }: { onAdd: () => void; onMock: () => void; busy: boolean }) {
  return (
    <section className="emptyState">
      <div className="emptyIcon">QT</div>
      <h1>QuotaTray</h1>
      <p>Add OpenRouter for real quota data, or use mock data to preview usage states.</p>
      <div className="emptyActions">
        <button className="primaryButton" onClick={onAdd}><Plus size={16} /> Add provider</button>
        <button className="secondaryButton" onClick={onMock} disabled={busy}>
          {busy ? <Loader2 className="spin" size={16} /> : <Zap size={16} />} Try mock
        </button>
      </div>
    </section>
  );
}

function ProviderIcon({ providerType }: { providerType: string }) {
  if (providerType === "codex") return <Bot size={22} />;
  if (providerType === "opencode_go") return <Activity size={22} />;
  if (providerType === "openai") return <Bot size={22} />;
  if (providerType === "openrouter") return <CircleDollarSign size={22} />;
  if (providerType === "mock") return <Bot size={22} />;
  return <Activity size={22} />;
}

function Metric({ label, value }: { label: string; value: string }) {
  return <div><span>{label}</span><strong>{value}</strong></div>;
}

function inferPercentage(provider: ProviderView) {
  if (provider.used != null && provider.limit != null && provider.limit > 0) {
    return (provider.used / provider.limit) * 100;
  }
  return 0;
}

function clamp(value: number) {
  if (!Number.isFinite(value)) return 0;
  return Math.max(0, Math.min(100, value));
}

function valueWithUnit(value: number | null | undefined, unit: UsageUnit) {
  if (value == null) return "—";
  if (unit === "usd") return `$${fmt.format(value)}`;
  if (unit === "percentage") return `${fmt.format(value)}%`;
  if (unit === "unknown") return fmt.format(value);
  return `${fmt.format(value)} ${unit}`;
}

function relativeTime(input: string) {
  const date = new Date(input);
  if (Number.isNaN(date.getTime())) return "unknown";
  const seconds = Math.max(0, Math.floor((Date.now() - date.getTime()) / 1000));
  if (seconds < 60) return "just now";
  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) return `${minutes}m ago`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours}h ago`;
  return `${Math.floor(hours / 24)}d ago`;
}

function relativeTimeFuture(input: string) {
  const date = new Date(input);
  if (Number.isNaN(date.getTime())) return "unknown";
  const seconds = Math.max(0, Math.floor((date.getTime() - Date.now()) / 1000));
  if (seconds < 60) return "in <1m";
  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) return `in ${minutes}m`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `in ${hours}h ${minutes % 60}m`;
  return `in ${Math.floor(hours / 24)}d ${hours % 24}h`;
}

function toMessage(err: unknown) {
  if (typeof err === "string") return err;
  if (err instanceof Error) return err.message;
  return "Something went wrong.";
}
