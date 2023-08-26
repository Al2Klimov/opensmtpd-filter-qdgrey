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

type tuple struct {
	from, to string
}

func main() {
	hardening()
	log.SetOutput(os.Stderr)
	log.SetLevel(log.TraceLevel)

	log.WithFields(log.Fields{"projects": source_repos.GetLinks()}).Info(
		"For the terms of use, the source code and the authors see the projects this program is assembled from",
	)

	ignoreLvl := log.DebugLevel
	sendersBySession := map[string]string{}
	greylisted := map[tuple]struct{}{}

	for in := bufio.NewReader(os.Stdin); ; {
		switch line, err := in.ReadString('\n'); err {
		case nil:
			if line = strings.TrimRight(line, "\n"); line == "config|ready" {
				fmt.Println("register|report|smtp-in|tx-mail")
				fmt.Println("register|report|smtp-in|link-disconnect")
				fmt.Println("register|filter|smtp-in|rcpt-to")
				fmt.Println("register|ready")
				log.Info("Completed handshake")

				ignoreLvl = log.WarnLevel
			} else {
				switch tokens := strings.Split(line, "|"); tokens[0] {
				case "filter":
					if len(tokens) >= 7 {
						lf := log.WithFields(tokens2fields(tokens, 7))
						allowLvl := log.WarnLevel

						if tokens[3] == "smtp-in" && tokens[4] == "rcpt-to" {
							if sender, ok := sendersBySession[tokens[5]]; ok {
								delete(sendersBySession, tokens[5])
								lf.Trace("GC-ed mail sender")

								if len(tokens) < 8 {
									lf.Warn("Recipient missing")
								} else {
									tpl := tuple{sender, tokens[7]}
									if _, ok := greylisted[tpl]; ok {
										allowLvl = log.InfoLevel

										delete(greylisted, tpl)
										lf.Info("Grey-de-listed")
									} else {
										greylisted[tpl] = struct{}{}

										lf.Info("Greylisted")
										fmt.Printf("filter-result|%s|%s|reject|450 Greylisted\n", tokens[5], tokens[6])
										continue
									}
								}
							} else {
								lf.Warn("Sender missing")
							}
						}

						lf.Log(allowLvl, "Allowing filter input")
						fmt.Printf("filter-result|%s|%s|proceed\n", tokens[5], tokens[6])
						continue
					}
				case "report":
					if len(tokens) >= 6 && tokens[3] == "smtp-in" {
						switch tokens[4] {
						case "tx-mail":
							if len(tokens) >= 9 {
								if tokens[7] == "ok" {
									sendersBySession[tokens[5]] = tokens[8]
									log.WithFields(tokens2fields(tokens, 6)).Trace("Noted mail sender")
								} else {
									delete(sendersBySession, tokens[5])
									log.WithFields(tokens2fields(tokens, 6)).Trace("GC-ed mail sender")
								}

								continue
							}
						case "link-disconnect":
							delete(sendersBySession, tokens[5])
							log.WithFields(tokens2fields(tokens, 6)).Trace("GC-ed mail sender")
							continue
						}
					}
				}

				log.WithField("input", line).Log(ignoreLvl, "Ignoring input")
			}
		case io.EOF:
			log.Info("End of stdin, terminating")
			return
		default:
			log.WithError(err).Fatal("Couldn't read stdin")
		}
	}
}

func tokens2fields(tokens []string, paramsOffset int) log.Fields {
	return log.Fields{
		"protocol":  tokens[1],
		"timestamp": tokens[2],
		"subsystem": tokens[3],
		"phase":     tokens[4],
		"session":   tokens[5],
		"params":    tokens[paramsOffset:],
	}
}
