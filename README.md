<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# opensmtpd-filter-qdgrey

## Name

Meaning: **OpenSMTPd filter** for **q**uick and **d**irty **grey**listing

The naming pattern was inspired by **QDOS**,
the **q**uick and **d**irty **o**perating **s**ystem.

Pronounce "qd" as _cute_ (inspired by Qt), but with a D - _cude_.

## Rationale

Traditional greylisting temporarily blocks messages by these attributes:

* MTA IP
* sender
* recipient

Unfortunately the more MTAs of a large network fall back to each other because
of greylisting, the more a single message is delayed - if it arrives at all.
(Due to the fact that every new MTA gets greylisted for the same message.)

This could be fixed by taking into account the sender's SPF records.
Theoretically. But no one has to publish such.

The quick and dirty workaround of opensmtpd-filter-qdgrey is simple:
Ignore the MTA IP, just greylist a message by sender and recipient.

## Build

Compile like any other Rust program: `cargo build -r`

Find the resulting binary directly under `target/release/`.

## Usage

Integrate this filter into smtpd.conf(5). Search in smtpd.conf(5) for "proc-exec" on how to do so.

### Command-line interface

```
opensmtpd-filter-qdgrey -redis HOST:PORT|/SOCKET
```

Connect to your Redis instance by providing its address via `-redis HOST:PORT` for TCP
or `-redis /SOCKET` for a Unix socket. Redis is required for greylisting state storage.
