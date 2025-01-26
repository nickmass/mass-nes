use nes::SaveWram;
use sha2::{Digest, Sha256};

#[derive(Debug, Copy, Clone)]
pub struct CartridgeId([u8; 32]);

impl std::fmt::Display for CartridgeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for b in self.0 {
            write!(f, "{:02X}", b)?
        }
        Ok(())
    }
}

impl CartridgeId {
    pub fn new(data: &[u8]) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(data);
        let mut hash = [0; 32];
        hash.copy_from_slice(&hasher.finalize());
        Self(hash)
    }
}

#[derive(Debug, Clone)]
pub struct WramStorage {
    #[cfg(not(target_arch = "wasm32"))]
    storage: storage::DirectoryStorage,
    #[cfg(target_arch = "wasm32")]
    storage: storage::LocalStorage,
}

impl WramStorage {
    #[cfg(not(target_arch = "wasm32"))]
    pub fn directory<P: Into<std::path::PathBuf>>(path: P) -> Self {
        Self {
            storage: storage::DirectoryStorage { path: path.into() },
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub fn local_storage() -> Option<Self> {
        let storage = storage::LocalStorage::new()?;
        Some(Self { storage })
    }

    pub fn load_wram(&self, cart: CartridgeId) -> Option<SaveWram> {
        self.storage
            .load(cart.to_string())
            .map(|b| SaveWram::from_bytes(b))
    }

    pub fn save_wram(&self, cart: CartridgeId, wram: SaveWram) {
        self.storage.save(cart.to_string(), &wram.to_bytes());
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod storage {
    use std::path::PathBuf;
    #[derive(Debug, Clone)]
    pub struct DirectoryStorage {
        pub path: PathBuf,
    }

    impl DirectoryStorage {
        pub fn save(&self, name: String, data: &[u8]) {
            let mut path = self.path.clone();
            let _ = std::fs::create_dir_all(&path);
            path.push(format!("{}.sav", name));
            let _ = std::fs::write(path, data);
        }

        pub fn load(&self, name: String) -> Option<Vec<u8>> {
            let mut path = self.path.clone();
            path.push(format!("{}.sav", name));

            std::fs::read(path).ok()
        }
    }
}

#[cfg(target_arch = "wasm32")]
mod storage {
    use base64::prelude::*;
    #[derive(Debug, Clone)]
    pub struct LocalStorage {
        storage: web_sys::Storage,
    }

    impl LocalStorage {
        pub fn new() -> Option<Self> {
            let window = web_sys::window()?;
            let storage = window.local_storage().ok().flatten()?;

            Some(Self { storage })
        }
        pub fn save(&self, name: String, data: &[u8]) {
            let data = BASE64_STANDARD.encode(data);
            let _ = self.storage.set_item(&format!("wram.{name}"), &data);
        }

        pub fn load(&self, name: String) -> Option<Vec<u8>> {
            let s = self
                .storage
                .get_item(&format!("wram.{name}"))
                .ok()
                .flatten()?;
            BASE64_STANDARD.decode(&s).ok()
        }
    }
}
