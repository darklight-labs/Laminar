import { useCallback, useEffect, useMemo, useState } from "react";
import { invoke, isTauri } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import { flushSync } from "react-dom";

import { ImportScreen } from "./components/ImportScreen";
import { QrDisplayScreen } from "./components/QrDisplayScreen";
import { ReceiptScreen } from "./components/ReceiptScreen";
import { ReviewScreen } from "./components/ReviewScreen";
import type {
  DraftLoadedResponse,
  Network,
  QrOutput,
  QrStrategy,
  Receipt,
  SplitQrOutput,
  ValidateBatchResponse
} from "./lib/types";
import {
  parseGenerateQrResponse,
  parseValidateBatchResponse
} from "./lib/validation";

type Screen = "import" | "review" | "qr_display" | "receipt";
type Theme = "dark" | "light";
type LoadingPhase = "validate" | "generate" | null;

interface OperationProgressEvent {
  operation_id: string;
  phase: string;
  step: number;
  total_steps: number;
  title: string;
  detail: string;
}
const APP_VERSION = "0.1.0-alpha";
const STARTUP_LOADING_MS = 900;
const MIN_OPERATION_OVERLAY_MS = 450;
const VALIDATE_LOADING_HINTS = [
  "Reading batch file from disk...",
  "Parsing CSV/JSON rows...",
  "Validating recipient addresses...",
  "Checking amounts, precision, and memo rules..."
];
const GENERATE_LOADING_HINTS = [
  "Reading and validating recipients...",
  "Constructing deterministic ZIP-321 payload...",
  "Rendering batch QR frame sequence...",
  "Generating split-recipient fallback QRs...",
  "Building receipt metadata and hashes..."
];

function waitForUiPaint(): Promise<void> {
  return new Promise((resolve) => {
    if (typeof window === "undefined") {
      resolve();
      return;
    }

    window.requestAnimationFrame(() => {
      window.setTimeout(() => resolve(), 0);
    });
  });
}

function waitForMs(durationMs: number): Promise<void> {
  return new Promise((resolve) => {
    window.setTimeout(resolve, durationMs);
  });
}

function createOperationId(): string {
  if (typeof crypto !== "undefined" && typeof crypto.randomUUID === "function") {
    return crypto.randomUUID();
  }
  return `${Date.now()}-${Math.random().toString(16).slice(2)}`;
}

function normalizeNetwork(value: string): Network {
  return value === "testnet" ? "testnet" : "mainnet";
}

function normalizePathInput(value: string): string {
  const trimmed = value.trim().replace(/^["']|["']$/g, "");
  if (!trimmed.toLowerCase().startsWith("file://")) {
    return trimmed;
  }

  try {
    const parsed = new URL(trimmed);
    if (parsed.protocol !== "file:") {
      return trimmed;
    }

    const decoded = decodeURIComponent(parsed.pathname);
    if (/^\/[A-Za-z]:\//.test(decoded)) {
      return decoded.slice(1).replace(/\//g, "\\");
    }
    return decoded;
  } catch {
    return trimmed;
  }
}

export default function App() {
  const [theme, setTheme] = useState<Theme>(() => {
    if (typeof window === "undefined") {
      return "dark";
    }
    return window.localStorage.getItem("laminar-theme") === "light" ? "light" : "dark";
  });
  const [screen, setScreen] = useState<Screen>("import");
  const [batch, setBatch] = useState<ValidateBatchResponse | null>(null);
  const [qrOutput, setQrOutput] = useState<QrOutput | null>(null);
  const [splitQrOutputs, setSplitQrOutputs] = useState<SplitQrOutput[]>([]);
  const [qrStrategy, setQrStrategy] = useState<QrStrategy>("split");
  const [receipt, setReceipt] = useState<Receipt | null>(null);
  const [network, setNetwork] = useState<Network>("mainnet");
  const [filePath, setFilePath] = useState("");
  const [startupLoading, setStartupLoading] = useState(true);
  const [loading, setLoading] = useState(false);
  const [loadingPhase, setLoadingPhase] = useState<LoadingPhase>(null);
  const [activeOperationId, setActiveOperationId] = useState<string | null>(null);
  const [operationProgressStep, setOperationProgressStep] = useState(0);
  const [operationProgressTotal, setOperationProgressTotal] = useState(0);
  const [loadingTitle, setLoadingTitle] = useState("Processing");
  const [loadingDetail, setLoadingDetail] = useState("Running secure pipeline...");
  const [loadingHintIndex, setLoadingHintIndex] = useState(0);
  const [busyStartedAt, setBusyStartedAt] = useState<number | null>(null);
  const [busyElapsedSeconds, setBusyElapsedSeconds] = useState(0);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (typeof window !== "undefined") {
      window.localStorage.setItem("laminar-theme", theme);
    }
  }, [theme]);

  useEffect(() => {
    const timer = window.setTimeout(() => {
      setStartupLoading(false);
    }, STARTUP_LOADING_MS);

    return () => {
      window.clearTimeout(timer);
    };
  }, []);

  const showingBusyOverlay = startupLoading || loading;

  useEffect(() => {
    if (!showingBusyOverlay) {
      setBusyStartedAt(null);
      setBusyElapsedSeconds(0);
      return;
    }

    setBusyStartedAt(Date.now());
    setBusyElapsedSeconds(0);
  }, [showingBusyOverlay, loadingTitle]);

  useEffect(() => {
    if (!showingBusyOverlay || busyStartedAt === null) {
      return;
    }

    const timer = window.setInterval(() => {
      const elapsed = Math.max(0, Math.floor((Date.now() - busyStartedAt) / 1000));
      setBusyElapsedSeconds(elapsed);
    }, 250);

    return () => {
      window.clearInterval(timer);
    };
  }, [showingBusyOverlay, busyStartedAt]);

  useEffect(() => {
    if (!loading || !loadingPhase || operationProgressTotal > 0) {
      setLoadingHintIndex(0);
      return;
    }

    const hints = loadingPhase === "validate" ? VALIDATE_LOADING_HINTS : GENERATE_LOADING_HINTS;
    if (hints.length <= 1) {
      setLoadingHintIndex(0);
      return;
    }

    setLoadingHintIndex(0);
    const timer = window.setInterval(() => {
      setLoadingHintIndex((previous) => {
        const next = (previous + 1) % hints.length;
        setLoadingDetail(hints[next] ?? hints[0]);
        return next;
      });
    }, 1300);

    return () => {
      window.clearInterval(timer);
    };
  }, [loading, loadingPhase, operationProgressTotal]);

  useEffect(() => {
    if (!isTauri()) {
      return;
    }

    let mounted = true;
    let unlisten: (() => void) | null = null;
    void listen<OperationProgressEvent>("laminar://operation-progress", (event) => {
      if (!mounted || !activeOperationId) {
        return;
      }

      const payload = event.payload;
      if (!payload || payload.operation_id !== activeOperationId) {
        return;
      }

      setLoadingTitle(payload.title);
      setLoadingDetail(payload.detail);
      setOperationProgressStep(Math.max(0, payload.step));
      setOperationProgressTotal(Math.max(0, payload.total_steps));
    })
      .then((dispose) => {
        if (!mounted) {
          dispose();
          return;
        }
        unlisten = dispose;
      })
      .catch(() => null);

    return () => {
      mounted = false;
      if (unlisten) {
        unlisten();
      }
    };
  }, [activeOperationId]);

  const handleNetworkChange = useCallback((nextNetwork: Network) => {
    setNetwork(nextNetwork);
    setError(null);
    setScreen("import");
    setBatch(null);
    setQrOutput(null);
    setSplitQrOutputs([]);
    setQrStrategy("split");
    setReceipt(null);
    setActiveOperationId(null);
    setOperationProgressStep(0);
    setOperationProgressTotal(0);
  }, []);

  const handlePickFile = useCallback(async () => {
    const selected = (await open({
      multiple: false,
      directory: false,
      filters: [
        { name: "CSV", extensions: ["csv"] },
        { name: "JSON", extensions: ["json"] }
      ]
    })) as string | string[] | null;

    if (typeof selected === "string") {
      setFilePath(normalizePathInput(selected));
      setError(null);
      return;
    }

    if (Array.isArray(selected) && selected.length > 0) {
      setFilePath(normalizePathInput(selected[0] ?? ""));
      setError(null);
    }
  }, []);

  const validateBatch = useCallback(
    async (pathOverride?: string) => {
      const targetPath = normalizePathInput(pathOverride ?? filePath);
      if (!targetPath) {
        setError("Select a CSV or JSON file to continue.");
        return;
      }
      const operationId = createOperationId();

      flushSync(() => {
        setLoadingPhase("validate");
        setActiveOperationId(operationId);
        setOperationProgressStep(0);
        setOperationProgressTotal(0);
        setLoading(true);
        setLoadingTitle("Validating Batch");
        setLoadingDetail(VALIDATE_LOADING_HINTS[0]);
        setError(null);
      });
      const operationStartedAt = Date.now();
      try {
        await waitForUiPaint();
        const rawValidated = await invoke("validate_batch", {
          filePath: targetPath,
          network,
          operationId
        });
        const validated = parseValidateBatchResponse(rawValidated);
        setFilePath(targetPath);
        setBatch(validated);
        setQrOutput(null);
        setSplitQrOutputs([]);
        setScreen("review");
      } catch (err) {
        setError(String(err));
      } finally {
        const elapsedMs = Date.now() - operationStartedAt;
        if (elapsedMs < MIN_OPERATION_OVERLAY_MS) {
          await waitForMs(MIN_OPERATION_OVERLAY_MS - elapsedMs);
        }
        setActiveOperationId(null);
        setOperationProgressStep(0);
        setOperationProgressTotal(0);
        setLoadingPhase(null);
        setLoading(false);
      }
    },
    [filePath, network]
  );

  const proceedToQr = useCallback(async () => {
    if (!filePath) {
      setError("No batch file selected.");
      return;
    }
    const operationId = createOperationId();

    flushSync(() => {
      setLoadingPhase("generate");
      setActiveOperationId(operationId);
      setOperationProgressStep(0);
      setOperationProgressTotal(0);
      setLoading(true);
      setLoadingTitle("Constructing Payment Request");
      setLoadingDetail(GENERATE_LOADING_HINTS[0]);
      setError(null);
    });
    const operationStartedAt = Date.now();
    try {
      await waitForUiPaint();
      const rawGenerated = await invoke("generate_qr", {
        filePath: filePath,
        network,
        operationId
      });
      const generated = parseGenerateQrResponse(rawGenerated);

      setQrOutput(generated.qr_output);
      setSplitQrOutputs(generated.split_qr_outputs ?? []);
      setReceipt(generated.receipt);
      setScreen("qr_display");
    } catch (err) {
      setError(String(err));
    } finally {
      const elapsedMs = Date.now() - operationStartedAt;
      if (elapsedMs < MIN_OPERATION_OVERLAY_MS) {
        await waitForMs(MIN_OPERATION_OVERLAY_MS - elapsedMs);
      }
      setActiveOperationId(null);
      setOperationProgressStep(0);
      setOperationProgressTotal(0);
      setLoadingPhase(null);
      setLoading(false);
    }
  }, [filePath, network]);

  const networkTone = useMemo(() => (network === "mainnet" ? "network-mainnet" : "network-testnet"), [network]);
  const stepIndex = useMemo(() => {
    switch (screen) {
      case "import":
        return 0;
      case "review":
        return 1;
      case "qr_display":
        return 2;
      case "receipt":
        return 3;
      default:
        return 0;
    }
  }, [screen]);
  const navSteps = useMemo(
    () => [
      { label: "Import", key: "import" },
      { label: "Review", key: "review" },
      { label: "Handoff", key: "qr_display" },
      { label: "Receipt", key: "receipt" }
    ],
    []
  );
  const currentScreen = useMemo(() => {
    if (screen === "import") {
      return (
        <ImportScreen
          filePath={filePath}
          network={network}
          loading={loading}
          error={error}
          onFilePathChange={setFilePath}
          onPickFile={handlePickFile}
          onValidate={validateBatch}
          onDraftLoaded={(draft: DraftLoadedResponse) => {
            const nextNetwork = normalizeNetwork(draft.summary.network);
            setNetwork(nextNetwork);
            setBatch({
              validated_batch: draft.validated_batch,
              summary: draft.summary
            });
            setQrOutput(null);
            setSplitQrOutputs([]);
            setFilePath(`[draft] ${draft.draft.name}`);
            setScreen("review");
            setError(null);
          }}
        />
      );
    }

    if (screen === "review" && batch) {
      return (
        <ReviewScreen
          batch={batch}
          network={network}
          loading={loading}
          error={error}
          onBack={() => {
            setScreen("import");
            setError(null);
          }}
          onProceed={proceedToQr}
          qrStrategy={qrStrategy}
          onQrStrategyChange={setQrStrategy}
        />
      );
    }

    if (screen === "qr_display" && qrOutput) {
      return (
        <QrDisplayScreen
          batchQrOutput={qrOutput}
          splitQrOutputs={splitQrOutputs}
          strategy={qrStrategy}
          onStrategyChange={setQrStrategy}
          onBack={() => {
            setScreen("review");
            setError(null);
          }}
          onGenerateReceipt={() => {
            setScreen("receipt");
            setError(null);
          }}
        />
      );
    }

    if (screen === "receipt" && receipt) {
      return (
        <ReceiptScreen
          receipt={receipt}
          onNewBatch={() => {
            void invoke("clear_sensitive_memory").catch(() => null);
            setScreen("import");
            setBatch(null);
            setQrOutput(null);
            setSplitQrOutputs([]);
            setQrStrategy("split");
            setReceipt(null);
            setFilePath("");
            setError(null);
            setActiveOperationId(null);
            setOperationProgressStep(0);
            setOperationProgressTotal(0);
            setLoadingPhase(null);
            setLoading(false);
          }}
        />
      );
    }

    return null;
  }, [
    batch,
    error,
    filePath,
    handlePickFile,
    loading,
    network,
    proceedToQr,
    qrOutput,
    qrStrategy,
    receipt,
    screen,
    splitQrOutputs,
    validateBatch
  ]);

  const activeLoadingTitle = startupLoading ? "Initializing Laminar" : loadingTitle;
  const activeOperationHints =
    loadingPhase === "validate"
      ? VALIDATE_LOADING_HINTS
      : loadingPhase === "generate"
      ? GENERATE_LOADING_HINTS
      : [];
  const hasOperationHints = loading && activeOperationHints.length > 0;
  const boundedHintIndex = hasOperationHints
    ? Math.min(loadingHintIndex, activeOperationHints.length - 1)
    : 0;
  const effectiveProgressTotal =
    operationProgressTotal > 0
      ? operationProgressTotal
      : hasOperationHints
      ? activeOperationHints.length
      : 0;
  const effectiveProgressStep =
    operationProgressStep > 0 ? operationProgressStep : hasOperationHints ? boundedHintIndex + 1 : 0;
  const loadingProgressPercent =
    !startupLoading && effectiveProgressTotal > 0
      ? Math.max(6, Math.min(100, Math.round((effectiveProgressStep / effectiveProgressTotal) * 100)))
      : null;
  const activeLoadingDetail = startupLoading
    ? "Preparing secure local runtime..."
    : loadingDetail;
  const activeLoadingMeta = startupLoading
    ? "Air-gapped mode | no external network calls"
    : effectiveProgressTotal > 0
    ? `Step ${Math.min(effectiveProgressStep, effectiveProgressTotal)} of ${effectiveProgressTotal} | Elapsed ${busyElapsedSeconds}s`
    : busyElapsedSeconds > 0
    ? `Elapsed ${busyElapsedSeconds}s`
    : "Working...";

  return (
    <div className="app-container" data-theme={theme}>
      <div className="title-bar">
        <span className="title-bar-text">{`Laminar ${APP_VERSION}`}</span>
      </div>

      <header className="app-header">
        <div className="header-left">
          <div className="logo-mark">
            <h1 className="app-title">LAMINAR</h1>
            <span className="version-badge">Alpha</span>
          </div>
        </div>

        <nav className="header-nav" aria-label="Workflow steps">
          {navSteps.map((entry, index) => {
            const state =
              index < stepIndex ? "completed" : index === stepIndex ? "active" : "pending";
            return (
              <div className={`nav-step nav-step-${state}`} key={entry.key}>
                <span className="nav-step-number">{index + 1}</span>
                <span>{entry.label}</span>
              </div>
            );
          })}
        </nav>

        <div className="header-right">
          <button
            type="button"
            className="btn btn-ghost"
            onClick={() => setTheme((previous) => (previous === "dark" ? "light" : "dark"))}
          >
            {theme === "dark" ? "Light Mode" : "Dark Mode"}
          </button>

          <div className="network-selector">
            <button
              type="button"
              className={`network-option ${network === "mainnet" ? "active" : ""} ${networkTone}`}
              onClick={() => handleNetworkChange("mainnet")}
              disabled={loading}
            >
              Mainnet
            </button>
            <button
              type="button"
              className={`network-option ${network === "testnet" ? "active" : ""} ${networkTone}`}
              onClick={() => handleNetworkChange("testnet")}
              disabled={loading}
            >
              Testnet
            </button>
          </div>
        </div>
      </header>

      <main className={`main-content ${showingBusyOverlay ? "main-content-loading" : ""}`}>
        <div className="screen-shell" key={screen}>
          {currentScreen}
        </div>
        <section
          className={`loading-overlay ${showingBusyOverlay ? "active" : ""} ${
            startupLoading ? "loading-overlay-startup" : ""
          } ${loading && !startupLoading ? "loading-overlay-operation" : ""} ${
            loading ? "loading-overlay-working" : ""
          }`}
          aria-live="polite"
          aria-busy={showingBusyOverlay}
        >
          <div className="loading-shell">
            <div className="loading-sigil" aria-hidden="true">
              <span />
              <span />
              <span />
            </div>
            <div className="loading-spinner" />
            <p className="loading-text">{activeLoadingTitle}</p>
            <p className="loading-detail">{activeLoadingDetail}</p>
            <div className="loading-progress-track" aria-hidden="true">
              <div
                className={`loading-progress-fill ${
                  loadingProgressPercent !== null ? "loading-progress-fill-determinate" : ""
                }`}
                style={loadingProgressPercent !== null ? { width: `${loadingProgressPercent}%` } : undefined}
              />
            </div>
            <p className="loading-meta">{activeLoadingMeta}</p>
          </div>
        </section>
      </main>

      <footer className="status-bar">
        <div className="status-left">
          <span className="status-indicator">
            <span className="status-dot" />
            <span>Air-gapped</span>
          </span>
          <span>No network requests</span>
        </div>
        <div className="status-right">
          <span>Operator Mode</span>
          <span>{APP_VERSION}</span>
        </div>
      </footer>
    </div>
  );
}

