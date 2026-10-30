//! Projects: a folder with a note of the same name whose front matter says `type: project`.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde_json::{Map, Value, json};

use super::note::{create_note, parse_note, read_note};
use super::{Workspace, io, safe_filename, skipped};
use crate::{Error, Result};

#[derive(Debug, Clone, PartialEq)]
pub struct Project {
    pub name: String,
    pub folder: PathBuf,
    /// `Harness/Harness.md`.
    pub note: PathBuf,
    pub id: Option<String>,
    /// `owner/name`; in practice one.
    pub repos: Vec<String>,
    /// Linked root names this project works in.
    pub roots: Vec<String>,
}

/// A repo claimed by two projects. Both projects are left out of the result.
#[derive(Debug, Clone, PartialEq)]
pub struct ProjectError {
    pub repo: String,
    /// The folders of the projects that claim it.
    pub projects: Vec<PathBuf>,
    pub message: String,
}

fn strings(props: &Map<String, Value>, key: &str) -> Vec<String> {
    match props.get(key) {
        Some(Value::String(s)) => vec![s.clone()],
        Some(Value::Array(items)) => items.iter().filter_map(|v| v.as_str().map(str::to_string)).collect(),
        _ => Vec::new(),
    }
}

/// The project whose note is `<folder>/<folder name>.md`, if there is one.
pub(crate) fn project_at(folder: &Path) -> Option<Project> {
    let name = folder.file_name()?.to_string_lossy().into_owned();
    let note = folder.join(format!("{name}.md"));
    let text = std::fs::read_to_string(&note).ok()?;
    let n = parse_note(&note, &text);
    if n.props.get("type").and_then(Value::as_str) != Some("project") {
        return None;
    }
    Some(Project {
        name,
        folder: folder.to_path_buf(),
        note,
        id: n.id,
        repos: strings(&n.props, "repos"),
        roots: strings(&n.props, "roots"),
    })
}

fn find(dir: &Path, out: &mut Vec<Project>) {
    let Ok(read) = std::fs::read_dir(dir) else { return };
    let mut dirs: Vec<PathBuf> = read
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()))
        .filter(|e| !skipped(&e.file_name().to_string_lossy()))
        .map(|e| e.path())
        .collect();
    dirs.sort();
    for d in dirs {
        out.extend(project_at(&d));
        find(&d, out);
    }
}

impl Workspace {
    /// Every project in the workspace, and the errors found. A repo listed by two projects is an
    /// error and both projects are dropped, since the repo's issues can't belong to either.
    pub fn projects(&self) -> (Vec<Project>, Vec<ProjectError>) {
        let mut all = Vec::new();
        find(self.root(), &mut all);
        let mut by_repo: BTreeMap<String, Vec<PathBuf>> = BTreeMap::new();
        for p in &all {
            let mut seen = Vec::new();
            for r in &p.repos {
                let key = r.to_lowercase();
                if !seen.contains(&key) {
                    by_repo.entry(key.clone()).or_default().push(p.folder.clone());
                    seen.push(key);
                }
            }
        }
        let mut errors = Vec::new();
        let mut dropped: Vec<&PathBuf> = Vec::new();
        for (repo, folders) in &by_repo {
            if folders.len() > 1 {
                let names: Vec<String> = folders.iter().map(|f| f.display().to_string()).collect();
                errors.push(ProjectError {
                    repo: repo.clone(),
                    projects: folders.clone(),
                    message: format!("{repo} is listed by more than one project: {}", names.join(", ")),
                });
                dropped.extend(folders);
            }
        }
        let kept = all.iter().filter(|p| !dropped.contains(&&p.folder)).cloned().collect();
        (kept, errors)
    }

    /// Makes `<workspace>/<name>/<name>.md` as a project, with `repo` (`owner/name`) listed if given.
    pub fn create_project(&self, name: &str, repo: Option<&str>) -> Result<Project> {
        let name = safe_filename(name);
        let folder = self.root().join(&name);
        if folder.exists() {
            return Err(Error::Other(format!("{} already exists", folder.display())));
        }
        io(std::fs::create_dir_all(&folder), &folder)?;
        let mut props = Map::new();
        props.insert("type".into(), json!("project"));
        if let Some(repo) = repo {
            props.insert("repos".into(), json!([repo]));
        }
        let note = create_note(&folder, &name, &props)?;
        let n = read_note(&note)?;
        Ok(Project {
            name,
            folder,
            note,
            id: n.id,
            repos: repo.map(str::to_string).into_iter().collect(),
            roots: Vec::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(ws: &Workspace, rel: &str, text: &str) {
        let p = ws.root().join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, text).unwrap();
    }

    #[test]
    fn finds_projects_anywhere_and_skips_hidden() {
        let dir = tempfile::tempdir().unwrap();
        let ws = Workspace::open(dir.path()).unwrap();
        write(&ws, "Harness/Harness.md", "---\ntype: project\nrepos: [neoworks-dev/harness]\nroots: [harness]\n---\n");
        write(&ws, "Deep/er/Site/Site.md", "---\ntype: project\nrepos: neoworks-dev/site\n---\n");
        write(&ws, "NotOne/NotOne.md", "---\ntype: note\n---\n");
        write(&ws, "Other/Different.md", "---\ntype: project\n---\n");
        write(&ws, ".hidden/Hid/Hid.md", "---\ntype: project\n---\n");
        let (projects, errors) = ws.projects();
        assert!(errors.is_empty());
        let names: Vec<_> = projects.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, ["Site", "Harness"], "in path order");
        assert_eq!(projects[1].repos, ["neoworks-dev/harness"]);
        assert_eq!(projects[1].roots, ["harness"]);
        assert_eq!(projects[0].repos, ["neoworks-dev/site"]);
    }

    #[test]
    fn a_repo_in_two_projects_is_an_error_and_drops_both() {
        let dir = tempfile::tempdir().unwrap();
        let ws = Workspace::open(dir.path()).unwrap();
        write(&ws, "A/A.md", "---\ntype: project\nrepos: [o/r]\n---\n");
        write(&ws, "B/B.md", "---\ntype: project\nrepos: [O/R]\n---\n");
        write(&ws, "C/C.md", "---\ntype: project\nrepos: [o/c]\n---\n");
        let (projects, errors) = ws.projects();
        assert_eq!(projects.len(), 1);
        assert_eq!(projects[0].name, "C");
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].repo, "o/r");
        assert_eq!(errors[0].projects.len(), 2);
    }

    #[test]
    fn create_project_makes_folder_and_note() {
        let dir = tempfile::tempdir().unwrap();
        let ws = Workspace::open(dir.path()).unwrap();
        let p = ws.create_project("Harness", Some("o/harness")).unwrap();
        assert!(p.note.ends_with("Harness/Harness.md"));
        assert!(p.id.is_some());
        let (projects, _) = ws.projects();
        assert_eq!(projects, vec![p]);
        assert!(ws.create_project("Harness", None).is_err());
        let q = ws.create_project("Loose", None).unwrap();
        assert!(q.repos.is_empty());
    }
}
