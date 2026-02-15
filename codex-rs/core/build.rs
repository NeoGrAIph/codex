use std::fs;
use std::path::Path;

fn main() {
    let samples_dir = Path::new("src/skills/assets/samples");
    if samples_dir.exists() {
        println!("cargo:rerun-if-changed={}", samples_dir.display());
        visit_dir(samples_dir);
    }

    // FORK COMMIT OPEN [SA]: track agent templates as build dependencies.
    // Role: refresh embedded templates when `templates/agents/*.md` changes.
    let agents_dir = Path::new("templates/agents");
    if agents_dir.exists() {
        println!("cargo:rerun-if-changed={}", agents_dir.display());
        visit_dir(agents_dir);
    }
    // FORK COMMIT CLOSE: track agent templates as build dependencies.
}

fn visit_dir(dir: &Path) {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        println!("cargo:rerun-if-changed={}", path.display());
        if path.is_dir() {
            visit_dir(&path);
        }
    }
}
