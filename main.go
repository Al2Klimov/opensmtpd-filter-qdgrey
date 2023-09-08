// SPDX-License-Identifier: GPL-3.0-or-later

package main

import (
	"bufio"
	"context"
	"crypto/sha256"
	_ "embed"
	"encoding/base64"
	"flag"
	"fmt"
	"github.com/Al2Klimov/go-gen-source-repos"
	"github.com/redis/go-redis/v9"
	log "github.com/sirupsen/logrus"
	"io"
	"os"
	"strings"
)

//go:embed redis.lua
var redisLua string

var redisScript = redis.NewScript(redisLua)

func main() {
	addr := flag.String("redis", "", "HOST:PORT|/SOCKET")
	flag.Parse()

	rds := redis.NewClient(&redis.Options{Addr: *addr})
	opts := rds.Options()
	tcp := false
	sock := ""

	switch opts.Network {
	case "tcp":
		tcp = true
	case "unix":
		sock = opts.Addr
	}

	hardening(tcp, sock)
	log.SetOutput(os.Stderr)
	log.SetLevel(log.TraceLevel)

	log.WithFields(log.Fields{"projects": source_repos.GetLinks()}).Info(
		"For the terms of use, the source code and the authors see the projects this program is assembled from",
	)

	ignoreLvl := log.DebugLevel
	sendersBySession := map[string]string{}

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
									sb := &strings.Builder{}
									b64 := base64.NewEncoder(base64.RawURLEncoding, sb)
									h := sha256.New()

									_, _ = io.WriteString(h, sender)
									_, _ = io.WriteString(h, "\n")
									_, _ = io.WriteString(h, tokens[7])

									sb.WriteString("opensmtpd-filter-qdgrey{")
									_, _ = b64.Write(h.Sum(nil))
									_ = b64.Close()
									sb.WriteByte('}')

									cmd := redisScript.Run(
										context.Background(),
										rds,
										[]string{sb.String() + "grey", sb.String() + "white"},
									)

									if i, err := cmd.Int(); err == nil {
										switch i {
										case 0:
											lf.Info("Greylisted")
										case 1:
											lf.Info("Still greylisted")
										default:
											allowLvl = log.InfoLevel
										}

										switch i {
										case 0, 1:
											fmt.Printf("filter-result|%s|%s|reject|450 Greylisted\n", tokens[5], tokens[6])
											continue
										}
									} else {
										lf = lf.WithError(err)
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
