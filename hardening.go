// SPDX-License-Identifier: GPL-3.0-or-later
//go:build !js && !wasip1 && !openbsd

package main

import _ "golang.org/x/sys/unix"

func hardening(bool, string) {
}
