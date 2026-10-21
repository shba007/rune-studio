//! Production credentials use the OS keyring. Unit tests never touch it.
#[cfg(not(test))]
pub use keyring::Entry;

#[cfg(test)]
mod memory {
    use std::{
        cell::RefCell,
        collections::HashMap,
        sync::{Arc, Mutex},
    };
    type Secrets = Arc<Mutex<HashMap<(String, String), String>>>;
    thread_local! { static SECRETS: RefCell<Secrets> = RefCell::new(Arc::default()); }

    pub struct Entry {
        secrets: Secrets,
        key: (String, String),
    }
    impl Entry {
        pub fn new(service: &str, user: &str) -> keyring::Result<Self> {
            Ok(Self {
                secrets: SECRETS.with(|s| s.borrow().clone()),
                key: (service.into(), user.into()),
            })
        }
        pub fn get_password(&self) -> keyring::Result<String> {
            self.secrets
                .lock()
                .unwrap()
                .get(&self.key)
                .cloned()
                .ok_or(keyring::Error::NoEntry)
        }
        pub fn set_password(&self, password: &str) -> keyring::Result<()> {
            self.secrets
                .lock()
                .unwrap()
                .insert(self.key.clone(), password.into());
            Ok(())
        }
        pub fn delete_credential(&self) -> keyring::Result<()> {
            self.secrets
                .lock()
                .unwrap()
                .remove(&self.key)
                .map(|_| ())
                .ok_or(keyring::Error::NoEntry)
        }
    }
}
#[cfg(test)]
pub use memory::Entry;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn entries_share_secrets_only_within_the_same_test_thread() {
        let entry = Entry::new("test", "key").unwrap();
        entry.set_password("secret").unwrap();
        assert_eq!(
            Entry::new("test", "key").unwrap().get_password().unwrap(),
            "secret"
        );
        std::thread::spawn(|| {
            assert!(matches!(
                Entry::new("test", "key").unwrap().get_password(),
                Err(keyring::Error::NoEntry)
            ))
        })
        .join()
        .unwrap();
        entry.delete_credential().unwrap();
        assert!(matches!(entry.get_password(), Err(keyring::Error::NoEntry)));
    }
}
