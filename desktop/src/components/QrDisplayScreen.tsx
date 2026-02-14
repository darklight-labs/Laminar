import { useEffect, useState } from "react";
import type { QrOutput, QrStrategy, SplitQrOutput } from "../lib/types";

interface QrDisplayScreenProps {
  batchQrOutput: QrOutput;
  splitQrOutputs: SplitQrOutput[];
  strategy: QrStrategy;
  onStrategyChange: (strategy: QrStrategy) => void;
  onBack: () => void;
  onGenerateReceipt: () => void;
}

function bytesToObjectUrl(bytes: number[]): string {
  const blob = new Blob([Uint8Array.from(bytes)], { type: "image/png" });
  return URL.createObjectURL(blob);
}

function truncateAddress(value: string): string {
  if (value.length <= 20) {
    return value;
  }
  return `${value.slice(0, 10)}...${value.slice(-6)}`;
}

function shuffledIndices(length: number): number[] {
  const values = Array.from({ length }, (_, index) => index);
  for (let index = values.length - 1; index > 0; index -= 1) {
    const swapIndex = Math.floor(Math.random() * (index + 1));
    const tmp = values[index];
    values[index] = values[swapIndex];
    values[swapIndex] = tmp;
  }
  return values;
}

export function QrDisplayScreen({
  batchQrOutput,
  splitQrOutputs,
  strategy,
  onStrategyChange,
  onBack,
  onGenerateReceipt
}: QrDisplayScreenProps) {
  const [frameIndex, setFrameIndex] = useState(0);
  const [isPlaying, setIsPlaying] = useState(true);
  const [batchScanConfirmed, setBatchScanConfirmed] = useState(false);
  const [splitOrder, setSplitOrder] = useState<number[]>([]);
  const [splitCursor, setSplitCursor] = useState(0);
  const [splitScanned, setSplitScanned] = useState<number[]>([]);
  const [imageUrl, setImageUrl] = useState("");
  const [imageError, setImageError] = useState<string | null>(null);

  useEffect(() => {
    setSplitOrder(splitQrOutputs.map((_, index) => index));
    setSplitCursor(0);
    setSplitScanned([]);
    setBatchScanConfirmed(false);
    setFrameIndex(0);
    setIsPlaying(true);
  }, [splitQrOutputs, batchQrOutput]);

  const splitAvailable = splitQrOutputs.length > 0;
  const activeStrategy = strategy === "split" && splitAvailable ? "split" : "batch";
  const orderedIndices =
    splitOrder.length === splitQrOutputs.length
      ? splitOrder
      : splitQrOutputs.map((_, index) => index);
  const boundedSplitCursor =
    orderedIndices.length === 0 ? 0 : Math.min(splitCursor, orderedIndices.length - 1);
  const currentSplitArrayIndex = orderedIndices[boundedSplitCursor] ?? 0;
  const currentSplit = activeStrategy === "split" ? (splitQrOutputs[currentSplitArrayIndex] ?? null) : null;
  const qrOutput = currentSplit ? currentSplit.qr_output : batchQrOutput;
  const isAnimated = qrOutput.mode === "AnimatedUr";
  const splitComplete = splitAvailable && splitScanned.length >= splitQrOutputs.length;
  const scanConfirmed = activeStrategy === "batch" ? batchScanConfirmed : splitComplete;
  const currentSplitMarked = currentSplit ? splitScanned.includes(currentSplit.index) : false;

  useEffect(() => {
    setFrameIndex(0);
    setIsPlaying(true);
  }, [activeStrategy, currentSplitArrayIndex, qrOutput.total_frames, qrOutput.payload_bytes]);

  useEffect(() => {
    if (!isAnimated || !isPlaying || scanConfirmed || qrOutput.frames.length <= 1) {
      return;
    }

    const timer = window.setInterval(() => {
      setFrameIndex((previous) => (previous + 1) % qrOutput.frames.length);
    }, 100);

    return () => window.clearInterval(timer);
  }, [isAnimated, isPlaying, scanConfirmed, qrOutput.frames.length]);

  const currentFrame = qrOutput.frames[frameIndex] ?? qrOutput.frames[0];
  useEffect(() => {
    if (!currentFrame) {
      setImageUrl("");
      return;
    }

    const nextUrl = bytesToObjectUrl(currentFrame.png_bytes);
    setImageUrl(nextUrl);
    setImageError(null);
    return () => {
      URL.revokeObjectURL(nextUrl);
    };
  }, [currentFrame]);

  const frameCounter = `Frame ${frameIndex + 1} of ${qrOutput.total_frames}`;
  const progressPercent = qrOutput.total_frames > 0 ? ((frameIndex + 1) / qrOutput.total_frames) * 100 : 0;
  const splitProgress = `${splitScanned.length} of ${splitQrOutputs.length}`;
  const splitPercent =
    splitQrOutputs.length > 0 ? Math.min(100, (splitScanned.length / splitQrOutputs.length) * 100) : 0;

  const handleMarkScanned = () => {
    if (activeStrategy === "batch") {
      setBatchScanConfirmed(true);
      setIsPlaying(false);
      return;
    }
    if (!currentSplit || currentSplitMarked) {
      return;
    }

    const nextScanned = splitScanned.includes(currentSplit.index)
      ? splitScanned
      : [...splitScanned, currentSplit.index];
    setSplitScanned(nextScanned);

    if (nextScanned.length >= splitQrOutputs.length) {
      setIsPlaying(false);
      return;
    }

    for (let step = 1; step <= orderedIndices.length; step += 1) {
      const cursor = (boundedSplitCursor + step) % orderedIndices.length;
      const candidate = splitQrOutputs[orderedIndices[cursor]];
      if (candidate && !nextScanned.includes(candidate.index)) {
        setSplitCursor(cursor);
        return;
      }
    }
  };

  return (
    <section className="panel qr-display-panel">
      <header>
        <p className="eyebrow">STEP 03</p>
        <h2 className="panel-title">QR DISPLAY</h2>
      </header>

      <section className="scan-mode-panel">
        <p className="summary-label">SCAN STRATEGY</p>
        <div className="strategy-toggle">
          <button
            type="button"
            className={`btn ${activeStrategy === "batch" ? "btn-primary" : ""}`}
            onClick={() => onStrategyChange("batch")}
          >
            Batch QR
          </button>
          <button
            type="button"
            className={`btn ${activeStrategy === "split" ? "btn-primary" : ""}`}
            onClick={() => onStrategyChange("split")}
            disabled={!splitAvailable}
          >
            Split by Recipient
          </button>
        </div>
        <p className="muted-text">
          Batch mode uses one combined payment request. Split mode emits one request per recipient for better wallet
          compatibility.
        </p>
      </section>

      <div className="qr-meta-row">
        <span className="qr-meta-chip">MODE: {qrOutput.mode === "Static" ? "STATIC" : "ANIMATED UR"}</span>
        <span className="qr-meta-chip">FRAMES: {qrOutput.total_frames}</span>
        <span className="qr-meta-chip">BYTES: {qrOutput.payload_bytes}</span>
      </div>

      {activeStrategy === "split" && currentSplit ? (
        <section className="split-recipient-panel">
          <div className="split-recipient-header">
            <p className="frame-counter">
              Recipient {boundedSplitCursor + 1} of {orderedIndices.length}
            </p>
            <p className="muted-text">Scanned {splitProgress}</p>
          </div>
          <div className="progress-track split-progress-track">
            <div className="progress-fill" style={{ width: `${splitPercent}%` }} />
          </div>
          <div className="split-recipient-grid">
            <p className="summary-label">ROW</p>
            <p className="summary-value">{currentSplit.row_number}</p>
            <p className="summary-label">ADDRESS</p>
            <p className="summary-value" title={currentSplit.address}>
              {truncateAddress(currentSplit.address)}
            </p>
            <p className="summary-label">AMOUNT</p>
            <p className="summary-value">{currentSplit.amount_zec} ZEC</p>
            <p className="summary-label">TYPE</p>
            <p className="summary-value">{currentSplit.address_type.toUpperCase()}</p>
          </div>
          <div className="button-row">
            <button
              type="button"
              className="btn"
              onClick={() => setSplitCursor((previous) => Math.max(0, previous - 1))}
              disabled={boundedSplitCursor <= 0}
            >
              &larr; Prev Recipient
            </button>
            <button
              type="button"
              className="btn"
              onClick={() => setSplitCursor((previous) => Math.min(orderedIndices.length - 1, previous + 1))}
              disabled={boundedSplitCursor >= orderedIndices.length - 1}
            >
              Next Recipient &rarr;
            </button>
            <button
              type="button"
              className="btn"
              onClick={() => {
                setSplitOrder(shuffledIndices(splitQrOutputs.length));
                setSplitCursor(0);
              }}
              disabled={splitQrOutputs.length <= 1}
            >
              Shuffle Order
            </button>
          </div>
        </section>
      ) : null}

      <div className="qr-stage">
        <div className={`qr-image-wrap ${isAnimated && isPlaying ? "scanning-active" : ""}`}>
          {currentFrame ? (
            <img
              alt="Laminar QR frame"
              className="qr-image-large"
              src={imageUrl}
              onError={() => setImageError(`Unable to render QR frame ${frameIndex + 1}.`)}
              onLoad={() => setImageError(null)}
            />
          ) : null}
        </div>
      </div>
      {imageError ? (
        <div className="error-panel">
          <p className="error-title">QR RENDER ERROR</p>
          <p className="error-message">{imageError}</p>
        </div>
      ) : null}

      {isAnimated ? (
        <section className="animation-panel">
          <div className="animation-header">
            <p className="frame-counter">{frameCounter}</p>
            <button
              type="button"
              className="btn"
              onClick={() => setIsPlaying((previous) => !previous)}
              disabled={scanConfirmed}
            >
              {isPlaying ? "Pause" : "Play"}
            </button>
          </div>
          <div className="progress-track">
            <div className="progress-fill" style={{ width: `${progressPercent}%` }} />
          </div>
        </section>
      ) : null}

      {scanConfirmed ? (
        <section className="scan-complete-panel">
          <h3 className="scan-complete-title">BATCH COMPLETE</h3>
          <p className="muted-text">
            {activeStrategy === "batch"
              ? "Scan complete. Your wallet now handles signing and broadcast. Continue to generate an audit receipt."
              : "All recipient QR requests were marked scanned. Continue to generate an audit receipt."}
          </p>
          <button type="button" className="btn btn-primary" onClick={onGenerateReceipt}>
            Generate Receipt &rarr;
          </button>
        </section>
      ) : (
        <button
          type="button"
          className="btn btn-scan-success"
          onClick={handleMarkScanned}
          disabled={activeStrategy === "split" && currentSplitMarked}
        >
          {activeStrategy === "batch"
            ? "I've Scanned Successfully"
            : currentSplitMarked
            ? "Recipient Already Marked"
            : "Mark Recipient Scanned"}
        </button>
      )}

      <div className="button-row">
        <button className="btn" type="button" onClick={onBack}>
          &larr; Back
        </button>
      </div>
    </section>
  );
}
