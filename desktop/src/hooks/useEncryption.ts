import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";

export function useEncryption() {
  const [plaintext, setPlaintext] = useState("");
  const [ciphertext, setCiphertext] = useState("");
  const [error, setError] = useState<string | null>(null);

  async function encrypt() {
    setError(null);
    try {
      const encrypted = await invoke<string>("encrypt_field", { plaintext });
      setCiphertext(encrypted);
    } catch (err) {
      setError(String(err));
    }
  }

  async function decrypt() {
    setError(null);
    try {
      const decrypted = await invoke<string>("decrypt_field", { data: ciphertext });
      setPlaintext(decrypted);
    } catch (err) {
      setError(String(err));
    }
  }

  return {
    plaintext,
    ciphertext,
    error,
    setPlaintext,
    setCiphertext,
    encrypt,
    decrypt
  };
}
