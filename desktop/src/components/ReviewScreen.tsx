import type { Network, QrStrategy, ValidateBatchResponse } from "../lib/types";
import { zatoshiToZec } from "../lib/zatoshi";

interface ReviewScreenProps {
  batch: ValidateBatchResponse;
  network: Network;
  loading: boolean;
  error: string | null;
  qrStrategy: QrStrategy;
  onQrStrategyChange: (strategy: QrStrategy) => void;
  onBack: () => void;
  onProceed: () => Promise<void>;
}

export function ReviewScreen({
  batch,
  network,
  loading,
  error,
  qrStrategy,
  onQrStrategyChange,
  onBack,
  onProceed
}: ReviewScreenProps) {
  const recipients = batch.validated_batch.recipients;

  const truncateAddress = (value: string): string => {
    if (value.length <= 20) {
      return value;
    }
    return `${value.slice(0, 10)}...${value.slice(-6)}`;
  };

  const truncateMemo = (value: string | null): string => {
    if (!value) {
      return "-";
    }
    if (value.length <= 20) {
      return value;
    }
    return `${value.slice(0, 20)}...`;
  };

  return (
    <section className="panel review-panel">
      <header>
        <p className="eyebrow">STEP 02</p>
        <h2 className="panel-title">REVIEW BATCH</h2>
      </header>

      <section className="summary-panel">
        <div>
          <p className="summary-label">RECIPIENTS</p>
          <p className="summary-value">{batch.summary.recipient_count}</p>
        </div>
        <div>
          <p className="summary-label">TOTAL ZEC</p>
          <p className="summary-value">{batch.summary.total_zec}</p>
        </div>
        <div>
          <p className="summary-label">NETWORK</p>
          <p className={`summary-value ${network === "mainnet" ? "network-mainnet" : "network-testnet"}`}>
            {network.toUpperCase()}
          </p>
        </div>
      </section>

      {batch.validated_batch.warnings.length > 0 ? (
        <div className="notice warning">
          <strong>WARNINGS</strong>
          <ul className="warning-list">
            {batch.validated_batch.warnings.map((warning) => (
              <li key={warning}>{warning}</li>
            ))}
          </ul>
        </div>
      ) : null}

      <section className="scan-mode-panel">
        <p className="summary-label">SCAN STRATEGY</p>
        <div className="strategy-toggle">
          <button
            type="button"
            className={`btn ${qrStrategy === "batch" ? "btn-primary" : ""}`}
            onClick={() => onQrStrategyChange("batch")}
          >
            Batch QR
          </button>
          <button
            type="button"
            className={`btn ${qrStrategy === "split" ? "btn-primary" : ""}`}
            onClick={() => onQrStrategyChange("split")}
          >
            Split by Recipient
          </button>
        </div>
        <p className="muted-text">
          Batch mode is faster when wallets support multi-recipient ZIP-321. Split mode creates one QR per CSV row for
          wider wallet compatibility.
        </p>
      </section>

      <div className="table-wrap review-table-wrap">
        <table className="review-table">
          <thead>
            <tr>
              <th>ROW#</th>
              <th>ADDRESS</th>
              <th>LABEL</th>
              <th>AMOUNT (ZEC)</th>
              <th>MEMO</th>
              <th>STATUS</th>
            </tr>
          </thead>
          <tbody>
            {recipients.map((entry) => (
              <tr key={`${entry.row_number}-${entry.recipient.address}`}>
                <td>{entry.row_number}</td>
                <td title={entry.recipient.address}>{truncateAddress(entry.recipient.address)}</td>
                <td>{entry.recipient.label ?? "-"}</td>
                <td>{zatoshiToZec(entry.recipient.amount)}</td>
                <td>{truncateMemo(entry.recipient.memo)}</td>
                <td className="status-ok">&#10003;</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      {error ? (
        <div className="error-panel">
          <p className="error-title">PROCESSING ERROR</p>
          <p className="error-message">{error}</p>
        </div>
      ) : null}

      <div className="button-row">
        <button className="btn" type="button" onClick={onBack}>
          &larr; Back
        </button>
        <button className="btn btn-primary" type="button" disabled={loading} onClick={() => void onProceed()}>
          {loading ? "PROCESSING..." : "Construct Payment Request \u2192"}
        </button>
      </div>
    </section>
  );
}
