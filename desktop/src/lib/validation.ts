import { z } from "zod";

import type {
  Contact,
  ConstructBatchResponse,
  DraftLoadedResponse,
  DraftSummary,
  GenerateQrResponse,
  Network,
  ValidateBatchResponse
} from "./types";

const API_SCHEMA_VERSION = "1.0";

const zNetwork = z.enum(["mainnet", "testnet"]);
const zZatoshi = z
  .string()
  .regex(/^(0|[1-9]\d*)$/, "zatoshi values must be unsigned integer strings");
const zNullableString = z.string().nullable();

const zRecipient = z.object({
  address: z.string().min(1),
  amount: zZatoshi,
  memo: zNullableString,
  label: zNullableString
});

const zValidatedRecipient = z.object({
  row_number: z.number().int().positive(),
  address_type: z.enum(["unified", "sapling", "transparent"]),
  recipient: zRecipient
});

const zValidatedBatch = z.object({
  recipients: z.array(zValidatedRecipient),
  total: zZatoshi,
  network: zNetwork,
  warnings: z.array(z.string())
});

const zTransactionIntent = z.object({
  schema_version: z.literal(API_SCHEMA_VERSION),
  id: z.string().uuid(),
  created_at: z.string().datetime(),
  network: zNetwork,
  recipients: z.array(zRecipient),
  total_zat: zZatoshi,
  zip321_uri: z.string(),
  payload_bytes: z.number().int().nonnegative(),
  payload_hash: z.string()
});

const zQrFrame = z.object({
  index: z.number().int().nonnegative(),
  png_bytes: z.array(z.number().int().nonnegative().max(255)),
  data: z.string()
});

const zQrOutput = z.object({
  mode: z.enum(["Static", "AnimatedUr"]),
  frames: z.array(zQrFrame),
  total_frames: z.number().int().positive(),
  payload_bytes: z.number().int().nonnegative()
});

const zSplitQrOutput = z.object({
  index: z.number().int().nonnegative(),
  row_number: z.number().int().positive(),
  address_type: z.enum(["unified", "sapling", "transparent"]),
  address: z.string(),
  amount_zatoshis: zZatoshi,
  amount_zec: z.string(),
  memo: zNullableString,
  label: zNullableString,
  transaction_intent: zTransactionIntent,
  qr_output: zQrOutput
});

const zReceiptRecipient = z.object({
  address: z.string(),
  amount_zatoshis: zZatoshi,
  amount_zec: z.string(),
  memo: zNullableString,
  label: zNullableString
});

const zReceipt = z.object({
  laminar_version: z.string(),
  timestamp: z.string().datetime(),
  batch_id: z.string().uuid(),
  network: zNetwork,
  total_zatoshis: zZatoshi,
  total_zec: z.string(),
  recipient_count: z.number().int().nonnegative(),
  recipients: z.array(zReceiptRecipient),
  zip321_payload_hash: z.string().startsWith("sha256:"),
  segments: z.number().int().positive()
});

const zValidateBatchResponse = z.object({
  validated_batch: zValidatedBatch,
  summary: z.object({
    network: zNetwork,
    recipient_count: z.number().int().nonnegative(),
    total_zatoshis: zZatoshi,
    total_zec: z.string()
  })
});

const zConstructBatchResponse = z.object({
  validated_batch: zValidatedBatch,
  transaction_intent: zTransactionIntent
});

const zGenerateQrResponse = z.object({
  transaction_intent: zTransactionIntent,
  qr_output: zQrOutput,
  split_qr_outputs: z.array(zSplitQrOutput).default([]),
  receipt: zReceipt
});

const zContact = z.object({
  id: z.string().min(1),
  address: z.string().min(1),
  label: z.string(),
  notes: z.string(),
  created_at: z.string().datetime(),
  updated_at: z.string().datetime()
});

const zDraftSummary = z.object({
  id: z.string().min(1),
  name: z.string().min(1),
  network: zNetwork,
  recipient_count: z.number().int().nonnegative(),
  created_at: z.string().datetime(),
  updated_at: z.string().datetime()
});

const zDraftLoadedResponse = z.object({
  draft: z.object({
    id: z.string().min(1),
    name: z.string().min(1),
    network: zNetwork,
    created_at: z.string().datetime(),
    updated_at: z.string().datetime()
  }),
  validated_batch: zValidatedBatch,
  summary: z.object({
    network: zNetwork,
    recipient_count: z.number().int().nonnegative(),
    total_zatoshis: zZatoshi,
    total_zec: z.string()
  })
});
const zBoolean = z.boolean();

export function isNetwork(value: string): value is Network {
  return value === "mainnet" || value === "testnet";
}

export function isSupportedBatchFile(filePath: string): boolean {
  const lower = filePath.toLowerCase();
  return lower.endsWith(".csv") || lower.endsWith(".json");
}

export function truncateAddress(address: string): string {
  if (address.length <= 18) {
    return address;
  }
  return `${address.slice(0, 9)}...${address.slice(-7)}`;
}

export function parseValidateBatchResponse(value: unknown): ValidateBatchResponse {
  return zValidateBatchResponse.parse(value);
}

export function parseConstructBatchResponse(value: unknown): ConstructBatchResponse {
  return zConstructBatchResponse.parse(value);
}

export function parseGenerateQrResponse(value: unknown): GenerateQrResponse {
  return zGenerateQrResponse.parse(value);
}

export function parseContacts(value: unknown): Contact[] {
  return z.array(zContact).parse(value);
}

export function parseDraftSummaries(value: unknown): DraftSummary[] {
  return z.array(zDraftSummary).parse(value);
}

export function parseDraftLoadedResponse(value: unknown): DraftLoadedResponse {
  return zDraftLoadedResponse.parse(value);
}

export function parseBoolean(value: unknown): boolean {
  return zBoolean.parse(value);
}
