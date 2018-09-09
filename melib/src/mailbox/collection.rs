extern crate bincode;
extern crate xdg;

use super::*;
use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::ops::{Deref, DerefMut};
use std::result;

extern crate fnv;
use self::fnv::FnvHashMap;

/// `Mailbox` represents a folder of mail.
#[derive(Debug, Clone, Default)]
pub struct Collection {
    pub envelopes: FnvHashMap<EnvelopeHash, Envelope>,
    date_index: BTreeMap<UnixTimestamp, EnvelopeHash>,
    subject_index: Option<BTreeMap<String, EnvelopeHash>>,
    pub threads: Threads,
}

impl Collection {
    pub fn new(vec: Vec<Envelope>, name: &str) -> Collection {
        let mut envelopes: FnvHashMap<EnvelopeHash, Envelope> =
            FnvHashMap::with_capacity_and_hasher(vec.len(), Default::default());
        for e in vec {
            envelopes.insert(e.hash(), e);
        }
        let date_index = BTreeMap::new();
        let subject_index = None;

        let cache_dir = xdg::BaseDirectories::with_profile("meli", name).unwrap();
        let threads = if let Some(cached) = cache_dir.find_cache_file("threads") {
            let reader = io::BufReader::new(fs::File::open(cached).unwrap());
            let result: result::Result<Threads, _> = bincode::deserialize_from(reader);
            if let Ok(mut cached_t) = result {
                cached_t.update(&mut envelopes);
                cached_t
            } else {
                Threads::new(&mut envelopes) // sent_folder);
            }
        } else {
            Threads::new(&mut envelopes) // sent_folder);
        };
        Collection {
            envelopes,
            date_index,
            subject_index,
            threads,
        }
    }

    pub fn len(&self) -> usize {
        self.envelopes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.envelopes.is_empty()
    }

    pub fn insert(&mut self, hash: EnvelopeHash, mut envelope: Envelope) {
        self.threads.insert(&mut envelope);
        self.envelopes.insert(hash, envelope);
    }
    pub(crate) fn insert_reply(&mut self, hash: EnvelopeHash, mut envelope: Envelope) {
        if self.threads.insert_reply(&mut envelope) {
            self.envelopes.insert(hash, envelope);
        }
    }
}

impl Deref for Collection {
    type Target = FnvHashMap<EnvelopeHash, Envelope>;

    fn deref(&self) -> &FnvHashMap<EnvelopeHash, Envelope> {
        &self.envelopes
    }
}

impl DerefMut for Collection {
    fn deref_mut(&mut self) -> &mut FnvHashMap<EnvelopeHash, Envelope> {
        &mut self.envelopes
    }
}
