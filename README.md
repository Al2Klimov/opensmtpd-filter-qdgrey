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

## Usage

Build the source code like any other program written in Golang.

The resulting executable communicates with smtpd(8) as per smtpd-filters(7).
Search in smtpd.conf(5) for "proc-exec" on how to integrate it.

Also run a Redis server, either on its default port and on the same machine
as OpenSMTPd or tell opensmtpd-filter-qdgrey how to connect
to your Redis instance (see `go run . -h`).
