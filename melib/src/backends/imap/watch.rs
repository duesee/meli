/*
 * meli - imap module.
 *
 * Copyright 2019 Manos Pitsidianakis
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
use std::sync::Arc;

use super::*;
use crate::backends::SpecialUsageMailbox;

/// Arguments for IMAP watching functions
pub struct ImapWatchKit {
    pub conn: ImapConnection,
    pub main_conn: Arc<FutureMutex<ImapConnection>>,
    pub uid_store: Arc<UIDStore>,
}

pub async fn poll_with_examine(kit: ImapWatchKit) -> Result<()> {
    debug!("poll with examine");
    let ImapWatchKit {
        mut conn,
        main_conn: _,
        uid_store,
    } = kit;
    conn.connect().await?;
    let mailboxes: HashMap<MailboxHash, ImapMailbox> = {
        let mailboxes_lck = timeout(uid_store.timeout, uid_store.mailboxes.lock()).await?;
        mailboxes_lck.clone()
    };
    loop {
        for (_, mailbox) in mailboxes.clone() {
            examine_updates(mailbox, &mut conn, &uid_store).await?;
        }
        //FIXME: make sleep duration configurable
        smol::Timer::after(std::time::Duration::from_secs(3 * 60)).await;
    }
}

pub async fn idle(kit: ImapWatchKit) -> Result<()> {
    debug!("IDLE");
    /* IDLE only watches the connection's selected mailbox. We will IDLE on INBOX
     * and every ~5 minutes wake up and poll the others */
    let ImapWatchKit {
        mut conn,
        main_conn,
        uid_store,
    } = kit;
    conn.connect().await?;
    let mailbox: ImapMailbox = match uid_store
        .mailboxes
        .lock()
        .await
        .values()
        .find(|f| f.parent.is_none() && (f.special_usage() == SpecialUsageMailbox::Inbox))
        .map(std::clone::Clone::clone)
    {
        Some(mailbox) => mailbox,
        None => {
            return Err(Error::new(
                "INBOX mailbox not found in local mailbox index. meli may have not parsed the \
                 IMAP mailboxes correctly",
            ));
        }
    };
    let mailbox_hash = mailbox.hash();
    let mut response = Vec::with_capacity(8 * 1024);
    let select_response = conn
        .examine_mailbox(mailbox_hash, &mut response, true)
        .await?
        .unwrap();
    {
        let mut uidvalidities = uid_store.uidvalidity.lock().unwrap();

        if let Some(v) = uidvalidities.get(&mailbox_hash) {
            if *v != select_response.uidvalidity {
                if uid_store.keep_offline_cache {
                    #[cfg(not(feature = "sqlite3"))]
                    let mut cache_handle = super::cache::DefaultCache::get(uid_store.clone())?;
                    #[cfg(feature = "sqlite3")]
                    let mut cache_handle = super::cache::Sqlite3Cache::get(uid_store.clone())?;
                    cache_handle.clear(mailbox_hash, &select_response)?;
                }
                conn.add_refresh_event(RefreshEvent {
                    account_hash: uid_store.account_hash,
                    mailbox_hash,
                    kind: RefreshEventKind::Rescan,
                });
                /*
                uid_store.uid_index.lock().unwrap().clear();
                uid_store.hash_index.lock().unwrap().clear();
                uid_store.byte_cache.lock().unwrap().clear();
                */
            }
        } else {
            uidvalidities.insert(mailbox_hash, select_response.uidvalidity);
        }
    }
    let mailboxes: HashMap<MailboxHash, ImapMailbox> = {
        let mailboxes_lck = timeout(uid_store.timeout, uid_store.mailboxes.lock()).await?;
        mailboxes_lck.clone()
    };
    for (h, mailbox) in mailboxes.clone() {
        if mailbox_hash == h {
            continue;
        }
        examine_updates(mailbox, &mut conn, &uid_store).await?;
    }
    conn.send_command(b"IDLE").await?;
    let mut blockn = ImapBlockingConnection::from(conn);
    let mut watch = std::time::Instant::now();
    /* duration interval to send heartbeat */
    const _10_MINS: std::time::Duration = std::time::Duration::from_secs(10 * 60);
    /* duration interval to check other mailboxes for changes */
    const _5_MINS: std::time::Duration = std::time::Duration::from_secs(5 * 60);
    loop {
        let line = match timeout(Some(_10_MINS), blockn.as_stream()).await {
            Ok(Some(line)) => line,
            Ok(None) => {
                debug!("IDLE connection dropped: {:?}", &blockn.err());
                blockn.conn.connect().await?;
                let mut main_conn_lck = timeout(uid_store.timeout, main_conn.lock()).await?;
                main_conn_lck.connect().await?;
                continue;
            }
            Err(_) => {
                /* Timeout */
                blockn.conn.send_raw(b"DONE").await?;
                blockn
                    .conn
                    .read_response(&mut response, RequiredResponses::empty())
                    .await?;
                blockn.conn.send_command(b"IDLE").await?;
                let mut main_conn_lck = timeout(uid_store.timeout, main_conn.lock()).await?;
                main_conn_lck.connect().await?;
                continue;
            }
        };
        let now = std::time::Instant::now();
        if now.duration_since(watch) >= _5_MINS {
            /* Time to poll all inboxes */
            let mut conn = timeout(uid_store.timeout, main_conn.lock()).await?;
            for (_h, mailbox) in mailboxes.clone() {
                examine_updates(mailbox, &mut conn, &uid_store).await?;
            }
            watch = now;
        }
        if line
            .split_rn()
            .filter(|l| {
                !l.starts_with(b"+ ")
                    && !l.starts_with(b"* ok")
                    && !l.starts_with(b"* ok")
                    && !l.starts_with(b"* Ok")
                    && !l.starts_with(b"* OK")
            })
            .count()
            == 0
        {
            continue;
        }
        {
            blockn.conn.send_raw(b"DONE").await?;
            blockn
                .conn
                .read_response(&mut response, RequiredResponses::empty())
                .await?;
            for l in line.split_rn().chain(response.split_rn()) {
                debug!("process_untagged {:?}", &l);
                if l.starts_with(b"+ ")
                    || l.starts_with(b"* ok")
                    || l.starts_with(b"* ok")
                    || l.starts_with(b"* Ok")
                    || l.starts_with(b"* OK")
                {
                    debug!("ignore continuation mark");
                    continue;
                }
                blockn.conn.process_untagged(l).await?;
            }
            blockn.conn.send_command(b"IDLE").await?;
        }
    }
}

pub async fn examine_updates(
    mailbox: ImapMailbox,
    conn: &mut ImapConnection,
    uid_store: &Arc<UIDStore>,
) -> Result<()> {
    if mailbox.no_select {
        return Ok(());
    }
    let mailbox_hash = mailbox.hash();
    debug!("examining mailbox {} {}", mailbox_hash, mailbox.path());
    if let Some(new_envelopes) = conn.resync(mailbox_hash).await? {
        for env in new_envelopes {
            conn.add_refresh_event(RefreshEvent {
                mailbox_hash,
                account_hash: uid_store.account_hash,
                kind: RefreshEventKind::Create(Box::new(env)),
            });
        }
    } else {
        #[cfg(not(feature = "sqlite3"))]
        let mut cache_handle = super::cache::DefaultCache::get(uid_store.clone())?;
        #[cfg(feature = "sqlite3")]
        let mut cache_handle = super::cache::Sqlite3Cache::get(uid_store.clone())?;
        let mut response = Vec::with_capacity(8 * 1024);
        let select_response = conn
            .examine_mailbox(mailbox_hash, &mut response, true)
            .await?
            .unwrap();
        {
            let mut uidvalidities = uid_store.uidvalidity.lock().unwrap();

            if let Some(v) = uidvalidities.get(&mailbox_hash) {
                if *v != select_response.uidvalidity {
                    if uid_store.keep_offline_cache {
                        cache_handle.clear(mailbox_hash, &select_response)?;
                    }
                    conn.add_refresh_event(RefreshEvent {
                        account_hash: uid_store.account_hash,
                        mailbox_hash,
                        kind: RefreshEventKind::Rescan,
                    });
                    /*
                    uid_store.uid_index.lock().unwrap().clear();
                    uid_store.hash_index.lock().unwrap().clear();
                    uid_store.byte_cache.lock().unwrap().clear();
                    */
                    return Ok(());
                }
            } else {
                uidvalidities.insert(mailbox_hash, select_response.uidvalidity);
            }
        }
        if mailbox.is_cold() {
            /* Mailbox hasn't been loaded yet */
            let has_list_status: bool = conn
                .uid_store
                .capabilities
                .lock()
                .unwrap()
                .iter()
                .any(|cap| cap.eq_ignore_ascii_case(b"LIST-STATUS"));
            if has_list_status {
                conn.send_command(
                    format!(
                        "LIST \"{}\" \"\" RETURN (STATUS (MESSAGES UNSEEN))",
                        mailbox.imap_path()
                    )
                    .as_bytes(),
                )
                .await?;
                conn.read_response(
                    &mut response,
                    RequiredResponses::LIST_REQUIRED | RequiredResponses::STATUS,
                )
                .await?;
                debug!(
                    "list return status out: {}",
                    String::from_utf8_lossy(&response)
                );
                for l in response.split_rn() {
                    if !l.starts_with(b"*") {
                        continue;
                    }
                    if let Ok(status) = protocol_parser::status_response(l).map(|(_, v)| v) {
                        if Some(mailbox_hash) == status.mailbox {
                            if let Some(total) = status.messages {
                                if let Ok(mut exists_lck) = mailbox.exists.lock() {
                                    exists_lck.clear();
                                    exists_lck.set_not_yet_seen(total);
                                }
                            }
                            if let Some(total) = status.unseen {
                                if let Ok(mut unseen_lck) = mailbox.unseen.lock() {
                                    unseen_lck.clear();
                                    unseen_lck.set_not_yet_seen(total);
                                }
                            }
                            break;
                        }
                    }
                }
            } else {
                conn.send_command(b"SEARCH UNSEEN").await?;
                conn.read_response(&mut response, RequiredResponses::SEARCH)
                    .await?;
                let unseen_count = protocol_parser::search_results(&response)?.1.len();
                if let Ok(mut exists_lck) = mailbox.exists.lock() {
                    exists_lck.clear();
                    exists_lck.set_not_yet_seen(select_response.exists);
                }
                if let Ok(mut unseen_lck) = mailbox.unseen.lock() {
                    unseen_lck.clear();
                    unseen_lck.set_not_yet_seen(unseen_count);
                }
            }
            mailbox.set_warm(true);
            return Ok(());
        }

        if select_response.recent > 0 {
            /* UID SEARCH RECENT */
            conn.send_command(b"UID SEARCH RECENT").await?;
            conn.read_response(&mut response, RequiredResponses::SEARCH)
                .await?;
            let v = protocol_parser::search_results(response.as_slice()).map(|(_, v)| v)?;
            if v.is_empty() {
                debug!(
                    "search response was empty: {}",
                    String::from_utf8_lossy(&response)
                );
                return Ok(());
            }
            let mut cmd = "UID FETCH ".to_string();
            cmd.push_str(&v[0].to_string());
            if v.len() != 1 {
                for n in v.into_iter().skip(1) {
                    cmd.push(',');
                    cmd.push_str(&n.to_string());
                }
            }
            cmd.push_str(
                " (UID FLAGS ENVELOPE BODY.PEEK[HEADER.FIELDS (REFERENCES)] BODYSTRUCTURE)",
            );
            conn.send_command(cmd.as_bytes()).await?;
            conn.read_response(&mut response, RequiredResponses::FETCH_REQUIRED)
                .await?;
        } else if select_response.exists > mailbox.exists.lock().unwrap().len() {
            conn.send_command(
                format!(
                    "FETCH {}:* (UID FLAGS ENVELOPE BODY.PEEK[HEADER.FIELDS (REFERENCES)] \
                     BODYSTRUCTURE)",
                    std::cmp::max(mailbox.exists.lock().unwrap().len(), 1)
                )
                .as_bytes(),
            )
            .await?;
            conn.read_response(&mut response, RequiredResponses::FETCH_REQUIRED)
                .await?;
        } else {
            return Ok(());
        }
        debug!(
            "fetch response is {} bytes and {} lines",
            response.len(),
            String::from_utf8_lossy(&response).lines().count()
        );
        let (_, mut v, _) = protocol_parser::fetch_responses(&response)?;
        debug!("responses len is {}", v.len());
        for FetchResponse {
            ref uid,
            ref mut envelope,
            ref mut flags,
            ref references,
            ..
        } in v.iter_mut()
        {
            let uid = uid.unwrap();
            let env = envelope.as_mut().unwrap();
            env.set_hash(generate_envelope_hash(mailbox.imap_path(), &uid));
            if let Some(value) = references {
                env.set_references(value);
            }
            let mut tag_lck = uid_store.collection.tag_index.write().unwrap();
            if let Some((flags, keywords)) = flags {
                env.set_flags(*flags);
                if !env.is_seen() {
                    mailbox.unseen.lock().unwrap().insert_new(env.hash());
                }
                mailbox.exists.lock().unwrap().insert_new(env.hash());
                for f in keywords {
                    let hash = TagHash::from_bytes(f.as_bytes());
                    if !tag_lck.contains_key(&hash) {
                        tag_lck.insert(hash, f.to_string());
                    }
                    env.tags_mut().push(hash);
                }
            }
        }
        if uid_store.keep_offline_cache && cache_handle.mailbox_state(mailbox_hash)?.is_some() {
            cache_handle
                .insert_envelopes(mailbox_hash, &v)
                .chain_err_summary(|| {
                    format!(
                        "Could not save envelopes in cache for mailbox {}",
                        mailbox.imap_path()
                    )
                })?;
        }

        for FetchResponse { uid, envelope, .. } in v {
            if uid.is_none() || envelope.is_none() {
                continue;
            }
            let uid = uid.unwrap();
            if uid_store
                .uid_index
                .lock()
                .unwrap()
                .contains_key(&(mailbox_hash, uid))
            {
                continue;
            }
            let env = envelope.unwrap();
            debug!(
                "Create event {} {} {}",
                env.hash(),
                env.subject(),
                mailbox.path(),
            );
            uid_store
                .msn_index
                .lock()
                .unwrap()
                .entry(mailbox_hash)
                .or_default()
                .push(uid);
            uid_store
                .hash_index
                .lock()
                .unwrap()
                .insert(env.hash(), (uid, mailbox_hash));
            uid_store
                .uid_index
                .lock()
                .unwrap()
                .insert((mailbox_hash, uid), env.hash());
            conn.add_refresh_event(RefreshEvent {
                account_hash: uid_store.account_hash,
                mailbox_hash,
                kind: Create(Box::new(env)),
            });
        }
    }
    Ok(())
}
