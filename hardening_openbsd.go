// SPDX-License-Identifier: GPL-3.0-or-later
//go:build openbsd

package main

import "golang.org/x/sys/unix"

func hardening(tcp bool, sock string) {
	promises := "stdio tty"
	if tcp {
		promises += " dns inet"
	}

	if sock != "" {
		if err := unix.Unveil(sock, "rw"); err != nil {
			panic(err)
		}

		if err := unix.UnveilBlock(); err != nil {
			panic(err)
		}

		promises += " unix"
	}

	if err := unix.PledgePromises(promises); err != nil {
		panic(err)
	}
}
