// SPDX-License-Identifier: GPL-3.0-or-later

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::io::{BufRead, stdin};

const REDIS_LUA: &str = include_str!("../redis.lua");

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut redis_addr = String::new();
    let mut i = 1;
    while i < args.len() {
        if args[i] == "-redis" && i + 1 < args.len() {
            redis_addr = args[i + 1].clone();
            i += 2;
        } else {
            i += 1;
        }
    }

    let redis_url = if redis_addr.starts_with('/') {
        format!("redis+unix://{}", redis_addr)
    } else {
        format!("redis://{}", redis_addr)
    };

    let client = redis::Client::open(redis_url).expect("Failed to create Redis client");
    let mut con = client.get_connection().expect("Failed to connect to Redis");
    let script = redis::Script::new(REDIS_LUA);

    let stdin = stdin();
    let mut senders: HashMap<String, String> = HashMap::new();

    for line_result in stdin.lock().lines() {
        let line = match line_result {
            Ok(l) => l,
            Err(e) => {
                eprintln!("Error reading stdin: {}", e);
                return;
            }
        };

        if line == "config|ready" {
            println!("register|report|smtp-in|tx-mail");
            println!("register|report|smtp-in|link-disconnect");
            println!("register|filter|smtp-in|rcpt-to");
            println!("register|ready");
        } else {
            let tokens: Vec<&str> = line.split('|').collect();
            match tokens.first().copied() {
                Some("filter") if tokens.len() >= 7 => {
                    let session = tokens[5];
                    let token = tokens[6];
                    let mut reject = false;

                    if tokens[3] == "smtp-in" && tokens[4] == "rcpt-to" {
                        if let Some(sender) = senders.remove(session) {
                            if tokens.len() >= 8 {
                                let recipient = tokens[7];
                                let mut hasher = Sha256::new();
                                hasher.update(sender.as_bytes());
                                hasher.update(b"\n");
                                hasher.update(recipient.as_bytes());
                                let hash = hasher.finalize();
                                let encoded = URL_SAFE_NO_PAD.encode(hash);
                                let key_prefix = format!("opensmtpd-filter-qdgrey{{{}}}", encoded);
                                let grey_key = format!("{}grey", key_prefix);
                                let white_key = format!("{}white", key_prefix);

                                match script
                                    .key(&grey_key)
                                    .key(&white_key)
                                    .invoke::<i64>(&mut con)
                                {
                                    Ok(result) if result <= 1 => {
                                        reject = true;
                                    }
                                    Ok(_) => {}
                                    Err(e) => {
                                        eprintln!("Redis error: {}", e);
                                    }
                                }
                            }
                        }
                    }

                    if reject {
                        println!("filter-result|{}|{}|reject|450 Greylisted", session, token);
                    } else {
                        println!("filter-result|{}|{}|proceed", session, token);
                    }
                }
                Some("report") if tokens.len() >= 6 && tokens[3] == "smtp-in" => match tokens[4] {
                    "tx-mail" if tokens.len() >= 9 => {
                        let session = tokens[5];
                        if tokens[7] == "ok" {
                            senders.insert(session.to_owned(), tokens[8].to_owned());
                        } else {
                            senders.remove(session);
                        }
                    }
                    "link-disconnect" => {
                        senders.remove(tokens[5]);
                    }
                    _ => {}
                },
                _ => {}
            }
        }
    }
}
