// SPDX-License-Identifier: GPL-3.0-or-later

use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::Duration;

const REDIS_ADDR: &str = "localhost:6379";
const TIMEOUT: Duration = Duration::from_secs(5);
// 7 days in seconds (604800) minus a small buffer for test timing
const WHITE_TTL_MIN: i64 = 604000;

fn filter_cmd() -> Command {
    Command::new(env!("CARGO_BIN_EXE_opensmtpd-filter-qdgrey"))
}

struct FilterProcess {
    child: Child,
    stdin: ChildStdin,
    lines_rx: Receiver<String>,
}

impl FilterProcess {
    fn start() -> Self {
        let mut child = filter_cmd()
            .args(["-redis", REDIS_ADDR])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("Failed to start filter binary");

        let stdin = child.stdin.take().unwrap();
        let stdout = child.stdout.take().unwrap();

        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            let reader = BufReader::new(stdout);
            for line in reader.lines() {
                match line {
                    Ok(l) => {
                        if tx.send(l).is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        FilterProcess {
            child,
            stdin,
            lines_rx: rx,
        }
    }

    fn send(&mut self, line: &str) {
        writeln!(self.stdin, "{}", line).unwrap();
    }

    fn read_line(&self) -> String {
        self.lines_rx
            .recv_timeout(TIMEOUT)
            .expect("Timeout waiting for filter output")
    }

    fn handshake(&mut self) {
        self.send("config|ready");
        for _ in 0..4 {
            self.read_line();
        }
    }
}

impl Drop for FilterProcess {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn get_redis_con() -> redis::Connection {
    let client = redis::Client::open(format!("redis://{}/", REDIS_ADDR)).unwrap();
    client.get_connection().unwrap()
}

fn flush_redis() {
    let mut con = get_redis_con();
    let _: () = redis::cmd("FLUSHDB").query(&mut con).unwrap();
}

fn report_tx_mail(filter: &mut FilterProcess, session: &str, sender: &str) {
    filter.send(&format!(
        "report|0.7|1234567890.000000|smtp-in|tx-mail|{}|msgid1|ok|{}",
        session, sender
    ));
}

fn filter_rcpt_to(filter: &mut FilterProcess, session: &str, reqid: &str, recipient: &str) {
    filter.send(&format!(
        "filter|0.7|1234567890.000000|smtp-in|rcpt-to|{}|{}|{}",
        session, reqid, recipient
    ));
}

fn report_link_disconnect(filter: &mut FilterProcess, session: &str) {
    filter.send(&format!(
        "report|0.7|1234567890.000000|smtp-in|link-disconnect|{}",
        session
    ));
}

#[test]
fn test_handshake() {
    flush_redis();
    let mut filter = FilterProcess::start();
    filter.send("config|ready");

    assert_eq!(filter.read_line(), "register|report|smtp-in|tx-mail");
    assert_eq!(
        filter.read_line(),
        "register|report|smtp-in|link-disconnect"
    );
    assert_eq!(filter.read_line(), "register|filter|smtp-in|rcpt-to");
    assert_eq!(filter.read_line(), "register|ready");
}

#[test]
fn test_first_contact_greylisted() {
    flush_redis();
    let mut filter = FilterProcess::start();
    filter.handshake();

    report_tx_mail(&mut filter, "sess1", "sender@example.com");
    filter_rcpt_to(&mut filter, "sess1", "req1", "recipient@example.com");

    let response = filter.read_line();
    assert!(
        response.contains("reject|450 Greylisted"),
        "Expected reject|450 Greylisted, got: {}",
        response
    );
}

#[test]
fn test_still_greylisted_retry_within_window() {
    flush_redis();
    let mut filter = FilterProcess::start();
    filter.handshake();

    report_tx_mail(&mut filter, "sess1", "sender@example.com");
    filter_rcpt_to(&mut filter, "sess1", "req1", "recipient@example.com");
    let r1 = filter.read_line();
    assert!(
        r1.contains("reject|450 Greylisted"),
        "Expected greylisted on first attempt, got: {}",
        r1
    );

    report_tx_mail(&mut filter, "sess2", "sender@example.com");
    filter_rcpt_to(&mut filter, "sess2", "req2", "recipient@example.com");
    let r2 = filter.read_line();
    assert!(
        r2.contains("reject|450 Greylisted"),
        "Expected still greylisted on retry, got: {}",
        r2
    );
}

#[test]
fn test_whitelisted_after_grey_expiry() {
    flush_redis();
    let mut filter = FilterProcess::start();
    filter.handshake();

    report_tx_mail(&mut filter, "sess1", "sender@example.com");
    filter_rcpt_to(&mut filter, "sess1", "req1", "recipient@example.com");
    let r1 = filter.read_line();
    assert!(
        r1.contains("reject|450 Greylisted"),
        "Expected greylisted on first attempt, got: {}",
        r1
    );

    let mut con = get_redis_con();
    let grey_keys: Vec<String> = redis::cmd("KEYS")
        .arg("opensmtpd-filter-qdgrey{*}grey")
        .query(&mut con)
        .unwrap();
    assert!(!grey_keys.is_empty(), "Expected grey key to exist");
    for key in &grey_keys {
        let _: () = redis::cmd("DEL").arg(key).query(&mut con).unwrap();
    }

    report_tx_mail(&mut filter, "sess2", "sender@example.com");
    filter_rcpt_to(&mut filter, "sess2", "req2", "recipient@example.com");
    let r2 = filter.read_line();
    assert!(
        r2.contains("proceed"),
        "Expected proceed after grey expiry, got: {}",
        r2
    );
}

#[test]
fn test_re_greylisted_after_full_expiry() {
    flush_redis();
    let mut filter = FilterProcess::start();
    filter.handshake();

    report_tx_mail(&mut filter, "sess1", "sender@example.com");
    filter_rcpt_to(&mut filter, "sess1", "req1", "recipient@example.com");
    let r1 = filter.read_line();
    assert!(
        r1.contains("reject|450 Greylisted"),
        "Expected greylisted on first attempt, got: {}",
        r1
    );

    flush_redis();

    report_tx_mail(&mut filter, "sess2", "sender@example.com");
    filter_rcpt_to(&mut filter, "sess2", "req2", "recipient@example.com");
    let r2 = filter.read_line();
    assert!(
        r2.contains("reject|450 Greylisted"),
        "Expected re-greylisted after full expiry, got: {}",
        r2
    );
}

#[test]
fn test_link_disconnect_cleans_sender_state() {
    flush_redis();
    let mut filter = FilterProcess::start();
    filter.handshake();

    report_tx_mail(&mut filter, "sess1", "sender@example.com");
    report_link_disconnect(&mut filter, "sess1");

    filter_rcpt_to(&mut filter, "sess1", "req1", "recipient@example.com");
    let response = filter.read_line();
    assert!(
        response.contains("proceed"),
        "Expected proceed after link-disconnect cleaned sender state, got: {}",
        response
    );
}

#[test]
fn test_whitelist_ttl_extended() {
    flush_redis();
    let mut filter = FilterProcess::start();
    filter.handshake();

    report_tx_mail(&mut filter, "sess1", "sender@example.com");
    filter_rcpt_to(&mut filter, "sess1", "req1", "recipient@example.com");
    let r1 = filter.read_line();
    assert!(
        r1.contains("reject|450 Greylisted"),
        "Expected greylisted on first attempt, got: {}",
        r1
    );

    let mut con = get_redis_con();
    let grey_keys: Vec<String> = redis::cmd("KEYS")
        .arg("opensmtpd-filter-qdgrey{*}grey")
        .query(&mut con)
        .unwrap();
    for key in &grey_keys {
        let _: () = redis::cmd("DEL").arg(key).query(&mut con).unwrap();
    }

    report_tx_mail(&mut filter, "sess2", "sender@example.com");
    filter_rcpt_to(&mut filter, "sess2", "req2", "recipient@example.com");
    let r2 = filter.read_line();
    assert!(
        r2.contains("proceed"),
        "Expected proceed after grey expiry, got: {}",
        r2
    );

    let white_keys: Vec<String> = redis::cmd("KEYS")
        .arg("opensmtpd-filter-qdgrey{*}white")
        .query(&mut con)
        .unwrap();
    assert!(!white_keys.is_empty(), "Expected white key to exist");
    for key in &white_keys {
        let ttl: i64 = redis::cmd("TTL").arg(key).query(&mut con).unwrap();
        assert!(
            ttl > WHITE_TTL_MIN,
            "Expected white key TTL > {} seconds, got: {}",
            WHITE_TTL_MIN,
            ttl
        );
    }
}
