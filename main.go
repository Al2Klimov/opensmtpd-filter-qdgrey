// SPDX-License-Identifier: GPL-3.0-or-later

package main

import (
	"github.com/Al2Klimov/go-gen-source-repos"
	log "github.com/sirupsen/logrus"
	"os"
)

func main() {
	hardening()
	log.SetOutput(os.Stderr)
	log.SetLevel(log.TraceLevel)

	log.WithFields(log.Fields{"projects": source_repos.GetLinks()}).Info(
		"For the terms of use, the source code and the authors see the projects this program is assembled from",
	)
}
