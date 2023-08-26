// SPDX-License-Identifier: GPL-3.0-or-later

package main

import (
	"bufio"
	"fmt"
	"github.com/Al2Klimov/go-gen-source-repos"
	log "github.com/sirupsen/logrus"
	"io"
	"os"
	"strings"
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
			if line = strings.TrimRight(line, "\n"); line == "config|ready" {
				fmt.Println("register|filter|smtp-in|rcpt-to")
				fmt.Println("register|ready")
				log.Info("Completed handshake")
			} else {
				switch tokens := strings.Split(line, "|"); tokens[0] {
				case "filter":
					if len(tokens) >= 7 {
						log.WithFields(log.Fields{
							"protocol":  tokens[1],
							"timestamp": tokens[2],
							"subsystem": tokens[3],
							"phase":     tokens[4],
							"session":   tokens[5],
							"params":    tokens[7:],
						}).Trace("Allowing filter input")

						fmt.Printf("filter-result|%s|%s|proceed\n", tokens[5], tokens[6])
						continue
					}
				}

				log.WithField("input", line).Debug("Ignoring input")
			}
		case io.EOF:
			log.Info("End of stdin, terminating")
			return
		default:
			log.WithError(err).Fatal("Couldn't read stdin")
		}
	}
}
