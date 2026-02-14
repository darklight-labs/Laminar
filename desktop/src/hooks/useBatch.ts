import { useSyncExternalStore } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open, save } from "@tauri-apps/plugin-dialog";

import {
  isSupportedBatchFile,
  parseConstructBatchResponse,
  parseGenerateQrResponse,
  parseValidateBatchResponse
} from "../lib/validation";
import type {
  GenerateQrResponse
} from "../lib/types";
import { batchStore } from "../stores/batchStore";

async function pickFilePath(): Promise<string | null> {
  const result = await open({
    multiple: false,
    directory: false,
    filters: [
      {
        name: "Batch Files",
        extensions: ["csv", "json"]
      }
    ]
  });

  if (typeof result === "string") {
    return result;
  }
  return null;
}

export function useBatch() {
  const state = useSyncExternalStore(
    batchStore.subscribe,
    batchStore.getState,
    batchStore.getState
  );

  async function browseFile() {
    const selectedPath = await pickFilePath();
    if (selectedPath) {
      batchStore.setState({ filePath: selectedPath, error: null });
    }
  }

  function setNetwork(network: "mainnet" | "testnet") {
    batchStore.setState({ network, error: null });
  }

  function setFilePath(filePath: string) {
    batchStore.setState({ filePath, error: null });
  }

  async function validateCurrentBatch() {
    const current = batchStore.getState();
    if (!current.filePath) {
      batchStore.setState({ error: "Choose a CSV or JSON batch file first." });
      return;
    }
    if (!isSupportedBatchFile(current.filePath)) {
      batchStore.setState({ error: "Unsupported file type. Use .csv or .json." });
      return;
    }

    batchStore.setState({ loading: true, error: null, info: "Validating batch..." });

    try {
      const rawValidation = await invoke("validate_batch", {
        filePath: current.filePath,
        network: current.network
      });
      const validation = parseValidateBatchResponse(rawValidation);

      batchStore.setState({
        validation,
        generated: null,
        receiptJson: "",
        step: "review",
        loading: false,
        info: "Batch validated."
      });
    } catch (error) {
      batchStore.setState({
        loading: false,
        error: String(error),
        info: null
      });
    }
  }

  async function constructCurrentBatch() {
    const current = batchStore.getState();

    batchStore.setState({ loading: true, error: null, info: "Constructing ZIP-321 intent..." });

    try {
      const rawConstruct = await invoke("construct_batch", {
        filePath: current.filePath,
        network: current.network
      });
      parseConstructBatchResponse(rawConstruct);
      batchStore.setState({ loading: false, info: "Intent construction complete." });
    } catch (error) {
      batchStore.setState({ loading: false, error: String(error), info: null });
    }
  }

  async function generateQrArtifacts() {
    const current = batchStore.getState();
    if (!current.filePath) {
      batchStore.setState({ error: "No input file selected." });
      return;
    }

    batchStore.setState({ loading: true, error: null, info: "Generating QR artifacts..." });

    try {
      const rawGenerated = await invoke("generate_qr", {
        filePath: current.filePath,
        network: current.network
      });
      const generated: GenerateQrResponse = parseGenerateQrResponse(rawGenerated);
      const receiptJson = JSON.stringify(generated.receipt, null, 2);

      batchStore.setState({
        generated,
        receiptJson,
        step: "qr",
        loading: false,
        info: "QR artifacts generated."
      });
    } catch (error) {
      batchStore.setState({ loading: false, error: String(error), info: null });
    }
  }

  function goToReceipt() {
    batchStore.setState({ step: "receipt", error: null });
  }

  function backToReview() {
    batchStore.setState({ step: "review", error: null });
  }

  function backToImport() {
    batchStore.setState({ step: "import", error: null });
  }

  function updateReceiptJson(receiptJson: string) {
    batchStore.setState({ receiptJson });
  }

  async function saveReceiptFile() {
    const current = batchStore.getState();
    const receiptJson =
      current.receiptJson || (current.generated ? JSON.stringify(current.generated.receipt, null, 2) : "");

    if (!receiptJson) {
      batchStore.setState({ error: "No receipt content to save." });
      return;
    }

    const defaultName = current.generated
      ? `laminar-receipt-${current.generated.receipt.timestamp.slice(0, 10)}-${current.generated.receipt.batch_id.slice(
          0,
          8
        )}.json`
      : "laminar-receipt.json";

    const savePath = await save({
      defaultPath: defaultName,
      filters: [{ name: "JSON", extensions: ["json"] }]
    });

    if (!savePath) {
      return;
    }

    try {
      await invoke("save_receipt", {
        receiptJson: receiptJson,
        filePath: savePath
      });
      batchStore.setState({ info: `Receipt saved: ${savePath}`, error: null });
    } catch (error) {
      batchStore.setState({ error: String(error), info: null });
    }
  }

  function reset() {
    batchStore.reset();
  }

  return {
    state,
    actions: {
      browseFile,
      setNetwork,
      setFilePath,
      validateCurrentBatch,
      constructCurrentBatch,
      generateQrArtifacts,
      goToReceipt,
      backToReview,
      backToImport,
      updateReceiptJson,
      saveReceiptFile,
      reset
    }
  };
}
