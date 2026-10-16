//! The plain-text list of everything that ships inside the app. It is built from `app/src/licenses.json`, which
//! `npm run licenses` writes and which is compiled into the binary, so the notice travels with the app and nothing
//! is downloaded.

use std::io::Write;

use serde::Deserialize;

use crate::error::Result;

const FILE: &str = include_str!("../../src/licenses.json");

#[derive(Deserialize)]
struct Licenses {
    terms: Vec<Terms>,
    texts: Vec<Text>,
    packages: Vec<Package>,
}

#[derive(Deserialize)]
struct Terms {
    name: String,
    version: String,
    links: Vec<Link>,
}

#[derive(Deserialize)]
struct Link {
    label: String,
    url: String,
}

#[derive(Deserialize)]
struct Text {
    text: String,
}

#[derive(Deserialize)]
struct Package {
    name: String,
    version: String,
    license: String,
    repository: Option<String>,
    /// Indexes into `Licenses::texts`.
    texts: Vec<usize>,
}

/// Writes the list to `out`: terms of services first, then every package, then each licence text once.
pub fn export(mut out: impl Write, app_version: &str) -> Result<()> {
    let file: Licenses = serde_json::from_str(FILE)?;
    out.write_all(render(&file, app_version).as_bytes())?;
    Ok(())
}

fn render(file: &Licenses, app_version: &str) -> String {
    let mut out = format!(
        "Kosmotheka {app_version}\n\
         Licences of the third-party software that goes inside the app\n\n\
         Kosmotheka is free software under the AGPL-3.0-or-later licence. It is built with the work of the\n\
         libraries below, whose licences ask for this notice to travel with the app. This file was made by the\n\
         app itself, from the list it carries; nothing was downloaded.\n"
    );

    out.push_str("\n== Services with their own terms ==\n\n");
    out.push_str(
        "These are not licences you receive a copy of: they are services with terms of their own.\n",
    );
    for terms in &file.terms {
        out.push_str(&format!("\n{} {}\n", terms.name, terms.version));
        for link in &terms.links {
            out.push_str(&format!("  {}: {}\n", link.label, link.url));
        }
    }

    out.push_str(&format!("\n== Packages ({}) ==\n", file.packages.len()));
    for package in &file.packages {
        out.push_str(&format!("\n{} {}\n", package.name, package.version));
        out.push_str(&format!("  Licence: {}\n", package.license));
        if let Some(repository) = &package.repository {
            out.push_str(&format!("  Repository: {repository}\n"));
        }
    }

    out.push_str("\n== Licence texts ==\n");
    for (index, text) in file.texts.iter().enumerate() {
        out.push_str("\n----------------------------------------\nUsed by:\n");
        for package in file.packages.iter().filter(|p| p.texts.contains(&index)) {
            out.push_str(&format!("  {} {}\n", package.name, package.version));
        }
        out.push_str(&format!("\n{}\n", text.text.trim_end()));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn exported() -> String {
        let mut out = Vec::new();
        export(&mut out, "9.9.9").unwrap();
        String::from_utf8(out).unwrap()
    }

    #[test]
    fn the_file_lists_every_package_and_every_terms_entry() {
        let file: Licenses = serde_json::from_str(FILE).unwrap();
        let text = exported();
        assert!(text.starts_with("Kosmotheka 9.9.9\n"));
        for package in &file.packages {
            let block = format!(
                "\n{} {}\n  Licence: {}\n",
                package.name, package.version, package.license
            );
            assert!(text.contains(&block), "{} is missing", package.name);
        }
        for terms in &file.terms {
            assert!(text.contains(&terms.name));
            for link in &terms.links {
                assert!(text.contains(&link.url), "{} is missing", link.url);
            }
        }
        // The terms come before the packages.
        assert!(text.find("== Services").unwrap() < text.find("== Packages").unwrap());
    }

    #[test]
    fn each_licence_text_appears_once() {
        let file: Licenses = serde_json::from_str(FILE).unwrap();
        let text = exported();
        assert!(!file.texts.is_empty());
        // One block per text, in order; counting the words themselves would also count a text that another one contains.
        let section = text.split("== Licence texts ==").nth(1).unwrap();
        let blocks: Vec<&str> = section
            .split("\n----------------------------------------\n")
            .skip(1)
            .collect();
        assert_eq!(blocks.len(), file.texts.len());
        for (block, entry) in blocks.iter().zip(&file.texts) {
            assert!(block.starts_with("Used by:\n"));
            assert!(block.ends_with(&format!("\n{}\n", entry.text.trim_end())));
        }
    }

    #[test]
    fn every_text_a_package_points_to_exists() {
        let file: Licenses = serde_json::from_str(FILE).unwrap();
        for package in &file.packages {
            for index in &package.texts {
                assert!(
                    *index < file.texts.len(),
                    "{} points past the texts",
                    package.name
                );
            }
        }
    }
}
