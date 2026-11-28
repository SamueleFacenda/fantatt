/// Why a bundle could not be loaded. Every variant names the file at fault, so
/// the message points an author at what to fix rather than at the abstract
/// layout rule.
#[derive(Debug, thiserror::Error)]
pub enum BundleError {
    #[error("the rules directory {path} could not be read: {reason}")]
    Io { path: String, reason: String },

    #[error("{path} is not a bundle directory: it must be named <name>/<semver>")]
    Layout { path: String },

    #[error("the manifest {path} is missing")]
    MissingManifest { path: String },

    #[error("the manifest {path} is not valid TOML: {reason}")]
    Manifest { path: String, reason: String },

    #[error("the manifest {path} is incoherent: {reason}")]
    InvalidManifest { path: String, reason: String },

    #[error(
        "the entry point {entrypoint} names the file {file}, which the bundle {bundle} does not carry"
    )]
    MissingEntrypoint {
        bundle: String,
        entrypoint: String,
        file: String,
    },

    #[error("the entry point {entrypoint} has no golden case at {file} in the bundle {bundle}")]
    MissingGolden {
        bundle: String,
        entrypoint: String,
        file: String,
    },

    #[error("the bundle {bundle} carries {path}, which its layout does not declare")]
    Unlisted { bundle: String, path: String },
}
