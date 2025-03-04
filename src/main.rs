/*
 * meli - bin.rs
 *
 * Copyright 2017-2018 Manos Pitsidianakis
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

//! Command line client binary.
//!
//! This crate contains the frontend stuff of the application. The application
//! entry way on `src/bin.rs` creates an event loop and passes input to a
//! thread.
//!
//! The mail handling stuff is done in the `melib` crate which includes all
//! backend needs. The split is done to theoretically be able to create
//! different frontends with the same innards.

use meli::*;
mod args;
use std::os::raw::c_int;

use args::*;

fn notify(
    signals: &[c_int],
    sender: crossbeam::channel::Sender<ThreadEvent>,
) -> std::result::Result<crossbeam::channel::Receiver<c_int>, std::io::Error> {
    use std::time::Duration;
    let (alarm_pipe_r, alarm_pipe_w) =
        nix::unistd::pipe().map_err(|err| std::io::Error::from_raw_os_error(err as i32))?;
    let alarm_handler = move |info: &nix::libc::siginfo_t| {
        let value = unsafe { info.si_value().sival_ptr as u8 };
        let _ = nix::unistd::write(alarm_pipe_w, &[value]);
    };
    unsafe {
        signal_hook_registry::register_sigaction(signal_hook::consts::SIGALRM, alarm_handler)?;
    }
    let (s, r) = crossbeam::channel::bounded(100);
    let mut signals = signal_hook::iterator::Signals::new(signals)?;
    let _ = nix::fcntl::fcntl(
        alarm_pipe_r,
        nix::fcntl::FcntlArg::F_SETFL(nix::fcntl::OFlag::O_NONBLOCK),
    );
    std::thread::spawn(move || {
        let mut ctr = 0;
        loop {
            ctr %= 3;
            if ctr == 0 {
                let _ = sender
                    .send_timeout(ThreadEvent::Pulse, Duration::from_millis(500))
                    .ok();
            }

            for signal in signals.pending() {
                let _ = s.send_timeout(signal, Duration::from_millis(500)).ok();
            }

            std::thread::sleep(std::time::Duration::from_millis(100));
            ctr += 1;
        }
    });
    Ok(r)
}

fn main() {
    let opt = Opt::from_args();
    ::std::process::exit(match run_app(opt) {
        Ok(()) => 0,
        Err(err) => {
            eprintln!("{}", err);
            1
        }
    });
}

fn run_app(opt: Opt) -> Result<()> {
    if let Some(config_location) = opt.config.as_ref() {
        std::env::set_var("MELI_CONFIG", config_location);
    }

    match opt.subcommand {
        Some(SubCommand::TestConfig { path }) => {
            let config_path = if let Some(path) = path {
                path
            } else {
                crate::conf::get_config_file()?
            };
            conf::FileSettings::validate(config_path, true, false)?; // TODO: test for tty/interaction
            return Ok(());
        }
        Some(SubCommand::CreateConfig { path }) => {
            let config_path = if let Some(path) = path {
                path
            } else {
                crate::conf::get_config_file()?
            };
            if config_path.exists() {
                return Err(Error::new(format!(
                    "File `{}` already exists.\nMaybe you meant to specify another path?",
                    config_path.display()
                )));
            }
            conf::create_config_file(&config_path)?;
            return Ok(());
        }
        Some(SubCommand::EditConfig) => {
            use std::process::{Command, Stdio};
            let editor = std::env::var("EDITOR")
                .or_else(|_| std::env::var("VISUAL"))
                .map_err(|err| {
                    format!(
                        "Could not find any value in environment variables EDITOR and VISUAL. \
                         {err}"
                    )
                })?;
            let config_path = crate::conf::get_config_file()?;

            let mut cmd = Command::new(&editor);

            let mut handle = &mut cmd;
            for c in crate::conf::get_included_configs(config_path)? {
                handle = handle.arg(&c);
            }
            let mut handle = handle
                .stdin(Stdio::inherit())
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit())
                .spawn()?;
            handle.wait()?;
            return Ok(());
        }
        #[cfg(feature = "cli-docs")]
        Some(SubCommand::Man(manopt)) => {
            let ManOpt { page, no_raw } = manopt;
            const MANPAGES: [&[u8]; 4] = [
                include_bytes!(concat!(env!("OUT_DIR"), "/meli.txt.gz")),
                include_bytes!(concat!(env!("OUT_DIR"), "/meli.conf.txt.gz")),
                include_bytes!(concat!(env!("OUT_DIR"), "/meli-themes.txt.gz")),
                include_bytes!(concat!(env!("OUT_DIR"), "/meli.7.txt.gz")),
            ];
            use std::io::prelude::*;

            use flate2::bufread::GzDecoder;
            let mut gz = GzDecoder::new(MANPAGES[page as usize]);
            let mut v = String::with_capacity(
                str::parse::<usize>(unsafe {
                    std::str::from_utf8_unchecked(gz.header().unwrap().comment().unwrap())
                })
                .unwrap_or_else(|_| {
                    panic!("{:?} was not compressed with size comment header", page)
                }),
            );
            gz.read_to_string(&mut v)?;

            if let Some(no_raw) = no_raw {
                match no_raw {
                    Some(true) => {}
                    None if (unsafe { libc::isatty(libc::STDOUT_FILENO) == 1 }) => {}
                    Some(false) | None => {
                        println!("{}", &v);
                        return Ok(());
                    }
                }
            } else if unsafe { libc::isatty(libc::STDOUT_FILENO) != 1 } {
                println!("{}", &v);
                return Ok(());
            }

            use std::process::{Command, Stdio};
            let mut handle = Command::new("sh")
                .arg("-c")
                .arg(std::env::var("PAGER").unwrap_or_else(|_| "more".to_string()))
                .stdin(Stdio::piped())
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit())
                .spawn()?;
            handle.stdin.take().unwrap().write_all(v.as_bytes())?;
            handle.wait()?;

            return Ok(());
        }
        #[cfg(not(feature = "cli-docs"))]
        Some(SubCommand::Man(_manopt)) => {
            return Err(Error::new("error: this version of meli was not build with embedded documentation (cargo feature `cli-docs`). You might have it installed as manpages (eg `man meli`), otherwise check https://meli.delivery"));
        }
        Some(SubCommand::CompiledWith) => {
            #[cfg(feature = "notmuch")]
            println!("notmuch");
            #[cfg(feature = "jmap")]
            println!("jmap");
            #[cfg(feature = "sqlite3")]
            println!("sqlite3");
            #[cfg(feature = "smtp")]
            println!("smtp");
            #[cfg(feature = "regexp")]
            println!("regexp");
            #[cfg(feature = "dbus-notifications")]
            println!("dbus-notifications");
            #[cfg(feature = "cli-docs")]
            println!("cli-docs");
            #[cfg(feature = "gpgme")]
            println!("gpgme");
            return Ok(());
        }
        Some(SubCommand::PrintLoadedThemes) => {
            let s = conf::FileSettings::new()?;
            print!("{}", s.terminal.themes);
            return Ok(());
        }
        Some(SubCommand::PrintDefaultTheme) => {
            print!("{}", conf::Themes::default().key_to_string("dark", false));
            return Ok(());
        }
        Some(SubCommand::View { ref path }) => {
            if !path.exists() {
                return Err(Error::new(format!(
                    "`{}` is not a valid path",
                    path.display()
                )));
            } else if !path.is_file() {
                return Err(Error::new(format!("`{}` is a directory", path.display())));
            }
        }
        None => {}
    }

    /* Create a channel to communicate with other threads. The main process is
     * the sole receiver.
     */
    let (sender, receiver) = crossbeam::channel::bounded(32 * ::std::mem::size_of::<ThreadEvent>());
    /* Catch SIGWINCH to handle terminal resizing */
    let signals = &[
        /* Catch SIGWINCH to handle terminal resizing */
        signal_hook::consts::SIGWINCH,
        /* Catch SIGCHLD to handle embed applications status change */
        signal_hook::consts::SIGCHLD,
    ];

    let signal_recvr = notify(signals, sender.clone())?;

    /* Create the application State. */
    let mut state;

    if let Some(SubCommand::View { path }) = opt.subcommand {
        let bytes = std::fs::read(&path)
            .chain_err_summary(|| format!("Could not read from `{}`", path.display()))?;
        let wrapper = Mail::new(bytes, Some(Flag::SEEN))
            .chain_err_summary(|| format!("Could not parse `{}`", path.display()))?;
        state = State::new(
            Some(Settings::without_accounts().unwrap_or_default()),
            sender,
            receiver.clone(),
        )?;
        state.register_component(Box::new(EnvelopeView::new(
            wrapper,
            None,
            None,
            AccountHash::default(),
        )));
    } else {
        state = State::new(None, sender, receiver.clone())?;
        #[cfg(feature = "svgscreenshot")]
        state.register_component(Box::new(components::svg::SVGScreenshotFilter::new()));
        let window = Box::new(Tabbed::new(
            vec![
                Box::new(listing::Listing::new(&mut state.context)),
                Box::new(ContactList::new(&state.context)),
            ],
            &state.context,
        ));

        let status_bar = Box::new(StatusBar::new(&state.context, window));
        state.register_component(status_bar);

        #[cfg(all(target_os = "linux", feature = "dbus-notifications"))]
        {
            let dbus_notifications = Box::new(components::notifications::DbusNotifications::new(
                &state.context,
            ));
            state.register_component(dbus_notifications);
        }
        state.register_component(Box::new(
            components::notifications::NotificationCommand::new(),
        ));
    }
    let enter_command_mode: Key = state
        .context
        .settings
        .shortcuts
        .general
        .enter_command_mode
        .clone();
    let quit_key: Key = state.context.settings.shortcuts.general.quit.clone();

    /* Keep track of the input mode. See UIMode for details */
    'main: loop {
        state.render();

        'inner: loop {
            /* Check if any components have sent reply events to State. */
            let events: smallvec::SmallVec<[UIEvent; 8]> = state.context.replies();
            for e in events {
                state.rcv_event(e);
            }
            state.redraw();

            /* Poll on all channels. Currently we have the input channel for stdin,
             * watching events and the signal watcher. */
            crossbeam::select! {
                recv(receiver) -> r => {
                    match r {
                         Ok(ThreadEvent::Pulse) | Ok(ThreadEvent::UIEvent(UIEvent::Timer(_))) => {},
                        _ => {debug!(&r);}
                    }
                    match r.unwrap() {
                        ThreadEvent::Input((Key::Ctrl('z'), _)) if state.mode != UIMode::Embed => {
                            state.switch_to_main_screen();
                            //_thread_handler.join().expect("Couldn't join on the associated thread");
                            let self_pid = nix::unistd::Pid::this();
                            nix::sys::signal::kill(self_pid, nix::sys::signal::Signal::SIGSTOP).unwrap();
                            state.switch_to_alternate_screen();
                            // BUG: thread sends input event after one received key
                            state.update_size();
                            state.render();
                            state.redraw();
                        },
                        ThreadEvent::Input(raw_input @ (Key::Ctrl('l'), _)) => {
                            /* Manual screen redraw */
                            state.update_size();
                            state.render();
                            state.redraw();
                            if state.mode == UIMode::Embed {
                                state.rcv_event(UIEvent::EmbedInput(raw_input));
                                state.redraw();
                            }
                        },
                        ThreadEvent::Input((k, r)) => {
                            match state.mode {
                                UIMode::Normal => {
                                    match k {
                                        _ if k == quit_key => {
                                            if state.can_quit_cleanly() {
                                                drop(state);
                                                break 'main;
                                            } else {
                                                state.redraw();
                                            }
                                        },
                                        _ if k == enter_command_mode => {
                                            state.mode = UIMode::Command;
                                            state.rcv_event(UIEvent::ChangeMode(UIMode::Command));
                                            state.redraw();
                                        }
                                        key  => {
                                            state.rcv_event(UIEvent::Input(key));
                                            state.redraw();
                                        },
                                    }
                                },
                                UIMode::Insert => {
                                    match k {
                                        Key::Esc => {
                                            state.rcv_event(UIEvent::ChangeMode(UIMode::Normal));
                                            state.redraw();
                                        },
                                        k => {
                                            state.rcv_event(UIEvent::InsertInput(k));
                                            state.redraw();
                                        },
                                    }
                                }
                                UIMode::Command => {
                                    match k {
                                        Key::Char('\n') => {
                                            state.mode = UIMode::Normal;
                                            state.rcv_event(UIEvent::ChangeMode(UIMode::Normal));
                                            state.redraw();
                                        },
                                        k => {
                                            state.rcv_event(UIEvent::CmdInput(k));
                                            state.redraw();
                                        },
                                    }
                                },
                                UIMode::Embed => {
                                    state.rcv_event(UIEvent::EmbedInput((k,r)));
                                    state.redraw();
                                },
                                UIMode::Fork => {
                                    break 'inner; // `goto` 'reap loop, and wait on child.
                                },
                            }
                        },
                        ThreadEvent::RefreshMailbox(event) => {
                            state.refresh_event(*event);
                            state.redraw();
                        },
                        ThreadEvent::UIEvent(UIEvent::ChangeMode(f)) => {
                            state.mode = f;
                            if f == UIMode::Fork {
                                break 'inner; // `goto` 'reap loop, and wait on child.
                            }
                        }
                        ThreadEvent::UIEvent(e) => {
                            state.rcv_event(e);
                            state.redraw();
                        },
                        ThreadEvent::Pulse => {
                            state.check_accounts();
                            state.redraw();
                        },
                        ThreadEvent::JobFinished(id) => {
                            debug!("Job finished {}", id);
                            for account in state.context.accounts.values_mut() {
                                if account.process_event(&id) {
                                    break;
                                }
                            }
                            //state.new_thread(id, name);
                        },
                    }
                },
                recv(signal_recvr) -> sig => {
                    match sig.unwrap() {
                        signal_hook::consts::SIGWINCH => {
                            if state.mode != UIMode::Fork  {
                                state.update_size();
                                state.render();
                                state.redraw();
                            }
                        },
                        signal_hook::consts::SIGCHLD => {
                            state.rcv_event(UIEvent::EmbedInput((Key::Null, vec![0])));
                            state.redraw();

                        }
                        other => {
                            debug!("got other signal: {:?}", other);
                        }
                    }
                },
            }
        } // end of 'inner

        'reap: loop {
            match state.try_wait_on_child() {
                Some(true) => {
                    state.restore_input();
                    state.switch_to_alternate_screen();
                }
                Some(false) => {
                    use std::{thread, time};
                    let ten_millis = time::Duration::from_millis(1500);
                    thread::sleep(ten_millis);

                    continue 'reap;
                }
                None => {
                    state.mode = UIMode::Normal;
                    state.render();
                    break 'reap;
                }
            }
        }
    }
    Ok(())
}
