export type Network = "mainnet" | "testnet";
export type ZatoshiString = string;

export interface Recipient {
  address: string;
  amount: ZatoshiString;
  memo: string | null;
  label: string | null;
}

export type RecipientAddressType = "unified" | "sapling" | "transparent";

export interface ValidatedRecipient {
  row_number: number;
  address_type: RecipientAddressType;
  recipient: Recipient;
}

export interface ValidatedBatch {
  recipients: ValidatedRecipient[];
  total: ZatoshiString;
  network: Network;
  warnings: string[];
}

export interface TransactionIntent {
  schema_version: string;
  id: string;
  created_at: string;
  network: Network;
  recipients: Recipient[];
  total_zat: ZatoshiString;
  zip321_uri: string;
  payload_bytes: number;
  payload_hash: string;
}

export type QrMode = "Static" | "AnimatedUr";
export type QrStrategy = "batch" | "split";

export interface QrFrame {
  index: number;
  png_bytes: number[];
  data: string;
}

export interface QrOutput {
  mode: QrMode;
  frames: QrFrame[];
  total_frames: number;
  payload_bytes: number;
}

export interface SplitQrOutput {
  index: number;
  row_number: number;
  address_type: RecipientAddressType;
  address: string;
  amount_zatoshis: ZatoshiString;
  amount_zec: string;
  memo: string | null;
  label: string | null;
  transaction_intent: TransactionIntent;
  qr_output: QrOutput;
}

export interface ReceiptRecipient {
  address: string;
  amount_zatoshis: ZatoshiString;
  amount_zec: string;
  memo: string | null;
  label: string | null;
}

export interface Receipt {
  laminar_version: string;
  timestamp: string;
  batch_id: string;
  network: Network;
  total_zatoshis: ZatoshiString;
  total_zec: string;
  recipient_count: number;
  recipients: ReceiptRecipient[];
  zip321_payload_hash: string;
  segments: number;
}

export interface ValidateBatchResponse {
  validated_batch: ValidatedBatch;
  summary: {
    network: Network;
    recipient_count: number;
    total_zatoshis: ZatoshiString;
    total_zec: string;
  };
}

export interface ConstructBatchResponse {
  validated_batch: ValidatedBatch;
  transaction_intent: TransactionIntent;
}

export interface GenerateQrResponse {
  transaction_intent: TransactionIntent;
  qr_output: QrOutput;
  split_qr_outputs: SplitQrOutput[];
  receipt: Receipt;
}

export interface Contact {
  id: string;
  address: string;
  label: string;
  notes: string;
  created_at: string;
  updated_at: string;
}

export interface DraftSummary {
  id: string;
  name: string;
  network: Network;
  recipient_count: number;
  created_at: string;
  updated_at: string;
}

export interface DraftLoadedResponse {
  draft: {
    id: string;
    name: string;
    network: Network;
    created_at: string;
    updated_at: string;
  };
  validated_batch: ValidatedBatch;
  summary: {
    network: Network;
    recipient_count: number;
    total_zatoshis: ZatoshiString;
    total_zec: string;
  };
}
