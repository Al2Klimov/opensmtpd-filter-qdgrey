// SPDX-License-Identifier: GPL-3.0-or-later

package main

import (
	"bufio"
	"github.com/Al2Klimov/go-gen-source-repos"
	log "github.com/sirupsen/logrus"
	"io"
	"os"
)

func main() {
	hardening()
	log.SetOutput(os.Stderr)
	log.SetLevel(log.TraceLevel)

	log.WithFields(log.Fields{"projects": source_repos.GetLinks()}).Info(
		"For the terms of use, the source code and the authors see the projects this program is assembled from",
	)

	for in := bufio.NewReader(os.Stdin); ; {
		switch line, err := in.ReadString('\n'); err {
		case nil:
			// TODO
			_ = line
		case io.EOF:
			log.Info("End of stdin, terminating")
			return
		default:
			log.WithError(err).Fatal("Couldn't read stdin")
		}
	}
}
