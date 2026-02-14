import { useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { save } from "@tauri-apps/plugin-dialog";

import type { Receipt } from "../lib/types";

interface ReceiptScreenProps {
  receipt: Receipt;
  onNewBatch: () => void;
}

function defaultReceiptFilename(receipt: Receipt): string {
  const date = receipt.timestamp.slice(0, 10);
  const id8 = receipt.batch_id.slice(0, 8);
  return `laminar-receipt-${date}-${id8}.json`;
}

export function ReceiptScreen({ receipt, onNewBatch }: ReceiptScreenProps) {
  const [saved, setSaved] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const receiptJson = useMemo(() => JSON.stringify(receipt, null, 2), [receipt]);

  const handleSave = async () => {
    setSaved(false);
    setError(null);

    const filePath = await save({
      defaultPath: defaultReceiptFilename(receipt),
      filters: [{ name: "JSON", extensions: ["json"] }]
    });

    if (!filePath) {
      return;
    }

    try {
      await invoke("save_receipt", {
        receiptJson: receiptJson,
        filePath: filePath
      });
      setSaved(true);
    } catch (err) {
      setError(String(err));
    }
  };

  return (
    <section className="panel receipt-panel">
      <header>
        <p className="eyebrow">STEP 04</p>
        <h2 className="panel-title">RECEIPT</h2>
      </header>

      <section className="receipt-grid">
        <div className="receipt-cell">
          <p className="summary-label">BATCH ID</p>
          <p className="receipt-value">{receipt.batch_id}</p>
        </div>
        <div className="receipt-cell">
          <p className="summary-label">TIMESTAMP</p>
          <p className="receipt-value">{receipt.timestamp}</p>
        </div>
        <div className="receipt-cell">
          <p className="summary-label">NETWORK</p>
          <p className={`receipt-value ${receipt.network === "mainnet" ? "network-mainnet" : "network-testnet"}`}>
            {receipt.network.toUpperCase()}
          </p>
        </div>
        <div className="receipt-cell">
          <p className="summary-label">RECIPIENTS</p>
          <p className="receipt-value">{receipt.recipient_count}</p>
        </div>
        <div className="receipt-cell">
          <p className="summary-label">TOTAL ZEC</p>
          <p className="receipt-value">{receipt.total_zec}</p>
        </div>
        <div className="receipt-cell">
          <p className="summary-label">SEGMENTS</p>
          <p className="receipt-value">{receipt.segments}</p>
        </div>
        <div className="receipt-cell receipt-cell-full">
          <p className="summary-label">PAYLOAD HASH</p>
          <p className="receipt-value hash-value">{receipt.zip321_payload_hash}</p>
        </div>
      </section>

      <label className="field-block">
        <span className="label">RECEIPT JSON</span>
        <textarea className="receipt-json" readOnly value={receiptJson} />
      </label>

      {saved ? <p className="saved-text">&#10003; Receipt Saved</p> : null}
      {error ? (
        <div className="error-panel">
          <p className="error-title">SAVE ERROR</p>
          <p className="error-message">{error}</p>
        </div>
      ) : null}

      <div className="button-row">
        <button className="btn btn-primary" type="button" onClick={() => void handleSave()}>
          Save Receipt
        </button>
        <button className="btn" type="button" onClick={onNewBatch}>
          New Batch
        </button>
      </div>
    </section>
  );
}
