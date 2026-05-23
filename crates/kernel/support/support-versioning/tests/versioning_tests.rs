use chrono::Utc;
use tempfile::tempdir;

use support_versioning::FileReleaseNotesStore;

#[test]
fn ensures_default_release_notes_file() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("release-notes.json");
    let store = FileReleaseNotesStore::new(&path);

    let notes = store.ensure_exists("0.1.0").unwrap();

    assert_eq!(notes.version, "0.1.0");
    assert!(notes.novidades.is_empty());
    assert!(notes.problemas_conhecidos.is_empty());
    assert!(path.exists());
}

#[test]
fn persists_version_novelties_and_known_issues() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("release-notes.json");
    let store = FileReleaseNotesStore::new(&path);

    store.ensure_exists("0.1.0").unwrap();
    store.set_version("0.2.0", Utc::now()).unwrap();
    store
        .add_novidade("Suporte a versionamento persistente", Utc::now())
        .unwrap();
    store
        .add_problema_conhecido("Ainda sem remoção de entradas", Utc::now())
        .unwrap();

    let notes = store.load().unwrap();
    assert_eq!(notes.version, "0.2.0");
    assert_eq!(notes.novidades.len(), 1);
    assert_eq!(notes.problemas_conhecidos.len(), 1);
}
