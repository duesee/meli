/*
 * meli - mailbox module.
 *
 * Copyright 2017 Manos Pitsidianakis
 *
 * This file is part of meli.
 *
 * meli is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * meli is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with meli. If not, see <http://www.gnu.org/licenses/>.
 */

/*!
 * https://wiki2.dovecot.org/MailboxFormat/mbox
 */

use crate::async_workers::{Async, AsyncBuilder, AsyncStatus, WorkContext};
use crate::backends::BackendOp;
use crate::backends::MailboxHash;
use crate::backends::{
    BackendMailbox, MailBackend, Mailbox, MailboxPermissions, RefreshEvent, RefreshEventConsumer,
    RefreshEventKind, SpecialUsageMailbox,
};
use crate::conf::AccountSettings;
use crate::email::parser::BytesExt;
use crate::email::*;
use crate::error::{MeliError, Result};
use crate::get_path_hash;
use crate::shellexpand::ShellExpandTrait;
use libc;
use memmap::{Mmap, Protection};
use nom::{self, error::ErrorKind, IResult};
extern crate notify;
use self::notify::{watcher, DebouncedEvent, RecursiveMode, Watcher};
use std::collections::hash_map::{DefaultHasher, HashMap};
use std::fs::File;
use std::hash::Hasher;
use std::io::BufReader;
use std::io::Read;
use std::os::unix::io::AsRawFd;
use std::path::{Path, PathBuf};
use std::sync::mpsc::channel;
use std::sync::{Arc, Mutex, RwLock};

type Offset = usize;
type Length = usize;

const F_OFD_SETLKW: libc::c_int = 38;

// Open file description locking
// # man fcntl
fn get_rw_lock_blocking(f: &File) {
    let fd: libc::c_int = f.as_raw_fd();
    let mut flock: libc::flock = libc::flock {
        l_type: libc::F_WRLCK as libc::c_short,
        l_whence: libc::SEEK_SET as libc::c_short,
        l_start: 0,
        l_len: 0, /* "Specifying 0 for l_len has the special meaning: lock all bytes starting at the location
                  specified by l_whence and l_start through to the end of file, no matter how large the file grows." */
        l_pid: 0, /* "By contrast with traditional record locks, the l_pid field of that structure must be set to zero when using the commands described below." */
    };
    let ptr: *mut libc::flock = &mut flock;
    let ret_val = unsafe { libc::fcntl(fd, F_OFD_SETLKW, ptr as *mut libc::c_void) };
    debug!(&ret_val);
    assert!(-1 != ret_val);
}

#[derive(Debug)]
struct MboxMailbox {
    hash: MailboxHash,
    name: String,
    path: PathBuf,
    content: Vec<u8>,
    children: Vec<MailboxHash>,
    parent: Option<MailboxHash>,
    usage: Arc<RwLock<SpecialUsageMailbox>>,
    is_subscribed: bool,
    permissions: MailboxPermissions,
    pub total: Arc<Mutex<usize>>,
    pub unseen: Arc<Mutex<usize>>,
}

impl BackendMailbox for MboxMailbox {
    fn hash(&self) -> MailboxHash {
        self.hash
    }

    fn name(&self) -> &str {
        self.name.as_str()
    }

    fn path(&self) -> &str {
        /* We know it's valid UTF-8 because we supplied it */
        self.path.to_str().unwrap()
    }

    fn change_name(&mut self, s: &str) {
        self.name = s.to_string();
    }

    fn clone(&self) -> Mailbox {
        Box::new(MboxMailbox {
            hash: self.hash,
            name: self.name.clone(),
            path: self.path.clone(),
            content: self.content.clone(),
            children: self.children.clone(),
            usage: self.usage.clone(),
            is_subscribed: self.is_subscribed,
            parent: self.parent,
            permissions: self.permissions,
            unseen: self.unseen.clone(),
            total: self.total.clone(),
        })
    }

    fn children(&self) -> &[MailboxHash] {
        &self.children
    }

    fn parent(&self) -> Option<MailboxHash> {
        self.parent
    }

    fn special_usage(&self) -> SpecialUsageMailbox {
        *self.usage.read().unwrap()
    }

    fn permissions(&self) -> MailboxPermissions {
        self.permissions
    }
    fn is_subscribed(&self) -> bool {
        self.is_subscribed
    }
    fn set_is_subscribed(&mut self, new_val: bool) -> Result<()> {
        self.is_subscribed = new_val;
        Ok(())
    }
    fn set_special_usage(&mut self, new_val: SpecialUsageMailbox) -> Result<()> {
        *self.usage.write()? = new_val;
        Ok(())
    }

    fn count(&self) -> Result<(usize, usize)> {
        Ok((*self.unseen.lock()?, *self.total.lock()?))
    }
}

/// `BackendOp` implementor for Mbox
#[derive(Debug, Default)]
pub struct MboxOp {
    hash: EnvelopeHash,
    path: PathBuf,
    offset: Offset,
    length: Length,
    slice: Option<Mmap>,
}

impl MboxOp {
    pub fn new(hash: EnvelopeHash, path: &Path, offset: Offset, length: Length) -> Self {
        MboxOp {
            hash,
            path: path.to_path_buf(),
            slice: None,
            offset,
            length,
        }
    }
}

impl BackendOp for MboxOp {
    fn description(&self) -> String {
        String::new()
    }

    fn as_bytes(&mut self) -> Result<&[u8]> {
        if self.slice.is_none() {
            self.slice = Some(Mmap::open_path(&self.path, Protection::Read)?);
        }
        /* Unwrap is safe since we use ? above. */
        Ok(unsafe {
            &self.slice.as_ref().unwrap().as_slice()[self.offset..self.offset + self.length]
        })
    }

    fn fetch_flags(&self) -> Flag {
        let mut flags = Flag::empty();
        let file = match std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&self.path)
        {
            Ok(f) => f,
            Err(e) => {
                debug!(e);
                return flags;
            }
        };
        get_rw_lock_blocking(&file);
        let mut buf_reader = BufReader::new(file);
        let mut contents = Vec::new();
        if let Err(e) = buf_reader.read_to_end(&mut contents) {
            debug!(e);
            return flags;
        };

        if let Ok((_, headers)) = parser::headers::headers_raw(contents.as_slice()) {
            if let Some(start) = headers.find(b"Status:") {
                if let Some(end) = headers[start..].find(b"\n") {
                    let start = start + b"Status:".len();
                    let status = headers[start..start + end].trim();
                    if status.contains(&b'F') {
                        flags.set(Flag::FLAGGED, true);
                    }
                    if status.contains(&b'A') {
                        flags.set(Flag::REPLIED, true);
                    }
                    if status.contains(&b'R') {
                        flags.set(Flag::SEEN, true);
                    }
                    if status.contains(&b'D') {
                        flags.set(Flag::TRASHED, true);
                    }
                    if status.contains(&b'T') {
                        flags.set(Flag::DRAFT, true);
                    }
                }
            }
            if let Some(start) = headers.find(b"X-Status:") {
                let start = start + b"X-Status:".len();
                if let Some(end) = headers[start..].find(b"\n") {
                    let status = headers[start..start + end].trim();
                    if status.contains(&b'F') {
                        flags.set(Flag::FLAGGED, true);
                    }
                    if status.contains(&b'A') {
                        flags.set(Flag::REPLIED, true);
                    }
                    if status.contains(&b'R') {
                        flags.set(Flag::SEEN, true);
                    }
                    if status.contains(&b'D') {
                        flags.set(Flag::TRASHED, true);
                    }
                    if status.contains(&b'T') {
                        flags.set(Flag::DRAFT, true);
                    }
                }
            }
        }
        flags
    }

    fn set_flag(&mut self, _envelope: &mut Envelope, _flag: Flag, _value: bool) -> Result<()> {
        Ok(())
    }

    fn set_tag(&mut self, _envelope: &mut Envelope, _tag: String, _value: bool) -> Result<()> {
        Err(MeliError::new("mbox doesn't support tags."))
    }
}

pub fn mbox_parse(
    index: Arc<Mutex<HashMap<EnvelopeHash, (Offset, Length)>>>,
    input: &[u8],
    file_offset: usize,
) -> IResult<&[u8], Vec<Envelope>> {
    if input.is_empty() {
        return Err(nom::Err::Error((input, ErrorKind::Tag)));
    }
    let mut input = input;
    let mut offset = 0;
    let mut index = index.lock().unwrap();
    let mut envelopes = Vec::with_capacity(32);
    while !input.is_empty() {
        let next_offset: Option<(usize, usize)> = input
            .find(b"\n\nFrom ")
            .and_then(|end| input.find(b"\n").and_then(|start| Some((start + 1, end))));

        if let Some((start, len)) = next_offset {
            match Envelope::from_bytes(&input[start..len], None) {
                Ok(mut env) => {
                    let mut flags = Flag::empty();
                    if env.other_headers().contains_key("Status") {
                        if env.other_headers()["Status"].contains("F") {
                            flags.set(Flag::FLAGGED, true);
                        }
                        if env.other_headers()["Status"].contains("A") {
                            flags.set(Flag::REPLIED, true);
                        }
                        if env.other_headers()["Status"].contains("R") {
                            flags.set(Flag::SEEN, true);
                        }
                        if env.other_headers()["Status"].contains("D") {
                            flags.set(Flag::TRASHED, true);
                        }
                    }
                    if env.other_headers().contains_key("X-Status") {
                        if env.other_headers()["X-Status"].contains("F") {
                            flags.set(Flag::FLAGGED, true);
                        }
                        if env.other_headers()["X-Status"].contains("A") {
                            flags.set(Flag::REPLIED, true);
                        }
                        if env.other_headers()["X-Status"].contains("R") {
                            flags.set(Flag::SEEN, true);
                        }
                        if env.other_headers()["X-Status"].contains("D") {
                            flags.set(Flag::TRASHED, true);
                        }
                        if env.other_headers()["X-Status"].contains("T") {
                            flags.set(Flag::DRAFT, true);
                        }
                    }
                    env.set_flags(flags);
                    index.insert(env.hash(), (offset + file_offset + start, len - start));
                    envelopes.push(env);
                }
                Err(_) => {
                    debug!("Could not parse mail at byte offset {}", offset);
                }
            }
            offset += len + 2;
            input = &input[len + 2..];
        } else {
            let start: Offset = input.find(b"\n").map(|v| v + 1).unwrap_or(0);
            match Envelope::from_bytes(&input[start..], None) {
                Ok(mut env) => {
                    let mut flags = Flag::empty();
                    if env.other_headers().contains_key("Status") {
                        if env.other_headers()["Status"].contains("F") {
                            flags.set(Flag::FLAGGED, true);
                        }
                        if env.other_headers()["Status"].contains("A") {
                            flags.set(Flag::REPLIED, true);
                        }
                        if env.other_headers()["Status"].contains("R") {
                            flags.set(Flag::SEEN, true);
                        }
                        if env.other_headers()["Status"].contains("D") {
                            flags.set(Flag::TRASHED, true);
                        }
                    }
                    if env.other_headers().contains_key("X-Status") {
                        if env.other_headers()["X-Status"].contains("F") {
                            flags.set(Flag::FLAGGED, true);
                        }
                        if env.other_headers()["X-Status"].contains("A") {
                            flags.set(Flag::REPLIED, true);
                        }
                        if env.other_headers()["X-Status"].contains("R") {
                            flags.set(Flag::SEEN, true);
                        }
                        if env.other_headers()["X-Status"].contains("D") {
                            flags.set(Flag::TRASHED, true);
                        }
                        if env.other_headers()["X-Status"].contains("T") {
                            flags.set(Flag::DRAFT, true);
                        }
                    }
                    env.set_flags(flags);
                    index.insert(
                        env.hash(),
                        (offset + file_offset + start, input.len() - start),
                    );
                    envelopes.push(env);
                }
                Err(_) => {
                    debug!("Could not parse mail at byte offset {}", offset);
                }
            }
            break;
        }
    }
    return Ok((&[], envelopes));
}

/// Mbox backend
#[derive(Debug, Default)]
pub struct MboxType {
    account_name: String,
    path: PathBuf,
    index: Arc<Mutex<HashMap<EnvelopeHash, (Offset, Length)>>>,
    mailboxes: Arc<Mutex<HashMap<MailboxHash, MboxMailbox>>>,
}

impl MailBackend for MboxType {
    fn is_online(&self) -> Result<()> {
        Ok(())
    }
    fn get(&mut self, mailbox: &Mailbox) -> Async<Result<Vec<Envelope>>> {
        let mut w = AsyncBuilder::new();
        let handle = {
            let tx = w.tx();
            let index = self.index.clone();
            let mailbox_path = mailbox.path().to_string();
            let mailbox_hash = mailbox.hash();
            let mailboxes = self.mailboxes.clone();
            let closure = move |_work_context| {
                let tx = tx.clone();
                let index = index.clone();
                let file = match std::fs::OpenOptions::new()
                    .read(true)
                    .write(true)
                    .open(&mailbox_path)
                {
                    Ok(f) => f,
                    Err(e) => {
                        tx.send(AsyncStatus::Payload(Err(MeliError::from(e))))
                            .unwrap();
                        return;
                    }
                };
                get_rw_lock_blocking(&file);
                let mut buf_reader = BufReader::new(file);
                let mut contents = Vec::new();
                if let Err(e) = buf_reader.read_to_end(&mut contents) {
                    tx.send(AsyncStatus::Payload(Err(MeliError::from(e))))
                        .unwrap();
                    return;
                };

                let payload = mbox_parse(index, contents.as_slice(), 0)
                    .map_err(|e| MeliError::from(e))
                    .map(|(_, v)| v);
                {
                    let mut mailbox_lock = mailboxes.lock().unwrap();
                    mailbox_lock
                        .entry(mailbox_hash)
                        .and_modify(|f| f.content = contents);
                }

                tx.send(AsyncStatus::Payload(payload)).unwrap();
                tx.send(AsyncStatus::Finished).unwrap();
            };
            Box::new(closure)
        };
        w.build(handle)
    }

    fn watch(
        &self,
        sender: RefreshEventConsumer,
        work_context: WorkContext,
    ) -> Result<std::thread::ThreadId> {
        let (tx, rx) = channel();
        let mut watcher = watcher(tx, std::time::Duration::from_secs(10))
            .map_err(|e| e.to_string())
            .map_err(MeliError::new)?;
        for f in self.mailboxes.lock().unwrap().values() {
            watcher
                .watch(&f.path, RecursiveMode::Recursive)
                .map_err(|e| e.to_string())
                .map_err(MeliError::new)?;
            debug!("watching {:?}", f.path.as_path());
        }
        let account_hash = {
            let mut hasher = DefaultHasher::new();
            hasher.write(self.account_name.as_bytes());
            hasher.finish()
        };
        let index = self.index.clone();
        let mailboxes = self.mailboxes.clone();
        let handle = std::thread::Builder::new()
            .name(format!("watching {}", self.account_name,))
            .spawn(move || {
                // Move `watcher` in the closure's scope so that it doesn't get dropped.
                let _watcher = watcher;
                let _work_context = work_context;
                let index = index;
                let mailboxes = mailboxes;
                loop {
                    match rx.recv() {
                        /*
                         * Event types:
                         *
                         * pub enum RefreshEventKind {
                         *     Update(EnvelopeHash, Envelope), // Old hash, new envelope
                         *     Create(Envelope),
                         *     Remove(EnvelopeHash),
                         *     Rescan,
                         * }
                         */
                        Ok(event) => match event {
                            /* Update */
                            DebouncedEvent::NoticeWrite(pathbuf)
                            | DebouncedEvent::Write(pathbuf) => {
                                let mailbox_hash = get_path_hash!(&pathbuf);
                                let file = match std::fs::OpenOptions::new()
                                    .read(true)
                                    .write(true)
                                    .open(&pathbuf)
                                {
                                    Ok(f) => f,
                                    Err(_) => {
                                        continue;
                                    }
                                };
                                get_rw_lock_blocking(&file);
                                let mut mailbox_lock = mailboxes.lock().unwrap();
                                let mut buf_reader = BufReader::new(file);
                                let mut contents = Vec::new();
                                if let Err(e) = buf_reader.read_to_end(&mut contents) {
                                    debug!(e);
                                    continue;
                                };
                                if contents
                                    .starts_with(mailbox_lock[&mailbox_hash].content.as_slice())
                                {
                                    if let Ok((_, envelopes)) = mbox_parse(
                                        index.clone(),
                                        &contents[mailbox_lock[&mailbox_hash].content.len()..],
                                        mailbox_lock[&mailbox_hash].content.len(),
                                    ) {
                                        for env in envelopes {
                                            sender.send(RefreshEvent {
                                                account_hash,
                                                mailbox_hash,
                                                kind: RefreshEventKind::Create(Box::new(env)),
                                            });
                                        }
                                    }
                                } else {
                                    sender.send(RefreshEvent {
                                        account_hash,
                                        mailbox_hash,
                                        kind: RefreshEventKind::Rescan,
                                    });
                                }
                                mailbox_lock
                                    .entry(mailbox_hash)
                                    .and_modify(|f| f.content = contents);
                            }
                            /* Remove */
                            DebouncedEvent::NoticeRemove(pathbuf)
                            | DebouncedEvent::Remove(pathbuf) => {
                                if mailboxes
                                    .lock()
                                    .unwrap()
                                    .values()
                                    .any(|f| &f.path == &pathbuf)
                                {
                                    let mailbox_hash = get_path_hash!(&pathbuf);
                                    sender.send(RefreshEvent {
                                        account_hash,
                                        mailbox_hash,
                                        kind: RefreshEventKind::Failure(MeliError::new(format!(
                                            "mbox mailbox {} was removed.",
                                            pathbuf.display()
                                        ))),
                                    });
                                    return;
                                }
                            }
                            DebouncedEvent::Rename(src, dest) => {
                                if mailboxes.lock().unwrap().values().any(|f| &f.path == &src) {
                                    let mailbox_hash = get_path_hash!(&src);
                                    sender.send(RefreshEvent {
                                        account_hash,
                                        mailbox_hash,
                                        kind: RefreshEventKind::Failure(MeliError::new(format!(
                                            "mbox mailbox {} was renamed to {}.",
                                            src.display(),
                                            dest.display()
                                        ))),
                                    });
                                    return;
                                }
                            }
                            /* Trigger rescan of mailboxes */
                            DebouncedEvent::Rescan => {
                                for &mailbox_hash in mailboxes.lock().unwrap().keys() {
                                    sender.send(RefreshEvent {
                                        account_hash,
                                        mailbox_hash,
                                        kind: RefreshEventKind::Rescan,
                                    });
                                }
                                return;
                            }
                            _ => {}
                        },
                        Err(e) => debug!("watch error: {:?}", e),
                    }
                }
            })?;
        Ok(handle.thread().id())
    }
    fn mailboxes(&self) -> Result<HashMap<MailboxHash, Mailbox>> {
        Ok(self
            .mailboxes
            .lock()
            .unwrap()
            .iter()
            .map(|(h, f)| (*h, f.clone() as Mailbox))
            .collect())
    }
    fn operation(&self, hash: EnvelopeHash) -> Box<dyn BackendOp> {
        let (offset, length) = {
            let index = self.index.lock().unwrap();
            index[&hash]
        };
        Box::new(MboxOp::new(hash, self.path.as_path(), offset, length))
    }

    fn save(&self, _bytes: &[u8], _mailbox_hash: MailboxHash, _flags: Option<Flag>) -> Result<()> {
        Err(MeliError::new("Unimplemented."))
    }

    fn as_any(&self) -> &dyn::std::any::Any {
        self
    }
}

impl MboxType {
    pub fn new(
        s: &AccountSettings,
        _is_subscribed: Box<dyn Fn(&str) -> bool>,
    ) -> Result<Box<dyn MailBackend>> {
        let path = Path::new(s.root_mailbox.as_str()).expand();
        if !path.exists() {
            return Err(MeliError::new(format!(
                "\"root_mailbox\" {} for account {} is not a valid path.",
                s.root_mailbox.as_str(),
                s.name()
            )));
        }
        let ret = MboxType {
            account_name: s.name().to_string(),
            path,
            ..Default::default()
        };
        let name: String = ret
            .path
            .file_name()
            .map(|f| f.to_string_lossy().into())
            .unwrap_or(String::new());
        let hash = get_path_hash!(&ret.path);

        let read_only = if let Ok(metadata) = std::fs::metadata(&ret.path) {
            metadata.permissions().readonly()
        } else {
            true
        };

        ret.mailboxes.lock().unwrap().insert(
            hash,
            MboxMailbox {
                hash,
                path: ret.path.clone(),
                name,
                content: Vec::new(),
                children: Vec::new(),
                parent: None,
                usage: Arc::new(RwLock::new(SpecialUsageMailbox::Normal)),
                is_subscribed: true,
                permissions: MailboxPermissions {
                    create_messages: !read_only,
                    remove_messages: !read_only,
                    set_flags: !read_only,
                    create_child: !read_only,
                    rename_messages: !read_only,
                    delete_messages: !read_only,
                    delete_mailbox: !read_only,
                    change_permissions: false,
                },
                unseen: Arc::new(Mutex::new(0)),
                total: Arc::new(Mutex::new(0)),
            },
        );
        /*
        /* Look for other mailboxes */
        let parent_mailbox = Path::new(path).parent().unwrap();
        let read_dir = std::fs::read_dir(parent_mailbox);
        if read_dir.is_ok() {
            for f in read_dir.unwrap() {
                if f.is_err() {
                    continue;
                }
                let f = f.unwrap().path();
                if f.is_file() && f != path {
                    let name: String = f
                        .file_name()
                        .map(|f| f.to_string_lossy().into())
                        .unwrap_or(String::new());
                    let hash = get_path_hash!(f);
                    ret.mailboxes.lock().unwrap().insert(
                        hash,
                        MboxMailbox {
                            hash,
                            path: f,
                            name,
                            content: Vec::new(),
                            children: Vec::new(),
                            parent: None,
                        },
                    );
                }
            }
        }
        */
        Ok(Box::new(ret))
    }

    pub fn validate_config(s: &AccountSettings) -> Result<()> {
        let path = Path::new(s.root_mailbox.as_str()).expand();
        if !path.exists() {
            return Err(MeliError::new(format!(
                "\"root_mailbox\" {} for account {} is not a valid path.",
                s.root_mailbox.as_str(),
                s.name()
            )));
        }
        Ok(())
    }
}
