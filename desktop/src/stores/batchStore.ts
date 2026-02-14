import type { GenerateQrResponse, ValidateBatchResponse } from "../lib/types";

export type BatchStep = "import" | "review" | "qr" | "receipt";

export interface BatchStoreState {
  step: BatchStep;
  filePath: string;
  network: "mainnet" | "testnet";
  loading: boolean;
  error: string | null;
  validation: ValidateBatchResponse | null;
  generated: GenerateQrResponse | null;
  receiptJson: string;
  info: string | null;
}

const initialState: BatchStoreState = {
  step: "import",
  filePath: "",
  network: "mainnet",
  loading: false,
  error: null,
  validation: null,
  generated: null,
  receiptJson: "",
  info: null
};

let state: BatchStoreState = { ...initialState };
const listeners = new Set<() => void>();

function emit() {
  listeners.forEach((listener) => listener());
}

export const batchStore = {
  getState(): BatchStoreState {
    return state;
  },

  setState(patch: Partial<BatchStoreState>) {
    state = { ...state, ...patch };
    emit();
  },

  reset() {
    state = { ...initialState };
    emit();
  },

  subscribe(listener: () => void) {
    listeners.add(listener);
    return () => listeners.delete(listener);
  }
};
