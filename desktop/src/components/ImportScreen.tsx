import { type DragEvent, useEffect, useState } from "react";
import { invoke, isTauri } from "@tauri-apps/api/core";
import { save } from "@tauri-apps/plugin-dialog";
import { getCurrentWebview } from "@tauri-apps/api/webview";

import type { Contact, DraftLoadedResponse, DraftSummary, Network } from "../lib/types";
import {
  parseBoolean,
  parseContacts,
  parseDraftLoadedResponse,
  parseDraftSummaries
} from "../lib/validation";

interface ImportScreenProps {
  filePath: string;
  network: Network;
  loading: boolean;
  error: string | null;
  onFilePathChange: (value: string) => void;
  onPickFile: () => Promise<void>;
  onValidate: (pathOverride?: string) => Promise<void>;
  onDraftLoaded: (draft: DraftLoadedResponse) => void;
}

export function ImportScreen({
  filePath,
  network,
  loading,
  error,
  onFilePathChange,
  onPickFile,
  onValidate,
  onDraftLoaded
}: ImportScreenProps) {
  const [isHovering, setIsHovering] = useState(false);
  const [passphrase, setPassphrase] = useState("");
  const [storageUnlocked, setStorageUnlocked] = useState(false);
  const [storageError, setStorageError] = useState<string | null>(null);
  const [storageInfo, setStorageInfo] = useState<string | null>(null);

  const [contacts, setContacts] = useState<Contact[]>([]);
  const [contactIdEditing, setContactIdEditing] = useState<string | null>(null);
  const [contactAddress, setContactAddress] = useState("");
  const [contactLabel, setContactLabel] = useState("");
  const [contactNotes, setContactNotes] = useState("");

  const [drafts, setDrafts] = useState<DraftSummary[]>([]);
  const [draftName, setDraftName] = useState("");

  const refreshDrafts = async () => {
    try {
      const list = await invoke("list_drafts");
      setDrafts(parseDraftSummaries(list));
    } catch (err) {
      setStorageError(String(err));
    }
  };

  const refreshContacts = async () => {
    try {
      const list = await invoke("list_contacts");
      setContacts(parseContacts(list));
    } catch (err) {
      setStorageError(String(err));
    }
  };

  useEffect(() => {
    if (!isTauri()) {
      return;
    }

    let mounted = true;
    void invoke("init_local_storage").catch(() => null);
    void invoke("is_storage_unlocked")
      .then((value) => {
        if (!mounted) {
          return;
        }
        const unlocked = parseBoolean(value);
        setStorageUnlocked(unlocked);
        if (unlocked) {
          void refreshContacts();
        }
      })
      .catch(() => null);
    void refreshDrafts();

    return () => {
      mounted = false;
    };
  }, []);

  useEffect(() => {
    if (!isTauri()) {
      return;
    }

    let mounted = true;
    const unlistenPromise = getCurrentWebview().onDragDropEvent((event) => {
      if (!mounted) {
        return;
      }

      if (event.payload.type === "enter" || event.payload.type === "over") {
        setIsHovering(true);
        return;
      }

      if (event.payload.type === "leave") {
        setIsHovering(false);
        return;
      }

      if (event.payload.type === "drop") {
        setIsHovering(false);
        const path = event.payload.paths[0];
        if (path) {
          onFilePathChange(path);
          void onValidate(path);
        }
      }
    });

    return () => {
      mounted = false;
      void unlistenPromise.then((unlisten) => unlisten());
    };
  }, [onFilePathChange, onValidate]);

  const handleDomDrop = (event: DragEvent<HTMLButtonElement>) => {
    event.preventDefault();
    setIsHovering(false);

    const droppedFile = event.dataTransfer.files[0] as (File & { path?: string }) | undefined;
    const filePath = droppedFile?.path;
    if (filePath) {
      onFilePathChange(filePath);
      void onValidate(filePath);
      return;
    }

    const uriList = event.dataTransfer.getData("text/uri-list");
    if (uriList) {
      const firstUri = uriList.split(/\r?\n/).find((entry) => entry.trim().length > 0);
      if (firstUri) {
        onFilePathChange(firstUri);
        void onValidate(firstUri);
      }
    }
  };

  const handleUnlock = async () => {
    setStorageError(null);
    setStorageInfo(null);

    if (!passphrase.trim()) {
      setStorageError("Enter a passphrase first.");
      return;
    }

    try {
      await invoke("set_storage_passphrase", { passphrase });
      setStorageUnlocked(true);
      setPassphrase("");
      setStorageInfo("Storage unlocked.");
      await refreshContacts();
      await refreshDrafts();
    } catch (err) {
      setStorageError(String(err));
    }
  };

  const handleLock = async () => {
    setStorageError(null);
    setStorageInfo(null);
    try {
      await invoke("clear_sensitive_memory");
      setStorageUnlocked(false);
      setContacts([]);
      setStorageInfo("Sensitive memory cleared.");
    } catch (err) {
      setStorageError(String(err));
    }
  };

  const handleSaveContact = async () => {
    setStorageError(null);
    setStorageInfo(null);
    if (!storageUnlocked) {
      setStorageError("Unlock storage to manage encrypted contacts.");
      return;
    }
    if (!contactAddress.trim()) {
      setStorageError("Contact address is required.");
      return;
    }

    try {
      if (contactIdEditing) {
        await invoke("update_contact", {
          id: contactIdEditing,
          address: contactAddress,
          label: contactLabel,
          notes: contactNotes
        });
      } else {
        await invoke("create_contact", {
          address: contactAddress,
          label: contactLabel,
          notes: contactNotes
        });
      }

      setContactIdEditing(null);
      setContactAddress("");
      setContactLabel("");
      setContactNotes("");
      setStorageInfo("Contact saved.");
      await refreshContacts();
    } catch (err) {
      setStorageError(String(err));
    }
  };

  const handleDeleteContact = async (id: string) => {
    setStorageError(null);
    setStorageInfo(null);
    try {
      await invoke("delete_contact", { id });
      setStorageInfo("Contact deleted.");
      await refreshContacts();
    } catch (err) {
      setStorageError(String(err));
    }
  };

  const handleSaveDraft = async () => {
    setStorageError(null);
    setStorageInfo(null);
    if (!storageUnlocked) {
      setStorageError("Unlock storage to save encrypted drafts.");
      return;
    }
    if (!filePath.trim()) {
      setStorageError("Select a batch file before saving a draft.");
      return;
    }
    if (!draftName.trim()) {
      setStorageError("Draft name is required.");
      return;
    }

    try {
      await invoke("save_draft", {
        name: draftName,
        filePath: filePath,
        network
      });
      setDraftName("");
      setStorageInfo("Draft saved.");
      await refreshDrafts();
    } catch (err) {
      setStorageError(String(err));
    }
  };

  const handleLoadDraft = async (id: string) => {
    setStorageError(null);
    setStorageInfo(null);
    try {
      const draft = await invoke("load_draft", { id });
      onDraftLoaded(parseDraftLoadedResponse(draft));
    } catch (err) {
      setStorageError(String(err));
    }
  };

  const handleDeleteDraft = async (id: string) => {
    setStorageError(null);
    setStorageInfo(null);
    try {
      await invoke("delete_draft", { id });
      setStorageInfo("Draft deleted.");
      await refreshDrafts();
    } catch (err) {
      setStorageError(String(err));
    }
  };

  const handleExport = async () => {
    setStorageError(null);
    setStorageInfo(null);
    if (!storageUnlocked) {
      setStorageError("Unlock storage before exporting plaintext data.");
      return;
    }

    try {
      const filePath = await save({
        defaultPath: "laminar-export.json",
        filters: [{ name: "JSON", extensions: ["json"] }]
      });
      if (!filePath) {
        return;
      }
      await invoke("export_plaintext_data_to_file", { filePath: filePath });
      setStorageInfo(`Exported plaintext data to ${filePath}`);
    } catch (err) {
      setStorageError(String(err));
    }
  };

  return (
    <section className="panel import-panel">
      <header>
        <p className="eyebrow">STEP 01</p>
        <h2 className="panel-title">IMPORT BATCH</h2>
      </header>

      <button
        type="button"
        className={`drop-zone ${isHovering ? "drop-zone-hover" : "drop-zone-default"}`}
        onDragOver={(event) => {
          event.preventDefault();
          setIsHovering(true);
        }}
        onDragLeave={() => setIsHovering(false)}
        onDrop={handleDomDrop}
        onClick={() => void onPickFile()}
      >
        <div className="drop-zone-content">
          <p className="drop-zone-title">DROP CSV/JSON HERE</p>
          <p className="drop-zone-subtitle">OR CLICK TO BROWSE</p>
          <div className="file-badges">
            <span className="file-badge">.CSV</span>
            <span className="file-badge">.JSON</span>
          </div>
        </div>
      </button>

      <label className="field-block">
        <span className="label">FILE PATH</span>
        <input
          className="input"
          placeholder="C:\\path\\to\\batch.csv"
          value={filePath}
          onChange={(event) => onFilePathChange(event.target.value)}
          onKeyDown={(event) => {
            if (event.key === "Enter") {
              event.preventDefault();
              void onValidate();
            }
          }}
        />
      </label>

      {loading ? <p className="processing-text">PROCESSING...</p> : null}

      {error ? (
        <div className="error-panel">
          <p className="error-title">VALIDATION ERROR</p>
          <p className="error-message">{error}</p>
        </div>
      ) : null}

      <div className="button-row">
        <button className="btn btn-primary" disabled={loading} type="button" onClick={() => void onValidate()}>
          VALIDATE BATCH &rarr;
        </button>
      </div>

      <section className="storage-panel">
        <header>
          <p className="eyebrow">ADDRESS BOOK + DRAFTS</p>
          <h3 className="panel-title">ENCRYPTED LOCAL STORAGE</h3>
        </header>

        <div className="lock-row">
          <input
            className="input small-input"
            type="password"
            placeholder="Storage passphrase"
            value={passphrase}
            onChange={(event) => setPassphrase(event.target.value)}
          />
          <button type="button" className="btn" onClick={() => void handleUnlock()}>
            Unlock
          </button>
          <button type="button" className="btn" onClick={() => void handleLock()}>
            Lock
          </button>
          <button type="button" className="btn" onClick={() => void handleExport()}>
            Export Plaintext
          </button>
        </div>

        {!storageUnlocked ? (
          <p className="muted-text">Without passphrase, encrypted labels/notes/recipients remain unreadable.</p>
        ) : null}

        {storageInfo ? <p className="saved-text">{storageInfo}</p> : null}
        {storageError ? (
          <div className="error-panel">
            <p className="error-title">STORAGE ERROR</p>
            <p className="error-message">{storageError}</p>
          </div>
        ) : null}

        <div className="storage-grid">
          <section className="storage-column">
            <h4 className="summary-label">CONTACTS</h4>
            <input
              className="input small-input"
              placeholder="Address"
              value={contactAddress}
              onChange={(event) => setContactAddress(event.target.value)}
            />
            <input
              className="input small-input"
              placeholder="Label (encrypted)"
              value={contactLabel}
              onChange={(event) => setContactLabel(event.target.value)}
            />
            <input
              className="input small-input"
              placeholder="Notes (encrypted)"
              value={contactNotes}
              onChange={(event) => setContactNotes(event.target.value)}
            />
            <div className="button-row">
              <button type="button" className="btn" onClick={() => void handleSaveContact()}>
                {contactIdEditing ? "Update Contact" : "Create Contact"}
              </button>
              {contactIdEditing ? (
                <button
                  type="button"
                  className="btn"
                  onClick={() => {
                    setContactIdEditing(null);
                    setContactAddress("");
                    setContactLabel("");
                    setContactNotes("");
                  }}
                >
                  Cancel
                </button>
              ) : null}
            </div>

            <ul className="storage-list">
              {contacts.map((contact) => (
                <li key={contact.id} className="storage-item">
                  <div className="storage-item-title">{contact.address}</div>
                  <div className="muted-text">
                    {contact.label} - {contact.notes}
                  </div>
                  <div className="button-row">
                    <button
                      type="button"
                      className="btn tiny-btn"
                      onClick={() => {
                        setContactIdEditing(contact.id);
                        setContactAddress(contact.address);
                        setContactLabel(contact.label);
                        setContactNotes(contact.notes);
                      }}
                    >
                      Edit
                    </button>
                    <button
                      type="button"
                      className="btn tiny-btn"
                      onClick={() => void handleDeleteContact(contact.id)}
                    >
                      Delete
                    </button>
                  </div>
                </li>
              ))}
            </ul>
          </section>

          <section className="storage-column">
            <h4 className="summary-label">DRAFTS</h4>
            <input
              className="input small-input"
              placeholder="Draft name"
              value={draftName}
              onChange={(event) => setDraftName(event.target.value)}
            />
            <div className="button-row">
              <button type="button" className="btn" onClick={() => void handleSaveDraft()}>
                Save Draft
              </button>
              <button type="button" className="btn" onClick={() => void refreshDrafts()}>
                Refresh
              </button>
            </div>

            <ul className="storage-list">
              {drafts.map((draft) => (
                <li key={draft.id} className="storage-item">
                  <div className="storage-item-title">{draft.name}</div>
                  <div className="muted-text">
                    {draft.network.toUpperCase()} - {draft.recipient_count} recipient(s)
                  </div>
                  <div className="button-row">
                    <button type="button" className="btn tiny-btn" onClick={() => void handleLoadDraft(draft.id)}>
                      Load
                    </button>
                    <button type="button" className="btn tiny-btn" onClick={() => void handleDeleteDraft(draft.id)}>
                      Delete
                    </button>
                  </div>
                </li>
              ))}
            </ul>
          </section>
        </div>
      </section>
    </section>
  );
}
