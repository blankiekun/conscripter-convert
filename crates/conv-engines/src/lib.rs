//! Source-engine readers. Each submodule handles one engine's formats.

pub mod build;
pub mod labels;
pub mod mages;
pub mod media;
pub mod metrics;
pub mod nitroplus;
pub mod ogg;
pub mod output;
pub mod read;
pub mod registry;
pub mod xuse;

/// `dir`/`rel` (`/`-separated) with each part matched in any case, as
/// Windows does: a game copied from a disc may be `MURAMASA.EXE` or
/// `hd/cdfiles`. A part found in no case is kept as written.
pub fn find(dir: &std::path::Path, rel: &str) -> std::path::PathBuf {
    rel.split('/').filter(|p| !p.is_empty()).fold(dir.to_path_buf(), |at, part| {
        if at.join(part).exists() {
            return at.join(part);
        }
        let lower = part.to_lowercase();
        std::fs::read_dir(&at)
            .ok()
            .and_then(|d| d.flatten().find(|e| e.file_name().to_string_lossy().to_lowercase() == lower))
            .map_or_else(|| at.join(part), |e| e.path())
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn find_matches_any_case() {
        let dir = std::env::temp_dir().join(format!("conv-find-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("HD/CdFiles")).unwrap();
        std::fs::write(dir.join("HD/CdFiles/0.CD"), b"").unwrap();
        assert_eq!(super::find(&dir, "hd/cdfiles/0.cd"), dir.join("HD/CdFiles/0.CD"));
        assert_eq!(super::find(&dir, "hd/none/x"), dir.join("HD/none/x"));
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
