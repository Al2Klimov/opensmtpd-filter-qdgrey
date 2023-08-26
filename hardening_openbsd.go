// SPDX-License-Identifier: GPL-3.0-or-later
//go:build openbsd

package main

import "golang.org/x/sys/unix"

func hardening() {
	if err := unix.PledgePromises("stdio tty"); err != nil {
		panic(err)
	}
}
