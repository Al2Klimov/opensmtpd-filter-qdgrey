// SPDX-License-Identifier: GPL-3.0-or-later

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::io::{self, BufRead, Write, stdin, stdout};
use std::process::exit;

const REDIS_LUA: &str = include_str!("../redis.lua");

fn main() -> io::Result<()> {
    let mut args = std::env::args();
    args.next();

    let mut redis_addr: Option<String> = None;

    loop {
        match args.next().as_deref() {
            None => break,
            Some("-h") | Some("--help") => {
                writeln!(
                    stdout().lock(),
                    "Usage: opensmtpd-filter-qdgrey -redis HOST:PORT|/SOCKET"
                )?;
                return Ok(());
            }
            Some("-redis") => match args.next() {
                None => {
                    eprintln!("Flag needs an argument: -redis");
                    exit(1);
                }
                Some(addr) => {
                    redis_addr = Some(addr);
                }
            },
            Some(arg) => {
                eprintln!("Unknown argument: {}", arg);
                exit(1);
            }
        }
    }

    let redis_addr = match redis_addr {
        Some(addr) => addr,
        None => {
            eprintln!("Missing required flag: -redis");
            exit(1);
        }
    };

    let redis_url = if redis_addr.starts_with('/') {
        format!("redis+unix://{}", redis_addr)
    } else {
        format!("redis://{}", redis_addr)
    };

    let client = match redis::Client::open(redis_url) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to create Redis client: {}", e);
            exit(1);
        }
    };

    let mut con = match client.get_connection() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to connect to Redis: {}", e);
            exit(1);
        }
    };

    let script = redis::Script::new(REDIS_LUA);

    let mut std_in = stdin().lock();
    let mut std_out = stdout().lock();
    let mut line = Vec::<u8>::new();
    let mut senders: HashMap<Vec<u8>, Vec<u8>> = HashMap::new();

    loop {
        line.clear();
        std_in.read_until(b'\n', &mut line)?;

        if line.is_empty() {
            return Ok(());
        }

        while line
            .pop_if(|last| match last {
                b'\r' => true,
                b'\n' => true,
                _ => false,
            })
            .is_some()
        {}

        let mut fields = line.split(|&sep| sep == b'|');

        match fields.next() {
            Some(b"config") => match fields.next() {
                Some(b"ready") => {
                    writeln!(std_out, "register|report|smtp-in|tx-mail")?;
                    writeln!(std_out, "register|report|smtp-in|link-disconnect")?;
                    writeln!(std_out, "register|filter|smtp-in|rcpt-to")?;
                    writeln!(std_out, "register|ready")?;
                }
                _ => {}
            },
            Some(b"report") => {
                fields.next(); // protocol version
                fields.next(); // timestamp
                fields.next(); // subsystem

                match (fields.next(), fields.next()) {
                    (Some(b"tx-mail"), Some(session)) => {
                        fields.next(); // message id
                        match fields.next() {
                            Some(b"ok") => match fields.next() {
                                Some(sender) => {
                                    senders.insert(session.to_owned(), sender.to_owned());
                                }
                                _ => {}
                            },
                            _ => {
                                senders.remove(session);
                            }
                        }
                    }
                    (Some(b"link-disconnect"), Some(session)) => {
                        senders.remove(session);
                    }
                    _ => {}
                }
            }
            Some(b"filter") => {
                fields.next(); // protocol version
                fields.next(); // timestamp
                fields.next(); // subsystem

                match (fields.next(), fields.next(), fields.next()) {
                    (Some(b"rcpt-to"), Some(session), Some(token)) => {
                        let reject = match senders.remove(session) {
                            Some(sender) => match fields.next() {
                                Some(recipient) => {
                                    let mut hasher = Sha256::new();
                                    hasher.update(&sender);
                                    hasher.update(b"\n");
                                    hasher.update(recipient);
                                    let hash = hasher.finalize();
                                    let encoded = URL_SAFE_NO_PAD.encode(hash);
                                    let key_prefix =
                                        format!("opensmtpd-filter-qdgrey{{{}}}", encoded);
                                    let grey_key = format!("{}grey", key_prefix);
                                    let white_key = format!("{}white", key_prefix);

                                    match script
                                        .key(&grey_key)
                                        .key(&white_key)
                                        .invoke::<i64>(&mut con)
                                    {
                                        Ok(result) => result <= 1,
                                        Err(e) => {
                                            eprintln!("Redis error: {}", e);
                                            false
                                        }
                                    }
                                }
                                None => false,
                            },
                            None => false,
                        };

                        std_out.write_all(b"filter-result|")?;
                        std_out.write_all(session)?;
                        std_out.write_all(b"|")?;
                        std_out.write_all(token)?;
                        if reject {
                            writeln!(std_out, "|reject|450 Greylisted")?;
                        } else {
                            writeln!(std_out, "|proceed")?;
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }
}
