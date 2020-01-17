use crate::LintLevel;
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Config {
    /// How to handle registries that weren't listed
    #[serde(default = "crate::lint_warn")]
    pub unknown_registry: LintLevel,
    /// How to handle git sources that weren't listed
    #[serde(default = "crate::lint_warn")]
    pub unknown_git: LintLevel,
    /// The list of registries that crates can be sourced from.
    /// Defaults to the crates.io registry if not specified.
    #[serde(default = "default_allow_registry")]
    pub allow_registry: Vec<String>,
    /// The list of git repositories that crates can be sourced from.
    #[serde(default)]
    pub allow_git: Vec<toml::Spanned<String>>,
}

fn default_allow_registry() -> Vec<String> {
    vec!["https://github.com/rust-lang/crates.io-index".to_owned()]
}

impl Default for Config {
    fn default() -> Self {
        Self {
            unknown_registry: LintLevel::Warn,
            unknown_git: LintLevel::Warn,
            allow_registry: default_allow_registry(),
            allow_git: Vec::new(),
        }
    }
}

use crate::diag::{Diagnostic, Label};

impl Config {
    pub fn validate(
        self,
        cfg_file: codespan::FileId,
        contents: &str,
    ) -> Result<ValidConfig, Vec<Diagnostic>> {
        let mut diags = Vec::new();

        let mut allowed_sources =
            Vec::with_capacity(self.allow_registry.len() + self.allow_git.len());
        for ar in self.allow_registry {
            // Attempt to find the url in the toml contents
            let span = match contents.find(&ar) {
                Some(ari) => (ari - 1) as u32..(ari + ar.len() + 1) as u32,
                None => 0..0,
            };

            match url::Url::parse(&ar) {
                Ok(url) => {
                    allowed_sources.push(SourceSpan::new(url, span));
                }
                Err(pe) => {
                    diags.push(Diagnostic::new_error(
                        "failed to parse url",
                        Label::new(cfg_file, span, pe.to_string()),
                    ));
                }
            }
        }

        for ag in self.allow_git {
            let span = ag.start() as u32..ag.end() as u32;
            match url::Url::parse(ag.get_ref()) {
                Ok(url) => {
                    allowed_sources.push(SourceSpan::new(url, span));
                }
                Err(pe) => {
                    diags.push(Diagnostic::new_error(
                        "failed to parse url",
                        Label::new(cfg_file, span, pe.to_string()),
                    ));
                }
            }
        }

        if !diags.is_empty() {
            return Err(diags);
        }

        Ok(ValidConfig {
            file_id: cfg_file,
            unknown_registry: self.unknown_registry,
            unknown_git: self.unknown_git,
            allowed_sources,
        })
    }
}

pub type SourceSpan = crate::Spanned<url::Url>;

#[doc(hidden)]
pub struct ValidConfig {
    pub file_id: codespan::FileId,

    pub unknown_registry: LintLevel,
    pub unknown_git: LintLevel,
    pub allowed_sources: Vec<SourceSpan>,
}
