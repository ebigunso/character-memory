use std::path::PathBuf;
use mockall::automock;

#[automock]
pub(crate) trait EnvLoader {
    fn load_from_path(&self, path: PathBuf) -> Result<(), std::io::Error>;
    fn exists(&self, path: PathBuf) -> bool;
}

#[derive(Default)]
pub(crate) struct DefaultEnvLoader;

impl EnvLoader for DefaultEnvLoader {
    fn load_from_path(&self, path: PathBuf) -> Result<(), std::io::Error> {
        dotenvy::from_path(&path)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
    }

    fn exists(&self, path: PathBuf) -> bool {
        path.exists()
    }
}
