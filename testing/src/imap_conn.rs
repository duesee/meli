extern crate melib;
use melib::*;
use std::collections::HashMap;

use melib::backends::ImapType;
use melib::AccountSettings;
use melib::Result;

fn main() -> Result<()> {
    let mut args = std::env::args().skip(1).collect::<Vec<String>>();
    if args.len() != 4 {
        eprintln!("Usage: imap_conn server_hostname server_username server_password server_port");
        std::process::exit(1);
    }

    let (a, b, c, d) = (
        std::mem::replace(&mut args[0], String::new()),
        std::mem::replace(&mut args[1], String::new()),
        std::mem::replace(&mut args[2], String::new()),
        std::mem::replace(&mut args[3], String::new()),
    );
    let set = AccountSettings {
        extra: [
            ("server_hostname".to_string(), a),
            ("server_username".to_string(), b),
            ("server_password".to_string(), c),
            ("server_port".to_string(), d),
            (
                "danger_accept_invalid_certs".to_string(),
                "true".to_string(),
            ),
        ]
        .iter()
        .cloned()
        .collect(),
        ..Default::default()
    };
    let mut imap = ImapType::new(&set, Box::new(|_| true));
    imap.shell();
    Ok(())
}
