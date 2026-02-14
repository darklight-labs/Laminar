use std::fs;
use std::path::{Path, PathBuf};

use chrono::{SecondsFormat, Utc};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use zeroize::Zeroize;

use crate::encryption;

pub const DB_NAME: &str = "laminar";
pub const DB_VERSION: u32 = 1;
const DB_FILE_NAME: &str = "laminar-indexeddb-v1.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContactRecord {
    pub id: String,
    pub address: String,
    pub label_encrypted: String,
    pub notes_encrypted: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DraftRecord {
    pub id: String,
    pub name: String,
    pub recipients_encrypted: String,
    pub network: String,
    pub recipient_count: usize,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigStore {
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalDb {
    pub db_name: String,
    pub version: u32,
    pub contacts: Vec<ContactRecord>,
    pub drafts: Vec<DraftRecord>,
    pub config: ConfigStore,
}

impl Default for LocalDb {
    fn default() -> Self {
        Self {
            db_name: DB_NAME.to_string(),
            version: DB_VERSION,
            contacts: Vec::new(),
            drafts: Vec::new(),
            config: ConfigStore {
                updated_at: now_iso(),
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContactPlain {
    pub id: String,
    pub address: String,
    pub label: String,
    pub notes: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DraftSummary {
    pub id: String,
    pub name: String,
    pub network: String,
    pub recipient_count: usize,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DraftRecipient {
    pub row_number: usize,
    pub address_type: String,
    pub address: String,
    #[serde(
        deserialize_with = "deserialize_zatoshi_string",
        serialize_with = "serialize_zatoshi_string"
    )]
    pub amount_zatoshis: String,
    pub memo: Option<String>,
    pub label: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DraftLoaded {
    pub id: String,
    pub name: String,
    pub network: String,
    pub recipients: Vec<DraftRecipient>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaintextExport {
    pub db_name: String,
    pub version: u32,
    pub contacts: Vec<ContactPlain>,
    pub drafts: Vec<DraftLoaded>,
}

fn serialize_zatoshi_string<S>(value: &str, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(value)
}

fn deserialize_zatoshi_string<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum ZatoshiRepr {
        String(String),
        Number(u64),
    }

    let value = ZatoshiRepr::deserialize(deserializer)?;
    let normalized = match value {
        ZatoshiRepr::String(raw) => raw.trim().to_string(),
        ZatoshiRepr::Number(raw) => raw.to_string(),
    };

    if normalized.is_empty() {
        return Err(serde::de::Error::custom("amount_zatoshis cannot be empty"));
    }
    if normalized.len() > 1 && normalized.starts_with('0') {
        return Err(serde::de::Error::custom(
            "amount_zatoshis cannot contain leading zeros",
        ));
    }
    if !normalized.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(serde::de::Error::custom(
            "amount_zatoshis must be an unsigned integer string",
        ));
    }

    Ok(normalized)
}

fn now_iso() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true)
}

fn make_id(prefix: &str) -> String {
    format!(
        "{prefix}-{}",
        Utc::now().timestamp_nanos_opt().unwrap_or_default()
    )
}

fn db_path(base_dir: &Path) -> PathBuf {
    base_dir.join(DB_FILE_NAME)
}

fn load_db(base_dir: &Path) -> Result<LocalDb, String> {
    fs::create_dir_all(base_dir)
        .map_err(|err| format!("failed to create storage directory: {err}"))?;
    let path = db_path(base_dir);
    if !path.exists() {
        return Ok(LocalDb::default());
    }

    let content = fs::read_to_string(&path)
        .map_err(|err| format!("failed to read local storage database: {err}"))?;
    serde_json::from_str::<LocalDb>(&content)
        .map_err(|err| format!("failed to parse local storage database: {err}"))
}

fn save_db(base_dir: &Path, db: &LocalDb) -> Result<(), String> {
    fs::create_dir_all(base_dir)
        .map_err(|err| format!("failed to create storage directory: {err}"))?;
    let path = db_path(base_dir);
    let content = serde_json::to_string_pretty(db)
        .map_err(|err| format!("failed to serialize local storage database: {err}"))?;
    fs::write(path, content)
        .map_err(|err| format!("failed to persist local storage database: {err}"))
}

fn decrypt_contact(record: &ContactRecord) -> Result<ContactPlain, String> {
    let label = encryption::decrypt_string(&record.label_encrypted)?;
    let notes = encryption::decrypt_string(&record.notes_encrypted)?;
    Ok(ContactPlain {
        id: record.id.clone(),
        address: record.address.clone(),
        label,
        notes,
        created_at: record.created_at.clone(),
        updated_at: record.updated_at.clone(),
    })
}

fn decrypt_draft(record: &DraftRecord) -> Result<DraftLoaded, String> {
    let mut decrypted = encryption::decrypt_string(&record.recipients_encrypted)?;
    let recipients = serde_json::from_str::<Vec<DraftRecipient>>(&decrypted)
        .map_err(|err| format!("failed to parse decrypted draft recipients: {err}"))?;
    decrypted.zeroize();
    Ok(DraftLoaded {
        id: record.id.clone(),
        name: record.name.clone(),
        network: record.network.clone(),
        recipients,
        created_at: record.created_at.clone(),
        updated_at: record.updated_at.clone(),
    })
}

pub fn init_db(base_dir: &Path) -> Result<(), String> {
    let db = load_db(base_dir)?;
    save_db(base_dir, &db)
}

pub fn create_contact(
    base_dir: &Path,
    address: String,
    mut label: String,
    mut notes: String,
) -> Result<ContactPlain, String> {
    let mut db = load_db(base_dir)?;
    let now = now_iso();
    let label_encrypted = encryption::encrypt_string(&label)?;
    let notes_encrypted = encryption::encrypt_string(&notes)?;
    label.zeroize();
    notes.zeroize();

    let record = ContactRecord {
        id: make_id("contact"),
        address,
        label_encrypted,
        notes_encrypted,
        created_at: now.clone(),
        updated_at: now,
    };

    let output = decrypt_contact(&record)?;
    db.contacts.push(record);
    db.config.updated_at = now_iso();
    save_db(base_dir, &db)?;
    Ok(output)
}

pub fn list_contacts(base_dir: &Path) -> Result<Vec<ContactPlain>, String> {
    let db = load_db(base_dir)?;
    db.contacts.iter().map(decrypt_contact).collect()
}

pub fn update_contact(
    base_dir: &Path,
    id: &str,
    address: Option<String>,
    mut label: Option<String>,
    mut notes: Option<String>,
) -> Result<ContactPlain, String> {
    let mut db = load_db(base_dir)?;
    let Some(record) = db.contacts.iter_mut().find(|entry| entry.id == id) else {
        return Err(format!("contact '{id}' not found"));
    };

    if let Some(next_address) = address {
        record.address = next_address;
    }
    if let Some(next_label) = label.as_deref() {
        record.label_encrypted = encryption::encrypt_string(next_label)?;
    }
    if let Some(next_notes) = notes.as_deref() {
        record.notes_encrypted = encryption::encrypt_string(next_notes)?;
    }
    if let Some(value) = label.as_mut() {
        value.zeroize();
    }
    if let Some(value) = notes.as_mut() {
        value.zeroize();
    }

    record.updated_at = now_iso();
    db.config.updated_at = now_iso();
    let output = decrypt_contact(record)?;
    save_db(base_dir, &db)?;
    Ok(output)
}

pub fn delete_contact(base_dir: &Path, id: &str) -> Result<(), String> {
    let mut db = load_db(base_dir)?;
    let before = db.contacts.len();
    db.contacts.retain(|entry| entry.id != id);
    if db.contacts.len() == before {
        return Err(format!("contact '{id}' not found"));
    }
    db.config.updated_at = now_iso();
    save_db(base_dir, &db)
}

pub fn save_draft(
    base_dir: &Path,
    name: String,
    network: String,
    recipients: &[DraftRecipient],
) -> Result<DraftSummary, String> {
    let mut db = load_db(base_dir)?;
    let mut recipients_json = serde_json::to_string(recipients)
        .map_err(|err| format!("failed to serialize draft recipients: {err}"))?;
    let recipients_encrypted = encryption::encrypt_string(&recipients_json)?;
    recipients_json.zeroize();

    let now = now_iso();
    let record = DraftRecord {
        id: make_id("draft"),
        name: name.clone(),
        recipients_encrypted,
        network: network.clone(),
        recipient_count: recipients.len(),
        created_at: now.clone(),
        updated_at: now.clone(),
    };
    db.drafts.push(record.clone());
    db.config.updated_at = now_iso();
    save_db(base_dir, &db)?;

    Ok(DraftSummary {
        id: record.id,
        name,
        network,
        recipient_count: recipients.len(),
        created_at: now.clone(),
        updated_at: now,
    })
}

pub fn list_drafts(base_dir: &Path) -> Result<Vec<DraftSummary>, String> {
    let db = load_db(base_dir)?;
    Ok(db
        .drafts
        .iter()
        .map(|entry| DraftSummary {
            id: entry.id.clone(),
            name: entry.name.clone(),
            network: entry.network.clone(),
            recipient_count: entry.recipient_count,
            created_at: entry.created_at.clone(),
            updated_at: entry.updated_at.clone(),
        })
        .collect())
}

pub fn load_draft(base_dir: &Path, id: &str) -> Result<DraftLoaded, String> {
    let db = load_db(base_dir)?;
    let Some(record) = db.drafts.iter().find(|entry| entry.id == id) else {
        return Err(format!("draft '{id}' not found"));
    };
    decrypt_draft(record)
}

pub fn delete_draft(base_dir: &Path, id: &str) -> Result<(), String> {
    let mut db = load_db(base_dir)?;
    let before = db.drafts.len();
    db.drafts.retain(|entry| entry.id != id);
    if db.drafts.len() == before {
        return Err(format!("draft '{id}' not found"));
    }
    db.config.updated_at = now_iso();
    save_db(base_dir, &db)
}

pub fn export_plaintext(base_dir: &Path) -> Result<PlaintextExport, String> {
    let db = load_db(base_dir)?;
    let contacts = db
        .contacts
        .iter()
        .map(decrypt_contact)
        .collect::<Result<Vec<_>, _>>()?;
    let drafts = db
        .drafts
        .iter()
        .map(decrypt_draft)
        .collect::<Result<Vec<_>, _>>()?;

    Ok(PlaintextExport {
        db_name: db.db_name,
        version: db.version,
        contacts,
        drafts,
    })
}

#[cfg(test)]
mod tests {
    use std::env;
    use std::path::PathBuf;

    use super::{
        create_contact, export_plaintext, list_contacts, load_draft, save_draft, DraftRecipient,
        DB_NAME,
    };
    use crate::encryption;

    fn temp_storage_dir(label: &str) -> PathBuf {
        let dir = env::temp_dir().join(format!(
            "laminar-storage-test-{label}-{}",
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn contact_roundtrip_requires_passphrase() {
        let _guard = encryption::test_key_lock().lock().unwrap();
        let dir = temp_storage_dir("contact");
        encryption::clear_session_key();
        encryption::set_passphrase("test-password".to_string()).unwrap();

        let created = create_contact(
            &dir,
            "t1Hsc1LR8yKnbbe3twRp88p6vFfC5t7DLbs".to_string(),
            "Alice".to_string(),
            "Payroll".to_string(),
        )
        .unwrap();
        assert_eq!(created.label, "Alice");

        let list = list_contacts(&dir).unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].notes, "Payroll");

        encryption::clear_session_key();
        let unreadable = list_contacts(&dir);
        assert!(unreadable.is_err());
    }

    #[test]
    fn draft_roundtrip_and_export() {
        let _guard = encryption::test_key_lock().lock().unwrap();
        let dir = temp_storage_dir("draft");
        encryption::clear_session_key();
        encryption::set_passphrase("test-password".to_string()).unwrap();

        let recipients = vec![DraftRecipient {
            row_number: 1,
            address_type: "transparent".to_string(),
            address: "t1Hsc1LR8yKnbbe3twRp88p6vFfC5t7DLbs".to_string(),
            amount_zatoshis: "100000000".to_string(),
            memo: Some("memo".to_string()),
            label: Some("label".to_string()),
        }];
        let draft = save_draft(
            &dir,
            "Payroll".to_string(),
            "mainnet".to_string(),
            &recipients,
        )
        .unwrap();

        // Simulate app restart: in-memory key is cleared, then unlocked again.
        encryption::clear_session_key();
        encryption::set_passphrase("test-password".to_string()).unwrap();
        let loaded = load_draft(&dir, &draft.id).unwrap();
        assert_eq!(loaded.recipients.len(), 1);
        assert_eq!(loaded.recipients[0].amount_zatoshis, "100000000");

        // Without passphrase, encrypted fields are unreadable.
        encryption::clear_session_key();
        assert!(load_draft(&dir, &draft.id).is_err());
        encryption::set_passphrase("test-password".to_string()).unwrap();
        let exported = export_plaintext(&dir).unwrap();
        assert_eq!(exported.contacts.len(), 0);
        assert_eq!(exported.drafts.len(), 1);
        let exported_json = serde_json::to_string_pretty(&exported).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&exported_json).unwrap();
        assert_eq!(parsed["db_name"], DB_NAME);
    }
}
